// Copyright © 2024 Atamert Ölçgen.
// This file is part of Yer.
//
// Yer is free software: you can redistribute it and/or modify it under the
// terms of the GNU General Public License as published by the Free Software
// Foundation, either version 3 of the License, or (at your option) any later
// version.
//
// Yer is distributed in the hope that it will be useful, but WITHOUT ANY
// WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE.  See the GNU General Public License for more
// details.
//
// You should have received a copy of the GNU General Public License along
// with Yer.  If not, see <https://www.gnu.org/licenses/>.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};

#[cfg(feature = "inspector")]
use bevy::window::PrimaryWindow;
#[cfg(feature = "inspector")]
use bevy_egui::EguiContext;
#[cfg(feature = "inspector")]
use bevy_inspector_egui::{
    bevy_inspector::{ui_for_state, ui_for_world},
    DefaultInspectorConfigPlugin,
};

use crate::layer;
use crate::session;
use crate::viewport;

mod file_dialog;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<UiState>()
            .add_plugins((EguiPlugin, file_dialog::UiFileDialogPlugin))
            .init_state::<UiState>()
            .add_systems(Update, draw_ui_system)
            .add_systems(
                OnEnter(UiState::ShowingSaveFileDialog),
                show_save_file_dialog_system,
            );

        #[cfg(feature = "inspector")]
        app.add_plugins(DefaultInspectorConfigPlugin)
            .add_systems(Update, inspector_ui);
    }
}

// RESOURCES

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq, Reflect, States)]
enum UiState {
    #[default]
    Interactive,
    ShowingSaveFileDialog,
}

impl UiState {
    fn is_interactive(&self) -> bool {
        matches!(self, UiState::Interactive)
    }
}

// SYSTEMS

#[cfg(feature = "inspector")]
fn inspector_ui(world: &mut World) {
    let Ok((window, egui_context)) = world
        .query_filtered::<(&Window, &mut EguiContext), With<PrimaryWindow>>()
        .get_single(world)
    else {
        return;
    };
    let mut egui_context = egui_context.clone();

    egui::Window::new("Inspector")
        .default_open(false)
        .pivot(egui::Align2::CENTER_BOTTOM)
        .default_pos((
            window.width() / 2.0f32,
            f32::max(0.0f32, window.height() - 16.0f32),
        ))
        .show(egui_context.get_mut(), |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui_for_world(world, ui);
                ui.collapsing("State", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("UiState");
                        ui_for_state::<UiState>(world, ui);
                    });
                });
            });
        });
}

fn draw_ui_system(
    mut app_exit_events: EventWriter<AppExit>,
    mut commands: Commands,
    mut contexts: EguiContexts,
    layers_query: Query<(&mut layer::Layer, &mut layer::HeightMap)>,
    primary_window: Query<&Window, With<PrimaryWindow>>,
    mut save_file_dialogs: Query<&mut file_dialog::SaveFileDialog>,
    ui_state: Res<State<UiState>>,
    viewport_region: ResMut<viewport::ViewportRegion>,
) {
    let ctx = contexts.ctx_mut();

    let menubar_height: f32 = egui::TopBottomPanel::top("menubar")
        .show(ctx, |ui| {
            ui.add_enabled_ui(ui_state.is_interactive(), |ui| {
                draw_ui_menu(ui, &mut app_exit_events, &mut commands);
            });
        })
        .response
        .rect
        .height();

    let panel_left_width: f32 = egui::SidePanel::left("sidepanel_left")
        .resizable(true)
        .show(ctx, |ui| {
            ui.add_enabled_ui(ui_state.is_interactive(), |ui| {
                ui.heading("Side Panel Left");
                ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover());
            });
        })
        .response
        .rect
        .width();

    let panel_right_width: f32 = egui::SidePanel::right("sidepanel_right")
        .resizable(true)
        .show(ctx, |ui| {
            ui.add_enabled_ui(ui_state.is_interactive(), |ui| {
                ui.with_layout(egui::Layout::top_down_justified(egui::Align::Min), |ui| {
                    ui.heading("Side Panel Right");
                    draw_ui_for_layers(&mut commands, ui, layers_query);
                    // Normally this should be placed in between the top and
                    // bottom parts.  However `available_rect_before_wrap` takes
                    // up all the available space before layers are considered,
                    // and the layers will be pushed down and they become
                    // invisible.
                    //
                    // Similarly using a combo of `top_down` and `bottom_up`
                    // layouts don't work.  `with_layout` allocates all available
                    // space, so we can't nest a `top_down` inside the `bottom_up`
                    // and expect `available_rect_before_wrap` to work.
                    //
                    // We can save the space layers section take up in the
                    // previous frame and use that to allocate empty space in
                    // between but this solution makes the ui (slightly) more
                    // complex and I am not sure it's entirely reliable.
                    ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover());
                });
            });
        })
        .response
        .rect
        .width();

    if !ui_state.is_interactive() {
        if let Ok(mut dialog) = save_file_dialogs.get_single_mut() {
            dialog.show(ctx);
        }
    }

    set_viewport_region(
        menubar_height,
        panel_left_width,
        panel_right_width,
        primary_window,
        viewport_region,
    );
}

fn show_save_file_dialog_system(mut commands: Commands) {
    commands.spawn((
        Name::new("Save File Dialog"),
        file_dialog::SaveFileDialog::default(),
        StateScoped(UiState::ShowingSaveFileDialog),
    ));
}

// LIB

/// Draw the UI for the stack of layers in the project.
fn draw_ui_for_layers(
    commands: &mut Commands,
    ui: &mut egui::Ui,
    mut layers_query: Query<(&mut layer::Layer, &mut layer::HeightMap)>,
) {
    egui::containers::ScrollArea::vertical().show(ui, |ui| {
        ui.heading("Layers");
        if ui.button("New Layer").clicked() {
            commands.add(layer::CreateLayer::OnTop)
        }
        // We need to iterate layers in reverse order to place the topmost
        // (last applied) layer on top.
        for (mut layer, mut height_map) in layers_query.iter_mut().sort::<&layer::Layer>().rev() {
            let mut height_value: f32 = match *height_map {
                layer::HeightMap::Constant(v) => v,
            };
            ui.group(|ui| {
                ui.label(format!("{}", layer.as_ref()));
                ui.add(egui::widgets::DragValue::new(&mut height_value));
                ui.toggle_value(&mut layer.enable_preview, "preview");
                ui.toggle_value(&mut layer.enable_baking, "bake");
                if ui.button("Delete").clicked() {
                    commands.add(layer::DeleteLayer(layer.id()))
                }
            });
            match *height_map {
                layer::HeightMap::Constant(ref mut v) => *v = height_value,
            }
        }
    });
}

fn draw_ui_menu(
    ui: &mut egui::Ui,
    app_exit_events: &mut EventWriter<AppExit>,
    commands: &mut Commands,
) {
    egui::menu::bar(ui, |ui| {
        egui::menu::menu_button(ui, "File", |ui| {
            if ui.button("New").clicked() {
                commands.add(session::InitializeNewSession);
            }
            let _ = ui.button("Open...");
            // TODO: `Save` and `Save As` should be disabled only when there
            // are no changes to be saved.
            ui.add_enabled_ui(false, |ui| {
                let _ = ui.button("Save");
                let _ = ui.button("Save As...");
            });
            ui.separator();
            if ui.button("Quit").clicked() {
                app_exit_events.send(AppExit::Success);
            }
        });
    });
}

/// Update the region where the viewport is visible by deducting the areas
/// allocated to menubar and side panels.
fn set_viewport_region(
    menubar_height: f32,
    panel_left_width: f32,
    panel_right_width: f32,
    primary_window: Query<&Window, With<PrimaryWindow>>,
    mut viewport_region: ResMut<viewport::ViewportRegion>,
) {
    if let Ok(window) = primary_window.get_single() {
        let scale_factor: f32 = window.scale_factor();
        viewport_region.set_rect(Rect::new(
            panel_left_width * scale_factor,
            menubar_height * scale_factor,
            (window.physical_width() as f32) - panel_right_width * scale_factor,
            window.physical_height() as f32,
        ));
    }
}
