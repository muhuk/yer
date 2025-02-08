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
use bevy::window::PrimaryWindow;
use bevy_egui::{egui, EguiContexts, EguiPlugin};

#[cfg(feature = "inspector")]
use bevy_egui::EguiContext;
#[cfg(feature = "inspector")]
use bevy_inspector_egui::{bevy_inspector, DefaultInspectorConfigPlugin};

use crate::constants;
use crate::session;
use crate::undo;
use crate::viewport;

mod file_dialog;
mod layer;
mod preview;
mod theme;
mod toolbar;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<UiState>()
            .add_plugins((
                EguiPlugin,
                file_dialog::UiFileDialogPlugin,
                layer::LayerUiPlugin,
                theme::ThemePlugin,
            ))
            .init_state::<UiState>()
            .enable_state_scoped_entities::<UiState>()
            .add_systems(
                Update,
                (
                    draw_ui_panels_system,
                    draw_ui_dialogs_system.after(draw_ui_panels_system),
                    update_window_title_system
                        .run_if(resource_exists_and_changed::<session::Session>),
                ),
            )
            .add_systems(
                OnEnter(UiState::ShowingLoadFileDialog),
                show_load_file_dialog_system,
            )
            .add_systems(
                OnEnter(UiState::ShowingSaveFileDialog),
                show_save_file_dialog_system,
            );

        #[cfg(feature = "inspector")]
        app.add_plugins(DefaultInspectorConfigPlugin)
            .add_systems(Update, inspector_ui_system);
    }
}

// RESOURCES

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq, Reflect, States)]
enum UiState {
    #[default]
    Interactive,
    ShowingLoadFileDialog,
    ShowingSaveFileDialog,
}

impl UiState {
    fn is_interactive(&self) -> bool {
        matches!(self, UiState::Interactive)
    }
}

// SYSTEMS

#[cfg(feature = "inspector")]
fn inspector_ui_system(world: &mut World) {
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
                egui::CollapsingHeader::new("Entities")
                    .default_open(true)
                    .show(ui, |ui| {
                        bevy_inspector::ui_for_world_entities_filtered::<(
                            Without<Parent>,
                            Without<Observer>,
                        )>(world, ui, true);
                    });
                ui.collapsing("Resources", |ui| {
                    bevy_inspector::ui_for_resources(world, ui);
                });
                ui.collapsing("Assets", |ui| {
                    bevy_inspector::ui_for_all_assets(world, ui);
                });
                //bevy_inspector::ui_for_world(world, ui);
                ui.collapsing("State", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("UiState");
                        bevy_inspector::ui_for_state::<UiState>(world, ui);
                    });
                });
            });
        });
}

fn draw_ui_dialogs_system(
    mut commands: Commands,
    mut contexts: EguiContexts,
    mut load_file_dialogs: Query<&mut file_dialog::LoadFileDialog>,
    mut save_file_dialogs: Query<&mut file_dialog::SaveFileDialog>,
    ui_state: Res<State<UiState>>,
    mut ui_state_next: ResMut<NextState<UiState>>,
) {
    if let Some(ctx) = contexts.try_ctx_mut() {
        if !ui_state.is_interactive() {
            match ui_state.as_ref().get() {
                UiState::Interactive => unreachable!(),
                UiState::ShowingLoadFileDialog => {
                    if let Ok(mut dialog) = load_file_dialogs.get_single_mut() {
                        match dialog.show(ctx) {
                            file_dialog::DialogState::Open => (),
                            file_dialog::DialogState::Selected(path) => {
                                ui_state_next.set(UiState::Interactive);
                                commands.queue(session::LoadSession(path));
                            }
                            file_dialog::DialogState::Cancelled => {
                                // Currently there is no cleanup necessary.  If there is
                                // need for cleanup in the future it should ideally be
                                // handled by a OnExit(state) system.
                                ui_state_next.set(UiState::Interactive);
                            }
                        }
                    }
                }
                UiState::ShowingSaveFileDialog => {
                    if let Ok(mut dialog) = save_file_dialogs.get_single_mut() {
                        match dialog.show(ctx) {
                            file_dialog::DialogState::Open => (),
                            file_dialog::DialogState::Selected(path) => {
                                ui_state_next.set(UiState::Interactive);
                                commands.queue(session::SaveSession(Some(path)));
                            }
                            file_dialog::DialogState::Cancelled => {
                                // Currently there is no cleanup necessary.  If there is
                                // need for cleanup in the future it should ideally be
                                // handled by a OnExit(state) system.
                                ui_state_next.set(UiState::Interactive);
                            }
                        }
                    }
                }
            }
        }
    }
}

fn draw_ui_panels_system(
    mut app_exit_events: EventWriter<AppExit>,
    mut commands: Commands,
    mut contexts: EguiContexts,
    layers_query: layer::LayersQuery,
    preview_query: preview::PreviewQuery,
    primary_window: Query<&Window, With<PrimaryWindow>>,
    session: Res<session::Session>,
    theme: Res<theme::Theme>,
    theme_colors: Res<Assets<theme::ThemeColors>>,
    undo_stack: Res<undo::UndoStack>,
    mut ui_state_next: ResMut<NextState<UiState>>,
    viewport_region: ResMut<viewport::ViewportRegion>,
) {
    let ctx = contexts.ctx_mut();

    let menubar_height: f32 = egui::TopBottomPanel::top("menubar")
        .show(ctx, |ui| {
            draw_ui_menu(
                ui,
                &mut app_exit_events,
                &mut commands,
                session.as_ref(),
                undo_stack.as_ref(),
                &mut ui_state_next,
            );
        })
        .response
        .rect
        .height();

    let toolbar_height: f32 = egui::TopBottomPanel::top("toolbar")
        .show(ctx, |ui| {
            if let Some(colors) = theme_colors.get(&theme.colors) {
                toolbar::draw_toolbar(&mut commands, ui, &theme, colors, &undo_stack);
            }
        })
        .response
        .rect
        .height();

    let panel_left_width: f32 = egui::SidePanel::left("sidepanel_left")
        .resizable(true)
        .show(ctx, |ui| {
            ui.heading("Side Panel Left");
            preview::draw_ui_for_preview(ui, preview_query);
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
                layer::draw_ui_for_layers(&mut commands, ui, layers_query);
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
        })
        .response
        .rect
        .width();

    set_viewport_region(
        menubar_height,
        toolbar_height,
        panel_left_width,
        panel_right_width,
        primary_window,
        viewport_region,
    );
}

fn show_load_file_dialog_system(mut commands: Commands) {
    // Set dir & file name.
    commands.spawn((
        Name::new("Load File Dialog"),
        file_dialog::LoadFileDialog::default(),
        StateScoped(UiState::ShowingLoadFileDialog),
    ));
}

fn show_save_file_dialog_system(mut commands: Commands) {
    commands.spawn((
        Name::new("Save File Dialog"),
        file_dialog::SaveFileDialog::default(),
        StateScoped(UiState::ShowingSaveFileDialog),
    ));
}

fn update_window_title_system(
    session: Res<session::Session>,
    mut primary_window: Query<&mut Window, With<PrimaryWindow>>,
) {
    info!("Updating window title.");
    if let Ok(mut window) = primary_window.get_single_mut() {
        let file_path_part: String = session
            .get_file_path()
            .map(|p| p.to_string_lossy().into())
            .unwrap_or("(Unsaved)".to_string());
        let save_status_part: &str = if session.has_unsaved_changes() {
            "*"
        } else {
            ""
        };
        window.title = format!(
            "{} {} — {}",
            save_status_part,
            file_path_part,
            constants::APPLICATION_TITLE
        );
    }
}

// LIB

fn draw_ui_menu(
    ui: &mut egui::Ui,
    app_exit_events: &mut EventWriter<AppExit>,
    commands: &mut Commands,
    session: &session::Session,
    undo_stack: &undo::UndoStack,
    ui_state_next: &mut ResMut<NextState<UiState>>,
) {
    egui::menu::bar(ui, |ui| {
        ui.menu_button("File", |ui| {
            if ui.button("New").clicked() {
                commands.queue(session::InitializeNewSession);
                ui.close_menu();
            }
            if ui.button("Open...").clicked() {
                ui_state_next.set(UiState::ShowingLoadFileDialog);
                ui.close_menu();
            }
            if ui.button("Save").clicked() {
                if session.has_save_file() {
                    commands.queue(session::SaveSession(None));
                } else {
                    ui_state_next.set(UiState::ShowingSaveFileDialog);
                }
                ui.close_menu();
            }
            if ui.button("Save As...").clicked() {
                ui_state_next.set(UiState::ShowingSaveFileDialog);
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Quit").clicked() {
                app_exit_events.send(AppExit::Success);
                ui.close_menu();
            }
        });

        ui.menu_button("Edit", |ui| {
            if ui
                .add_enabled_ui(undo_stack.can_undo(), |ui| ui.button("Undo"))
                .inner
                .clicked()
            {
                commands.queue(undo::UndoAction);
                ui.close_menu();
            }
            if ui
                .add_enabled_ui(undo_stack.can_redo(), |ui| ui.button("Redo"))
                .inner
                .clicked()
            {
                commands.queue(undo::RedoAction);
                ui.close_menu();
            }
        });
    });
}

/// Update the region where the viewport is visible by deducting the areas
/// allocated to menubar and side panels.
fn set_viewport_region(
    menubar_height: f32,
    toolbar_height: f32,
    panel_left_width: f32,
    panel_right_width: f32,
    primary_window: Query<&Window, With<PrimaryWindow>>,
    mut viewport_region: ResMut<viewport::ViewportRegion>,
) {
    if let Ok(window) = primary_window.get_single() {
        let scale_factor: f32 = window.scale_factor();
        viewport_region.set_rect(Rect::new(
            panel_left_width * scale_factor,
            (menubar_height + toolbar_height) * scale_factor,
            (window.physical_width() as f32) - panel_right_width * scale_factor,
            window.physical_height() as f32,
        ));
    }
}
