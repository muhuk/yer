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

use bevy::color::palettes::tailwind;
use bevy::prelude::*;
use bevy_egui::{
    egui::{self, Color32},
    EguiContexts, EguiSet, EguiUserTextures,
};

// PLUGIN

pub struct ThemePlugin;

impl Plugin for ThemePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Theme>().add_systems(
            Startup,
            setup_egui_theme_system.after(EguiSet::InitContexts),
        );
    }
}

// RESOURCES

#[derive(Debug, Reflect, Resource)]
#[reflect(Resource)]
pub struct Theme {
    #[reflect(ignore)]
    pub colors: ThemeColors,

    #[reflect(ignore)]
    pub icon_atlas: egui::TextureId,
}

impl FromWorld for Theme {
    fn from_world(world: &mut World) -> Self {
        let icon_atlas = {
            let icon_atlas_handle: Handle<Image> =
                world.resource::<AssetServer>().load("images/icons.png");
            world
                .resource_mut::<EguiUserTextures>()
                .add_image(icon_atlas_handle)
        };

        // FIXME: Load theme from file.
        const BG_COLOR: Color = Color::Srgba(tailwind::STONE_800);
        const BG_ALT_COLOR: Color = Color::Srgba(tailwind::STONE_700);
        const FG_COLOR: Color = Color::Srgba(tailwind::STONE_200);
        const FG_ALT_COLOR: Color = Color::Srgba(tailwind::ZINC_400);
        const PRIMARY_COLOR: Color = Color::Srgba(tailwind::AMBER_600);
        const PRIMARY_ALT_COLOR: Color = Color::Srgba(tailwind::YELLOW_950);
        const SECONDARY_COLOR: Color = Color::Srgba(tailwind::BLUE_500);
        const SECONDARY_ALT_COLOR: Color = Color::Srgba(tailwind::BLUE_300);
        let colors = ThemeColors::new(
            BG_COLOR,
            BG_ALT_COLOR,
            FG_COLOR,
            FG_ALT_COLOR,
            PRIMARY_COLOR,
            PRIMARY_ALT_COLOR,
            SECONDARY_COLOR,
            SECONDARY_ALT_COLOR,
        );

        Self { colors, icon_atlas }
    }
}

// SYSTEMS

fn setup_egui_theme_system(mut contexts: EguiContexts, theme: Res<Theme>) {
    const EGUI_THEME: egui::Theme = egui::Theme::Dark;
    let ctx = contexts.ctx_mut();
    ctx.set_theme(EGUI_THEME);
    let widgets = {
        let mut widgets = egui::style::Widgets::default();
        // fg_stroke
        widgets.noninteractive.fg_stroke.color = theme.colors.fg_alt_color.to_color32();
        widgets.inactive.fg_stroke.color = theme.colors.fg_color.to_color32();
        widgets.hovered.fg_stroke.color = theme.colors.fg_color.to_color32();
        widgets.active.fg_stroke.color = theme.colors.fg_color.to_color32();
        widgets.open.fg_stroke.color = theme.colors.primary_color.to_color32();

        // bg_fill
        widgets.noninteractive.bg_fill = theme.colors.bg_alt_color.to_color32();
        widgets.inactive.bg_fill = theme.colors.bg_alt_color.to_color32();
        widgets.hovered.bg_fill = theme.colors.primary_color.to_color32();
        widgets.active.bg_fill = theme.colors.primary_color.to_color32();
        widgets.open.bg_fill = theme.colors.primary_alt_color.to_color32();

        // weak_bg_fill
        widgets.noninteractive.weak_bg_fill = theme
            .colors
            .bg_alt_color
            .mix(&theme.colors.bg_color, 0.5)
            .to_color32();
        widgets.inactive.weak_bg_fill = theme.colors.bg_alt_color.to_color32();
        widgets.hovered.weak_bg_fill = theme.colors.bg_alt_color.to_color32();
        widgets.active.weak_bg_fill = theme.colors.bg_alt_color.to_color32();
        widgets.open.weak_bg_fill = theme.colors.bg_alt_color.to_color32();

        widgets
    };
    let selection = {
        let mut selection = egui::style::Selection::default();
        selection.bg_fill = theme.colors.bg_alt_color.to_color32();
        selection.stroke.color = theme.colors.primary_color.to_color32();
        selection
    };
    let text_cursor = {
        let mut text_cursor = egui::style::TextCursorStyle::default();
        text_cursor.stroke.color = theme.colors.primary_color.to_color32();
        text_cursor
    };
    let visuals = egui::style::Visuals {
        dark_mode: true,
        widgets,
        selection,
        hyperlink_color: theme.colors.secondary_color.to_color32(),
        faint_bg_color: theme
            .colors
            .bg_color
            .mix(&theme.colors.secondary_alt_color, 0.125)
            .to_color32(),
        extreme_bg_color: theme
            .colors
            .bg_color
            .mix(&theme.colors.secondary_alt_color, 0.25)
            .to_color32(),
        window_fill: theme
            .colors
            .bg_color
            .mix(&theme.colors.secondary_color, 0.0625)
            .to_color32(),
        panel_fill: theme.colors.bg_color.to_color32(),
        text_cursor,
        button_frame: true,
        ..default()
    };
    ctx.set_visuals_of(EGUI_THEME, visuals);
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

pub trait ToColor32 {
    fn to_color32(self) -> Color32;
}

impl ToColor32 for Color {
    fn to_color32(self) -> Color32 {
        let [r, g, b] = self.to_srgba().to_u8_array_no_alpha();
        Color32::from_rgb(r, g, b)
    }
}

#[derive(Debug, Default)]
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

impl ThemeColors {
    fn new(
        bg_color: Color,
        bg_alt_color: Color,
        fg_color: Color,
        fg_alt_color: Color,
        primary_color: Color,
        primary_alt_color: Color,
        secondary_color: Color,
        secondary_alt_color: Color,
    ) -> Self {
        Self {
            bg_color,
            bg_alt_color,
            fg_color,
            fg_alt_color,
            primary_color,
            primary_alt_color,
            secondary_color,
            secondary_alt_color,
        }
    }
}
