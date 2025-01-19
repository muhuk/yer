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
use bevy_egui::{egui, EguiUserTextures};

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
    icon_atlas: egui::TextureId,
}

impl FromWorld for ToolbarImages {
    fn from_world(world: &mut World) -> Self {
        let icon_atlas_handle: Handle<Image> =
            world.resource::<AssetServer>().load("images/icons.png");
        let icon_atlas = world
            .resource_mut::<EguiUserTextures>()
            .add_image(icon_atlas_handle);
        Self { icon_atlas }
    }
}

// LIB

pub fn draw_toolbar(
    commands: &mut Commands,
    ui: &mut egui::Ui,
    toolbar_images: &Res<ToolbarImages>,
    undo_stack: &Res<undo::UndoStack>,
) {
    const ICON_SIZE: [f32; 2] = [32.0, 32.0];
    const UNDO_UV: egui::Rect = egui::Rect {
        min: egui::Pos2::new(0.0f32, 0.125f32),
        max: egui::Pos2::new(0.125f32, 0.25f32),
    };
    const REDO_UV: egui::Rect = egui::Rect {
        min: egui::Pos2::new(0.125f32, 0.125f32),
        max: egui::Pos2::new(0.25f32, 0.25f32),
    };

    ui.horizontal(|ui| {
        if ui
            .add_enabled(
                undo_stack.can_undo(),
                egui::widgets::ImageButton::new(
                    egui::Image::new(egui::load::SizedTexture::new(
                        toolbar_images.icon_atlas,
                        ICON_SIZE,
                    ))
                    .uv(UNDO_UV),
                ),
            )
            .clicked()
        {
            commands.queue(undo::UndoAction)
        }
        if ui
            .add_enabled(
                undo_stack.can_redo(),
                egui::widgets::ImageButton::new(
                    egui::Image::new(egui::load::SizedTexture::new(
                        toolbar_images.icon_atlas,
                        ICON_SIZE,
                    ))
                    .uv(REDO_UV),
                ),
            )
            .clicked()
        {
            commands.queue(undo::RedoAction)
        }
    });
}
