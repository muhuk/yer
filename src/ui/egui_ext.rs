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
use bevy_egui::{
    egui::{self, Color32},
    EguiContexts, EguiSet, EguiUserTextures,
};

use crate::theme;

// PLUGIN

pub struct UiBevyExtPlugin;

impl Plugin for UiBevyExtPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<EguiTheme>()
            .init_resource::<EguiTheme>()
            .add_systems(
                Update,
                update_egui_theme_system.after(EguiSet::InitContexts),
            );
    }
}

// RESOURCES

#[derive(Debug, Reflect, Resource)]
#[reflect(Resource)]
pub struct EguiTheme {
    #[reflect(ignore)]
    pub icon_atlas: egui::TextureId,
}

impl FromWorld for EguiTheme {
    fn from_world(world: &mut World) -> Self {
        // FIXME: Make icon_atlas updateable.
        //
        // This is currently just taking the icon atlas loaded
        // on startup and does not update if the Theme changes.
        // See also [update_egui_theme_system].
        let icon_atlas_handle: Handle<Image> = world.resource::<theme::Theme>().icon_atlas.clone();
        let icon_atlas_texture_id = world
            .resource_mut::<EguiUserTextures>()
            .add_image(icon_atlas_handle);
        Self {
            icon_atlas: icon_atlas_texture_id,
        }
    }
}

// SYSTEMS

fn update_egui_theme_system(
    mut contexts: EguiContexts,
    theme: Res<theme::Theme>,
    theme_colors: Res<Assets<theme::ThemeColors>>,
) {
    if !theme.is_changed() {
        return;
    }

    const EGUI_THEME: egui::Theme = egui::Theme::Dark;
    if let Some(colors) = theme_colors.get(&theme.colors) {
        debug!("Updating theme.");
        let ctx = contexts.ctx_mut();
        ctx.set_theme(EGUI_THEME);
        let widgets = {
            let mut widgets = egui::style::Widgets::default();
            // fg_stroke
            widgets.noninteractive.fg_stroke.color = colors.fg_alt_color.to_color32();
            widgets.inactive.fg_stroke.color = colors.fg_color.to_color32();
            widgets.hovered.fg_stroke.color = colors.fg_color.to_color32();
            widgets.active.fg_stroke.color = colors.fg_color.to_color32();
            widgets.open.fg_stroke.color = colors.primary_color.to_color32();

            // bg_fill
            widgets.noninteractive.bg_fill = colors.bg_alt_color.to_color32();
            widgets.inactive.bg_fill = colors.bg_alt_color.to_color32();
            widgets.hovered.bg_fill = colors.primary_color.to_color32();
            widgets.active.bg_fill = colors.primary_color.to_color32();
            widgets.open.bg_fill = colors.primary_alt_color.to_color32();

            // weak_bg_fill
            widgets.noninteractive.weak_bg_fill =
                colors.bg_alt_color.mix(&colors.bg_color, 0.5).to_color32();
            widgets.inactive.weak_bg_fill = colors.bg_alt_color.to_color32();
            widgets.hovered.weak_bg_fill = colors.bg_alt_color.to_color32();
            widgets.active.weak_bg_fill = colors.bg_alt_color.to_color32();
            widgets.open.weak_bg_fill = colors.bg_alt_color.to_color32();

            widgets
        };
        let selection = {
            let mut selection = egui::style::Selection::default();
            selection.bg_fill = colors.bg_alt_color.to_color32();
            selection.stroke.color = colors.primary_color.to_color32();
            selection
        };
        let text_cursor = {
            let mut text_cursor = egui::style::TextCursorStyle::default();
            text_cursor.stroke.color = colors.primary_color.to_color32();
            text_cursor
        };
        let visuals = egui::style::Visuals {
            dark_mode: true,
            widgets,
            selection,
            hyperlink_color: colors.secondary_color.to_color32(),
            faint_bg_color: colors
                .bg_color
                .mix(&colors.secondary_alt_color, 0.125)
                .to_color32(),
            extreme_bg_color: colors
                .bg_color
                .mix(&colors.secondary_alt_color, 0.25)
                .to_color32(),
            window_fill: colors
                .bg_color
                .mix(&colors.secondary_color, 0.0625)
                .to_color32(),
            panel_fill: colors.bg_color.to_color32(),
            text_cursor,
            button_frame: true,
            ..default()
        };
        ctx.set_visuals_of(EGUI_THEME, visuals);
    }
}

// LIB

pub trait ToColor32 {
    fn to_color32(self) -> Color32;
}

impl ToColor32 for Color {
    fn to_color32(self) -> Color32 {
        let [r, g, b] = self.to_srgba().to_u8_array_no_alpha();
        Color32::from_rgb(r, g, b)
    }
}
