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

use crate::theme::{Theme, ThemeColors};
use crate::ui::egui_ext::{draw_modal_overlay, ToColor32};
use crate::ui::state::UiState;

// COMPONENTS

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct ErrorDialog {
    message: String,
    overlay_color: Color,
}

impl ErrorDialog {
    pub fn show(&mut self, ctx: &mut egui::Context) -> ErrorDialogState {
        draw_modal_overlay(ctx, self.overlay_color.to_color32());

        egui::Window::new("Error")
            .show(ctx, |ui| {
                ui.label(self.message.as_str());

                if ui.button("Close").clicked() {
                    ErrorDialogState::Closed
                } else {
                    ErrorDialogState::Open
                }
            })
            .map(|r| {
                ctx.move_to_top(r.response.layer_id);
                r.inner
            })
            .flatten()
            .unwrap()
    }
}

// COMMANDS

pub struct DisplayErrorDialog {
    message: String,
}

impl DisplayErrorDialog {
    pub fn new(message: String) -> Self {
        Self { message }
    }
}

impl Command<Result> for DisplayErrorDialog {
    fn apply(self, world: &mut World) -> Result {
        let overlay_color = world
            .resource::<Assets<ThemeColors>>()
            .get(&world.resource::<Theme>().colors)
            .ok_or("Cannot read theme colors.")?
            .modal_overlay_color();

        let dialog = ErrorDialog {
            message: self.message,
            overlay_color,
        };

        world.spawn((
            Name::new("Error Dialog"),
            dialog,
            DespawnOnExit(UiState::ShowingErrorDialog),
        ));
        world
            .resource_mut::<NextState<UiState>>()
            .set(UiState::ShowingErrorDialog);

        Ok(())
    }
}

// LIB

pub enum ErrorDialogState {
    Open,
    Closed,
}
