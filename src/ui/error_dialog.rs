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

use crate::theme::ThemeColors;

use super::egui_ext::{draw_modal_overlay, ToColor32};
use super::state::UiState;

// COMPONENTS

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct ErrorDialog {
    message: String,
}

impl ErrorDialog {
    pub(super) fn show(
        &mut self,
        ctx: &mut egui::Context,
        theme_colors: &ThemeColors,
    ) -> DialogState {
        draw_modal_overlay(ctx, theme_colors.modal_overlay_color().to_color32());

        egui::Window::new("Error")
            .show(ctx, |ui| {
                ui.label(self.message.as_str());

                if ui.button("Close").clicked() {
                    DialogState::Closed
                } else {
                    DialogState::Open
                }
            })
            .map(|r| {
                ctx.move_to_top(r.response.layer_id);
                r.inner
            })
            .flatten()
            .unwrap()
    }

    /// Shows the `ErrorDialog` to user.
    ///
    /// Use this to show the dialog instead of [ErrorDialog::show].
    /// `show` is named as such to align with egui API.
    ///
    /// This also change `UiState` to [UiState::ShowingErrorDialog].
    pub(super) fn spawn(world: &mut World, message: String) -> Entity {
        let dialog = Self { message };
        let entity = world
            .spawn((
                Name::new("Error Dialog"),
                dialog,
                DespawnOnExit(UiState::ShowingErrorDialog),
            ))
            .id();
        world
            .resource_mut::<NextState<UiState>>()
            .set(UiState::ShowingErrorDialog);
        entity
    }
}

// LIB

pub enum DialogState {
    Open,
    Closed,
}
