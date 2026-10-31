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

use bevy::prelude::*;
use bevy_egui::egui;

use crate::constants;
use crate::preferences::{Preferences, UpdatePreferences};
use crate::theme::ThemeColors;

use super::egui_ext::{draw_modal_overlay, ToColor32};

// COMPONENTS

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct PreferencesDialog {
    preferences: Preferences,
}

impl PreferencesDialog {
    pub fn show(&mut self, ctx: &mut egui::Context, theme_colors: &ThemeColors) -> DialogState {
        draw_modal_overlay(ctx, theme_colors.modal_overlay_color().to_color32());

        egui::Window::new("Preferences")
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Undo stack size");
                    ui.add(
                        egui::widgets::DragValue::new(&mut self.preferences.max_undo_stack_size)
                            .fixed_decimals(0)
                            .range(constants::UNDO_STACK_SIZE_RANGE),
                    );
                });

                let mut response = DialogState::Open;
                if ui.button("Save and Close").clicked() {
                    response = DialogState::Confirmed;
                }
                if ui.button("Cancel").clicked() {
                    response = DialogState::Cancelled;
                }
                response
            })
            .map(|r| {
                ctx.move_to_top(r.response.layer_id);
                r.inner
            })
            .flatten()
            .unwrap_or(DialogState::Cancelled)
    }

    pub fn to_command(&self) -> UpdatePreferences {
        UpdatePreferences(self.preferences.clone())
    }
}

impl FromWorld for PreferencesDialog {
    fn from_world(world: &mut World) -> Self {
        Self {
            preferences: world.resource::<Preferences>().clone(),
        }
    }
}

// LIB

pub enum DialogState {
    Open,
    Confirmed,
    Cancelled,
}
