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
                .tint(self.colors.primary_color.to_color32())
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
    const TEST_COLOR: Color = Color::Srgba(tailwind::FUCHSIA_500);
    let ctx = contexts.ctx_mut();
    ctx.set_theme(EGUI_THEME);
    let widgets = {
        let mut widgets = egui::style::Widgets::default();
        // fg_stroke
        widgets.noninteractive.fg_stroke.color = theme.colors.fg_alt_color.to_color32();
        widgets.inactive.fg_stroke.color = theme.colors.fg_color.to_color32();
        widgets.hovered.fg_stroke.color = theme.colors.fg_color.to_color32();
        widgets.active.fg_stroke.color = theme.colors.primary_color.to_color32();
        widgets.open.fg_stroke.color = theme.colors.primary_color.to_color32();

        // bg_fill
        widgets.noninteractive.bg_fill = theme.colors.bg_alt_color.to_color32();
        widgets.inactive.bg_fill = theme.colors.bg_alt_color.to_color32();
        widgets.hovered.bg_fill = theme.colors.bg_alt_color.to_color32();
        widgets.active.bg_fill = theme.colors.bg_alt_color.to_color32();
        widgets.open.bg_fill = theme.colors.bg_alt_color.to_color32();

        // weak_bg_fill
        widgets.noninteractive.weak_bg_fill = theme.colors.bg_color.to_color32();
        widgets.inactive.weak_bg_fill = theme.colors.bg_color.to_color32();
        widgets.hovered.weak_bg_fill = theme.colors.bg_color.to_color32();
        widgets.active.weak_bg_fill = theme.colors.bg_color.to_color32();
        widgets.open.weak_bg_fill = theme.colors.bg_color.to_color32();

        widgets
    };
    let selection = {
        let mut selection = egui::style::Selection::default();
        selection.bg_fill = theme.colors.primary_alt_color.to_color32();
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
        override_text_color: None,
        widgets,
        selection,
        hyperlink_color: TEST_COLOR.to_color32(),
        faint_bg_color: theme
            .colors
            .bg_color
            .mix(&theme.colors.bg_alt_color, 0.333)
            .to_color32(),
        extreme_bg_color: theme
            .colors
            .bg_color
            .mix(&theme.colors.bg_alt_color, 0.667)
            .to_color32(),
        // code_bg_color: theme.colors.test,
        // warn_fg_color: theme.colors.test,
        // error_fg_color: theme.colors.test,
        // pub window_rounding: Rounding,
        // pub window_shadow: Shadow,
        window_fill: theme
            .colors
            .bg_color
            .mix(&theme.colors.secondary_color, 0.0625)
            .to_color32(),
        // pub window_stroke: Stroke,
        // pub window_highlight_topmost: bool,
        // pub menu_rounding: Rounding,
        panel_fill: theme
            .colors
            .bg_color
            .mix(&theme.colors.secondary_color, 0.03125)
            .to_color32(),
        // pub popup_shadow: Shadow,
        // pub resize_corner_size: f32,
        text_cursor,
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

trait ToColor32 {
    fn to_color32(self) -> Color32;
}

impl ToColor32 for Color {
    fn to_color32(self) -> Color32 {
        let [r, g, b] = self.to_srgba().to_u8_array_no_alpha();
        Color32::from_rgb(r, g, b)
    }
}

#[derive(Debug, Default)]
struct ThemeColors {
    bg_color: Color,
    bg_alt_color: Color,
    fg_color: Color,
    fg_alt_color: Color,
    primary_color: Color,
    primary_alt_color: Color,
    secondary_color: Color,
    secondary_alt_color: Color,
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
