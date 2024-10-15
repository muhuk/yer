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

use std::path::PathBuf;
use std::sync::Arc;

use bevy::prelude::*;
use bevy_egui::egui::Context;
use egui_file_dialog;

// PLUGIN

pub struct UiFileDialogPlugin;

impl Plugin for UiFileDialogPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<SaveFileDialog>();
    }
}

// LIB

pub enum DialogState {
    Open,
    Selected(PathBuf),
    Cancelled,
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub(super) struct SaveFileDialog {
    #[reflect(ignore)]
    file_dialog: egui_file_dialog::FileDialog,
}

impl SaveFileDialog {
    pub(super) fn show(&mut self, ctx: &Context) -> DialogState {
        match self.file_dialog.update(ctx).state() {
            egui_file_dialog::DialogState::Open => DialogState::Open,
            egui_file_dialog::DialogState::Cancelled => DialogState::Cancelled,
            egui_file_dialog::DialogState::Selected(path) => DialogState::Selected(path),
            _ => unreachable!(),
        }
    }
}

impl Default for SaveFileDialog {
    fn default() -> Self {
        let mut file_dialog = egui_file_dialog::FileDialog::new()
            .add_file_filter(
                "Project files",
                Arc::new(|path| path.extension().unwrap_or_default() == "yer"),
            )
            .as_modal(true);
        file_dialog.save_file();

        Self { file_dialog }
    }
}
