// Copyright © 2024-2026 Atamert Ölçgen.
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
use std::ops::RangeInclusive;

use bevy::prelude::*;
use bevy_egui::{
    egui::{self, Color32},
    EguiContexts, EguiPreUpdateSet, EguiTextureHandle, EguiUserTextures,
};

use crate::theme;

const DEFAULT_SPEED: f32 = 0.05;

// PLUGIN

pub struct UiBevyExtPlugin;

impl Plugin for UiBevyExtPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<EguiTheme>()
            .init_resource::<EguiTheme>()
            .add_systems(
                Update,
                (
                    update_egui_theme_system
                        .run_if(resource_exists_and_changed::<theme::Theme>)
                        .after(EguiPreUpdateSet::InitContexts),
                    premultiply_alpha_for_images_system.run_if(on_message::<AssetEvent<Image>>),
                ),
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

impl EguiTheme {
    fn set_icon_atlas(&mut self, egui_user_textures: &mut EguiUserTextures, handle: Handle<Image>) {
        let texture_id = egui_user_textures.add_image(EguiTextureHandle::Strong(handle));
        self.icon_atlas = texture_id;
    }
}

impl FromWorld for EguiTheme {
    fn from_world(world: &mut World) -> Self {
        // FIXME: Make icon_atlas updateable.
        //
        // This is currently just taking the icon atlas loaded
        // on startup and does not update if the Theme changes.
        // See also [update_egui_theme_system].
        //
        // Also this works purely because by this resource gets
        // instantiated after Theme resource.  Ideally it should
        // be tracking changes on Theme anyway.
        let icon_atlas_handle: Handle<Image> = world.resource::<theme::Theme>().icon_atlas.clone();
        let icon_atlas_texture_id = world
            .resource_mut::<EguiUserTextures>()
            .add_image(EguiTextureHandle::Weak(icon_atlas_handle.id()));
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
) -> Result<(), BevyError> {
    const EGUI_THEME: egui::Theme = egui::Theme::Dark;
    if let Some(colors) = theme_colors.get(&theme.colors) {
        debug!("Updating theme.");
        let ctx = contexts.ctx_mut()?;
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

    Ok(())
}

fn premultiply_alpha_for_images_system(
    mut messages: MessageReader<AssetEvent<Image>>,
    mut egui_theme: ResMut<EguiTheme>,
    mut egui_user_textures: ResMut<EguiUserTextures>,
    theme: Res<theme::Theme>,
    mut images: ResMut<Assets<Image>>,
) {
    for asset_event in messages.read() {
        if let AssetEvent::LoadedWithDependencies { id } = asset_event {
            if *id == theme.icon_atlas.id() {
                info!("Premultiplying icon atlas image.");
                let mut image = images.get(*id).expect("should have loaded image").clone();
                premultiply_alpha(&mut image);
                let handle = images.add(image);
                egui_theme.set_icon_atlas(egui_user_textures.as_mut(), handle);
            }
        }
    }
}

// LIB

pub trait ToColor32 {
    fn to_color32(self) -> Color32;
}

impl ToColor32 for Color {
    fn to_color32(self) -> Color32 {
        let [r, g, b, a] = self.to_srgba().to_u8_array();
        Color32::from_rgba_unmultiplied(r, g, b, a)
    }
}

pub fn draw_ui_editable_f32(
    range: Option<RangeInclusive<f32>>,
    speed: Option<f32>,
    ui: &mut egui::Ui,
    value: f32,
) -> Option<f32> {
    let mut value_edited: f32 = value;
    let mut widget = egui::widgets::DragValue::new(&mut value_edited)
        .update_while_editing(false)
        .speed(speed.unwrap_or(DEFAULT_SPEED));
    if let Some(range) = range {
        widget = widget.range(range);
    }
    let response = ui.add(widget);
    if response.changed() && value_edited != value {
        Some(value_edited)
    } else {
        None
    }
}

pub fn premultiply_alpha(image: &mut Image) {
    for x in 0..image.width() {
        for y in 0..image.height() {
            let mut color = image
                .get_color_at(x, y)
                .expect("should have existing pixel")
                .to_linear();
            color.red *= color.alpha;
            color.green *= color.alpha;
            color.blue *= color.alpha;

            image
                .set_color_at(x, y, Color::LinearRgba(color))
                .expect("should set color");
        }
    }
}
