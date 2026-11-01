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

use super::egui_ext::ToColor32;
use super::state::UiState;

static DEFAULT_FILE_NAME: &str = "untitled.yer";
static FILE_FILTER_IMAGE_FILES_NAME: &str = "Image Files";
static FILE_FILTER_PROJECT_FILES_NAME: &str = "Project Files";
static IMAGE_FILE_SUFFIXES: [&str; 3] = ["png", "jpg", "jpeg"];
static SUFFIX: &str = "yer";

// PLUGIN

pub struct UiFileDialogPlugin;

impl Plugin for UiFileDialogPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<LoadFileDialog>()
            .register_type::<SaveFileDialog>();
    }
}

// EVENTS

#[derive(Debug, EntityEvent)]
pub struct DialogClosed {
    pub entity: Entity,
    pub result: DialogResult,
}

// COMPONENTS

#[derive(Component, Reflect)]
#[reflect(Component)]
pub(super) struct LoadFileDialog {
    #[reflect(ignore)]
    file_dialog: egui_file_dialog::FileDialog,
}

impl LoadFileDialog {
    pub(super) fn show(&mut self, ctx: &Context) -> Option<DialogResult> {
        match self.file_dialog.update(ctx).state() {
            egui_file_dialog::DialogState::Open => None,
            egui_file_dialog::DialogState::Cancelled => Some(DialogResult::Cancelled),
            egui_file_dialog::DialogState::Picked(path) => {
                Some(DialogResult::PickedFile(path.into()))
            }
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
            colors.modal_overlay_color().to_color32()
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
pub(super) struct LoadImageDialog {
    #[reflect(ignore)]
    file_dialog: egui_file_dialog::FileDialog,
}

impl LoadImageDialog {
    pub(super) fn show(&mut self, ctx: &Context) -> Option<DialogResult> {
        match self.file_dialog.update(ctx).state() {
            egui_file_dialog::DialogState::Open => None,
            egui_file_dialog::DialogState::Cancelled => Some(DialogResult::Cancelled),
            egui_file_dialog::DialogState::Picked(path) => {
                Some(DialogResult::PickedFile(path.into()))
            }
            _ => unreachable!(),
        }
    }
}

impl FromWorld for LoadImageDialog {
    fn from_world(world: &mut World) -> Self {
        let modal_overlay_color: Color32 = {
            let theme = world.resource::<Theme>();
            let theme_colors = world.resource::<Assets<ThemeColors>>();
            let colors = theme_colors
                .get(&theme.colors)
                .expect("Cannot read theme colors.");
            colors.modal_overlay_color().to_color32()
        };

        let mut file_dialog = egui_file_dialog::FileDialog::new()
            .add_file_filter_extensions(FILE_FILTER_IMAGE_FILES_NAME, IMAGE_FILE_SUFFIXES.into())
            .default_file_filter(FILE_FILTER_IMAGE_FILES_NAME)
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
    pub(super) fn show(&mut self, ctx: &Context) -> Option<DialogResult> {
        match self.file_dialog.update(ctx).state() {
            egui_file_dialog::DialogState::Open => None,
            egui_file_dialog::DialogState::Cancelled => Some(DialogResult::Cancelled),
            egui_file_dialog::DialogState::Picked(path) => {
                Some(DialogResult::PickedFile(sanitize_path(path.into())))
            }
            _ => unreachable!(),
        }
    }
}

impl FromWorld for SaveFileDialog {
    fn from_world(world: &mut World) -> Self {
        let modal_overlay_color: Color32 = {
            let theme = world.resource::<Theme>();
            let theme_colors = world.resource::<Assets<ThemeColors>>();
            let colors = theme_colors
                .get(&theme.colors)
                .expect("Cannot read theme colors.");
            colors.modal_overlay_color().to_color32()
        };

        let mut file_dialog = egui_file_dialog::FileDialog::new()
            .add_save_extension(FILE_FILTER_PROJECT_FILES_NAME, SUFFIX)
            .default_save_extension(FILE_FILTER_PROJECT_FILES_NAME)
            .default_file_name(DEFAULT_FILE_NAME)
            .modal_overlay_color(modal_overlay_color)
            .as_modal(true);
        file_dialog.save_file();

        Self { file_dialog }
    }
}

// COMMANDS

pub(super) struct DisplayLoadFileDialog;

impl Command for DisplayLoadFileDialog {
    fn apply(self, world: &mut World) -> () {
        let dialog = LoadFileDialog::from_world(world);
        world.spawn((
            Name::new("Load File Dialog"),
            dialog,
            DespawnOnExit(UiState::ShowingLoadFileDialog),
        ));
        world
            .resource_mut::<NextState<UiState>>()
            .set(UiState::ShowingLoadFileDialog);
    }
}

#[derive(Default)]
pub(super) struct DisplayLoadImageDialog {
    and_then: Option<Box<dyn FnOnce(EntityWorldMut<'_>) + Send + 'static>>,
}

impl DisplayLoadImageDialog {
    pub fn and_then<F: FnOnce(EntityWorldMut<'_>) + Send + 'static>(self, f: F) -> Self {
        debug_assert!(
            self.and_then.is_none(),
            "and_then is called more than once."
        );

        Self {
            and_then: Some(Box::new(f)),
        }
    }
}

impl Command<Result> for DisplayLoadImageDialog {
    fn apply(self, world: &mut World) -> Result {
        let dialog = LoadImageDialog::from_world(world);
        let entity = world
            .spawn((
                Name::new("Load Image Dialog"),
                dialog,
                DespawnOnExit(UiState::ShowingLoadImageDialog),
            ))
            .id();
        world
            .resource_mut::<NextState<UiState>>()
            .set(UiState::ShowingLoadImageDialog);

        if let Some(and_then) = self.and_then {
            world
                .commands()
                .entity(entity)
                .queue(|entity: EntityWorldMut<'_>| {
                    and_then(entity);
                });
        }

        Ok(())
    }
}

pub(super) struct DisplaySaveFileDialog;

impl Command for DisplaySaveFileDialog {
    fn apply(self, world: &mut World) -> () {
        let dialog = SaveFileDialog::from_world(world);
        world.spawn((
            Name::new("Save File Dialog"),
            dialog,
            DespawnOnExit(UiState::ShowingSaveFileDialog),
        ));
        world
            .resource_mut::<NextState<UiState>>()
            .set(UiState::ShowingSaveFileDialog);
    }
}

// LIB

#[derive(Debug)]
pub enum DialogResult {
    Cancelled,
    PickedFile(PathBuf),
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
