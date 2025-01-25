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
use bevy_egui::egui;

use crate::ui::theme::Theme;
use crate::undo;

// LIB

pub fn draw_toolbar(
    commands: &mut Commands,
    ui: &mut egui::Ui,
    theme: &Res<Theme>,
    undo_stack: &Res<undo::UndoStack>,
) {
    ui.horizontal(|ui| {
        // TODO: Add tooltips.
        if ui
            .add_enabled_ui(undo_stack.can_undo(), |ui| {
                theme.draw_toolbar_button(ui, UVec2::new(0, 1))
            })
            .inner
            .clicked()
        {
            commands.queue(undo::UndoAction)
        }
        if ui
            .add_enabled_ui(undo_stack.can_redo(), |ui| {
                theme.draw_toolbar_button(ui, UVec2::new(1, 1))
            })
            .inner
            .clicked()
        {
            commands.queue(undo::RedoAction)
        }
    });
}
