use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};

#[cfg(feature = "inspector")]
use bevy::window::PrimaryWindow;
#[cfg(feature = "inspector")]
use bevy_egui::EguiContext;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin)
            .add_systems(Update, draw_ui_system);

        #[cfg(feature = "inspector")]
        app.add_systems(Update, inspector_ui);
    }
}

#[cfg(feature = "inspector")]
fn inspector_ui(world: &mut World) {
    let Ok(egui_context) = world
        .query_filtered::<&mut EguiContext, With<PrimaryWindow>>()
        .get_single(world)
    else {
        return;
    };
    let mut egui_context = egui_context.clone();

    egui::Window::new("Inspector")
        .default_open(false)
        .anchor(egui::Align2::RIGHT_BOTTOM, (-16.0f32, -16.0f32))
        .show(egui_context.get_mut(), |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                bevy_inspector_egui::bevy_inspector::ui_for_world(world, ui);
            });
        });
}

fn draw_ui_system(mut app_exit_events: EventWriter<AppExit>, mut contexts: EguiContexts) {
    let ctx = contexts.ctx_mut();

    egui::TopBottomPanel::top("menubar").show(ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            egui::menu::menu_button(ui, "File", |ui| {
                if ui.button("Quit").clicked() {
                    app_exit_events.send(AppExit::Success);
                }
            });
        });
    });

    egui::SidePanel::left("sidepanel_left")
        .resizable(true)
        .show(ctx, |ui| {
            ui.heading("Side Panel Left");
            ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover());
        });

    egui::SidePanel::right("sidepanel_right")
        .resizable(true)
        .show(ctx, |ui| {
            ui.heading("Side Panel Right");
            ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover());
        });
}
