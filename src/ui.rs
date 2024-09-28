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
use bevy_inspector_egui::DefaultInspectorConfigPlugin;

use crate::layer;
use crate::viewport;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin)
            .add_systems(Update, draw_ui_system);

        #[cfg(feature = "inspector")]
        app.add_plugins(DefaultInspectorConfigPlugin)
            .add_systems(Update, inspector_ui);
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
                bevy_inspector_egui::bevy_inspector::ui_for_world(world, ui);
            });
        });
}

fn draw_ui_system(
    mut app_exit_events: EventWriter<AppExit>,
    mut commands: Commands,
    mut contexts: EguiContexts,
    layers_query: Query<&mut layer::Layer>,
    primary_window: Query<&Window, With<PrimaryWindow>>,
    viewport_region: ResMut<viewport::ViewportRegion>,
) {
    let ctx = contexts.ctx_mut();

    let menubar_height: f32 = egui::TopBottomPanel::top("menubar")
        .show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                egui::menu::menu_button(ui, "File", |ui| {
                    if ui.button("Quit").clicked() {
                        app_exit_events.send(AppExit::Success);
                    }
                });
            });
        })
        .response
        .rect
        .height();

    let panel_left_width: f32 = egui::SidePanel::left("sidepanel_left")
        .resizable(true)
        .show(ctx, |ui| {
            ui.heading("Side Panel Left");
            ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover());
        })
        .response
        .rect
        .width();

    let panel_right_width: f32 = egui::SidePanel::right("sidepanel_right")
        .resizable(true)
        .show(ctx, |ui| {
            ui.with_layout(egui::Layout::top_down_justified(egui::Align::Min), |ui| {
                ui.heading("Side Panel Right");
                draw_ui_layers(&mut commands, ui, layers_query);
                // Notmally this should be placed in between the top and
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
        })
        .response
        .rect
        .width();

    set_viewport_region(
        menubar_height,
        panel_left_width,
        panel_right_width,
        primary_window,
        viewport_region,
    );
}

// LIB

/// Draw the UI for the stack of layers in the project.
fn draw_ui_layers(
    commands: &mut Commands,
    ui: &mut egui::Ui,
    mut layers_query: Query<&mut layer::Layer>,
) {
    egui::containers::ScrollArea::vertical().show(ui, |ui| {
        ui.heading("Layers");
        if ui.button("New Layer").clicked() {
            commands.add(layer::CreateLayer::OnTop)
        }
        // We need to iterate layers in reverse order to place the topmost
        // (last applied) layer on top.
        for mut layer in layers_query.iter_mut().sort::<&layer::Layer>().rev() {
            ui.group(|ui| {
                ui.label(format!("{}", layer.as_ref()));
                ui.toggle_value(&mut layer.enable_preview, "preview");
                ui.toggle_value(&mut layer.enable_baking, "bake");
                if ui.button("Delete").clicked() {
                    commands.add(layer::DeleteLayer(layer.id()))
                }
            });
        }
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
