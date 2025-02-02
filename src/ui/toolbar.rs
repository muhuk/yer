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
use bevy_egui::egui;

use crate::ui::theme::{IconAtlasSprite, Theme, ToColor32};
use crate::undo;

// LIB

pub fn draw_toolbar(
    commands: &mut Commands,
    ui: &mut egui::Ui,
    theme: &Res<Theme>,
    undo_stack: &Res<undo::UndoStack>,
) {
    ui.horizontal(|ui| {
        // TODO: Add tooltips.
        if ui
            .add_enabled_ui(undo_stack.can_undo(), |ui| {
                draw_toolbar_button(theme, ui, IconAtlasSprite::Undo)
            })
            .inner
            .clicked()
        {
            commands.queue(undo::UndoAction)
        }
        if ui
            .add_enabled_ui(undo_stack.can_redo(), |ui| {
                draw_toolbar_button(theme, ui, IconAtlasSprite::Redo)
            })
            .inner
            .clicked()
        {
            commands.queue(undo::RedoAction)
        }
    });
}

fn draw_toolbar_button(
    theme: &Res<Theme>,
    ui: &mut egui::Ui,
    sprite: IconAtlasSprite,
) -> egui::Response {
    const ICON_SIZE: [f32; 2] = [32.0, 32.0];
    const SPRITE_SIZE: f32 = 0.125f32;
    let sprite_index: UVec2 = sprite.into();
    let uv_min = egui::Pos2::new(
        sprite_index.x as f32 * SPRITE_SIZE,
        sprite_index.y as f32 * SPRITE_SIZE,
    );
    let uv_max = uv_min + egui::Vec2::splat(SPRITE_SIZE);

    let widget = egui::widgets::ImageButton::new(
        egui::Image::new(egui::load::SizedTexture::new(theme.icon_atlas, ICON_SIZE))
            .tint(theme.colors.fg_color.to_color32())
            .uv(egui::Rect::from_min_max(uv_min, uv_max)),
    );
    ui.add(widget)
}
