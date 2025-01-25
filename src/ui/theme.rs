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

// PLUGIN

pub struct ThemePlugin;

impl Plugin for ThemePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Theme>();
    }
}

// RESOURCES

#[derive(Debug, Reflect, Resource)]
#[reflect(Resource)]
pub struct Theme {
    #[reflect(ignore)]
    icon_atlas: egui::TextureId,
}

impl FromWorld for Theme {
    fn from_world(world: &mut World) -> Self {
        let icon_atlas_handle: Handle<Image> =
            world.resource::<AssetServer>().load("images/icons.png");
        let icon_atlas = world
            .resource_mut::<EguiUserTextures>()
            .add_image(icon_atlas_handle);
        Self { icon_atlas }
    }
}

impl Theme {
    pub fn draw_toolbar_button(&self, ui: &mut egui::Ui, sprite_index: UVec2) -> egui::Response {
        const ICON_SIZE: [f32; 2] = [32.0, 32.0];
        const SPRITE_SIZE: f32 = 0.125f32;
        let uv_min = egui::Pos2::new(
            sprite_index.x as f32 * SPRITE_SIZE,
            sprite_index.y as f32 * SPRITE_SIZE,
        );
        let uv_max = uv_min + egui::Vec2::splat(SPRITE_SIZE);

        let widget = egui::widgets::ImageButton::new(
            egui::Image::new(egui::load::SizedTexture::new(self.icon_atlas, ICON_SIZE))
                .uv(egui::Rect::from_min_max(uv_min, uv_max)),
        );
        ui.add(widget)
    }
}
