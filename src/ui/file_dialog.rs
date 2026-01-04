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

use std::ffi::OsStr;
use std::path::PathBuf;

use bevy::prelude::*;
use bevy_egui::egui::{Color32, Context};
use egui_file_dialog;

use crate::theme::{Theme, ThemeColors};
use crate::ui::egui_ext::ToColor32;

static DEFAULT_FILE_NAME: &str = "untitled.yer";
static FILE_FILTER_PROJECT_FILES_NAME: &str = "Project Files";
static SUFFIX: &str = "yer";

// PLUGIN

pub struct UiFileDialogPlugin;

impl Plugin for UiFileDialogPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<LoadFileDialog>()
            .register_type::<SaveFileDialog>();
    }
}

// COMPONENTS

#[derive(Component, Reflect)]
#[reflect(Component)]
pub(super) struct LoadFileDialog {
    #[reflect(ignore)]
    file_dialog: egui_file_dialog::FileDialog,
}

impl LoadFileDialog {
    pub(super) fn show(&mut self, ctx: &Context) -> DialogState {
        match self.file_dialog.update(ctx).state() {
            egui_file_dialog::DialogState::Open => DialogState::Open,
            egui_file_dialog::DialogState::Cancelled => DialogState::Cancelled,
            egui_file_dialog::DialogState::Picked(path) => DialogState::Selected(path.into()),
            _ => unreachable!(),
        }
    }
}

impl FromWorld for LoadFileDialog {
    fn from_world(world: &mut World) -> Self {
        let modal_overlay_color: Color32 = {
            let theme = world.resource::<Theme>();
            let theme_colors = world.resource::<Assets<ThemeColors>>();
            let colors = theme_colors
                .get(&theme.colors)
                .expect("Cannot read theme colors.");
            colors.bg_color.with_alpha(0.85).to_color32()
        };

        let mut file_dialog = egui_file_dialog::FileDialog::new()
            .add_file_filter_extensions(FILE_FILTER_PROJECT_FILES_NAME, vec![SUFFIX])
            .default_file_filter(FILE_FILTER_PROJECT_FILES_NAME)
            .modal_overlay_color(modal_overlay_color)
            .as_modal(true);
        file_dialog.pick_file();

        Self { file_dialog }
    }
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
            egui_file_dialog::DialogState::Picked(path) => {
                DialogState::Selected(sanitize_path(path.into()))
            }
            _ => unreachable!(),
        }
    }
}

impl Default for SaveFileDialog {
    fn default() -> Self {
        let mut file_dialog = egui_file_dialog::FileDialog::new()
            .add_save_extension(FILE_FILTER_PROJECT_FILES_NAME, SUFFIX)
            .default_save_extension(FILE_FILTER_PROJECT_FILES_NAME)
            .default_file_name(DEFAULT_FILE_NAME)
            .as_modal(true);
        file_dialog.save_file();

        Self { file_dialog }
    }
}

// LIB

pub enum DialogState {
    Open,
    Selected(PathBuf),
    Cancelled,
}

fn sanitize_path(path: PathBuf) -> PathBuf {
    let suffix = format!(".{}", SUFFIX);
    if path.extension() != Some(OsStr::new(SUFFIX)) {
        match path.file_name() {
            Some(file_name) => {
                let mut file_name = file_name.to_owned();
                file_name.push(&suffix);
                path.with_file_name(file_name)
            }
            None => panic!("sanitize_path is called with a PathBuf that doesn't have a file name."),
        }
    } else {
        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_path_appends_extension_to_filename_without_extension() {
        assert_eq!(
            PathBuf::from("somefile.yer"),
            sanitize_path(PathBuf::from("somefile"))
        );
    }

    #[test]
    fn sanitize_path_does_not_change_filename_with_correct_extension() {
        assert_eq!(
            PathBuf::from("somefile.yer"),
            sanitize_path(PathBuf::from("somefile.yer"))
        );
    }

    #[test]
    fn sanitize_path_adds_extension_if_the_filename_has_a_different_extension() {
        assert_eq!(
            PathBuf::from("some.file.yer"),
            sanitize_path(PathBuf::from("some.file"))
        );
    }
}
