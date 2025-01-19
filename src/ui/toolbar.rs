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

use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::undo;

// PLUGIN

pub struct ToolbarPlugin;

impl Plugin for ToolbarPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ToolbarImages>();
    }
}

// RESOURCES

#[derive(Debug, Reflect, Resource)]
#[reflect(Resource)]
pub struct ToolbarImages {
    #[reflect(ignore)]
    undo_icon: egui::TextureId,
    #[reflect(ignore)]
    redo_icon: egui::TextureId,
}

impl FromWorld for ToolbarImages {
    fn from_world(world: &mut World) -> Self {
        let result: Self = world
            .run_system_once(
                |asset_server: Res<AssetServer>, mut contexts: EguiContexts| -> Self {
                    let undo_image_handle: Handle<Image> = asset_server.load("icons/undo.png");
                    let undo_image_texture_id = contexts.add_image(undo_image_handle);
                    let redo_image_handle: Handle<Image> = asset_server.load("icons/redo.png");
                    let redo_image_texture_id = contexts.add_image(redo_image_handle);
                    Self {
                        undo_icon: undo_image_texture_id,
                        redo_icon: redo_image_texture_id,
                    }
                },
            )
            .expect("Failed to load toolbar icons");
        result
    }
}

// LIB

pub fn draw_toolbar(
    commands: &mut Commands,
    ui: &mut egui::Ui,
    toolbar_images: &Res<ToolbarImages>,
    undo_stack: &Res<undo::UndoStack>,
) {
    ui.horizontal(|ui| {
        if ui
            .add_enabled(
                undo_stack.can_undo(),
                egui::widgets::ImageButton::new(egui::Image::new(egui::load::SizedTexture::new(
                    toolbar_images.undo_icon,
                    [64.0, 64.0],
                ))),
            )
            .clicked()
        {
            commands.queue(undo::UndoAction)
        }
        if ui
            .add_enabled(
                undo_stack.can_redo(),
                egui::widgets::ImageButton::new(egui::Image::new(egui::load::SizedTexture::new(
                    toolbar_images.redo_icon,
                    [64.0, 64.0],
                ))),
            )
            .clicked()
        {
            commands.queue(undo::RedoAction)
        }
    });
}
