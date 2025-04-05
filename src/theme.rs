// Copyright © 2024-2025 Atamert Ölçgen.
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
use bevy_common_assets::toml::TomlAssetPlugin;
use serde::Deserialize;

// PLUGIN

pub struct ThemePlugin;

impl Plugin for ThemePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Theme>()
            .register_asset_reflect::<ThemeColors>()
            .init_asset::<ThemeColors>()
            .add_plugins(TomlAssetPlugin::<ThemeColors>::new(&[".color_theme.toml"]))
            .init_resource::<Theme>();
    }
}

// RESOURCES

#[derive(Debug, Reflect, Resource)]
#[reflect(Resource)]
pub struct Theme {
    pub colors: Handle<ThemeColors>,
    pub icon_atlas: Handle<Image>,
}

impl FromWorld for Theme {
    fn from_world(world: &mut World) -> Self {
        let icon_atlas = world.resource::<AssetServer>().load("images/icons.png");
        let colors = world
            .resource::<AssetServer>()
            .load("themes/dark.color_theme.toml");
        Self { colors, icon_atlas }
    }
}

// ASSETS

#[derive(Asset, Debug, Deserialize, Reflect)]
pub struct ThemeColors {
    pub bg_color: Color,
    pub bg_alt_color: Color,
    pub fg_color: Color,
    pub fg_alt_color: Color,
    pub primary_color: Color,
    pub primary_alt_color: Color,
    pub secondary_color: Color,
    pub secondary_alt_color: Color,
}

// LIB

#[derive(Copy, Clone, Debug)]
pub enum IconAtlasSprite {
    Undo,
    Redo,
}

impl Into<UVec2> for IconAtlasSprite {
    fn into(self) -> UVec2 {
        match self {
            Self::Undo => UVec2::new(0, 1),
            Self::Redo => UVec2::new(1, 1),
        }
    }
}
