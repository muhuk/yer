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
    colors: ThemeColors,

    #[reflect(ignore)]
    icon_atlas: egui::TextureId,
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
                .tint(self.colors.primary_color)
                .uv(egui::Rect::from_min_max(uv_min, uv_max)),
        );
        ui.add(widget)
    }
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

        let colors = ThemeColors {
            bg_color: srgba_to_color32(tailwind::STONE_800), // STONE_800
            mid_color: srgba_to_color32(tailwind::ZINC_400), // STONE_700
            fg_color: srgba_to_color32(tailwind::STONE_200), // STONE_200
            primary_color: srgba_to_color32(tailwind::AMBER_600), // AMBER_600
            secondary_color: srgba_to_color32(tailwind::YELLOW_950), // YELLOW_950
            test: srgba_to_color32(tailwind::FUCHSIA_500),
        };

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
        widgets.noninteractive.fg_stroke.color = theme.colors.mid_color;
        widgets.inactive.fg_stroke.color = theme.colors.fg_color;
        widgets.hovered.fg_stroke.color = theme.colors.primary_color;
        widgets.active.fg_stroke.color = theme.colors.secondary_color;
        widgets.open.fg_stroke.color = theme.colors.mid_color;

        // bg_fill
        widgets.noninteractive.bg_fill = theme.colors.bg_color;
        widgets.inactive.bg_fill = theme.colors.secondary_color;
        widgets.hovered.bg_fill = theme.colors.secondary_color;
        widgets.active.bg_fill = theme.colors.primary_color;
        widgets.open.bg_fill = theme.colors.secondary_color;

        // weak_bg_fill
        widgets.noninteractive.weak_bg_fill = theme.colors.bg_color;
        widgets.inactive.weak_bg_fill = theme.colors.secondary_color;
        widgets.hovered.weak_bg_fill = theme.colors.secondary_color;
        widgets.active.weak_bg_fill = theme.colors.primary_color;
        widgets.open.weak_bg_fill = theme.colors.secondary_color;

        widgets
    };
    let selection = {
        let mut selection = egui::style::Selection::default();
        selection.bg_fill = theme.colors.primary_color;
        selection.stroke.color = theme.colors.fg_color;
        selection
    };
    let visuals = egui::style::Visuals {
        dark_mode: true,
        override_text_color: None,
        widgets,
        selection,
        // hyperlink_color: theme.colors.mid_color,
        // faint_bg_color: theme.colors.test,
        extreme_bg_color: theme.colors.secondary_color,
        // code_bg_color: theme.colors.test,
        // warn_fg_color: theme.colors.test,
        // error_fg_color: theme.colors.test,
        // pub window_rounding: Rounding,
        // pub window_shadow: Shadow,
        window_fill: theme.colors.bg_color,
        // pub window_stroke: Stroke,
        // pub window_highlight_topmost: bool,
        // pub menu_rounding: Rounding,
        panel_fill: theme.colors.bg_color,
        // pub popup_shadow: Shadow,
        // pub resize_corner_size: f32,
        // pub text_cursor: TextCursorStyle,
        // pub clip_rect_margin: f32,
        // pub button_frame: bool,
        // pub collapsing_header_frame: bool,
        // pub indent_has_left_vline: bool,
        // pub striped: bool,
        // pub slider_trailing_fill: bool,
        // pub handle_shape: HandleShape,
        // pub interact_cursor: Option<CursorIcon>,
        // pub image_loading_spinners: bool,
        // pub numeric_color_space: NumericColorSpace,
        ..default()
    };
    ctx.set_visuals_of(EGUI_THEME, visuals);
}

// LIB

fn srgba_to_color32(value: Srgba) -> Color32 {
    let [r, g, b] = value.to_u8_array_no_alpha();
    Color32::from_rgb(r, g, b)
}

#[derive(Debug, Default)]
struct ThemeColors {
    bg_color: Color32,
    mid_color: Color32,
    fg_color: Color32,
    primary_color: Color32,
    secondary_color: Color32,
    test: Color32,
}
