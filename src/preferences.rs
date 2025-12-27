// Copyright © 2024-2025 Atamert Ölçgen.
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
use std::fs::{self, File};
use std::io::{Error as IoError, Read, Write};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::time::Duration;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use toml::{de::Error as TomlDeserializeError, ser::Error as TomlSerializeError};

use crate::constants;

const BACKUP_SUFFIX: &str = ".bak";
const SAVE_DELAY: Duration = Duration::from_millis(1500);

// PLUGIN

pub struct PreferencesPlugin {
    pub config_file_path: PathBuf,
}

impl Plugin for PreferencesPlugin {
    fn build(&self, app: &mut App) {
        match read_or_create(&self.config_file_path) {
            Ok(preferences) => {
                app.insert_resource(preferences);
            }
            Err(e) => {
                panic!("Failed to configure app: {}", e);
            }
        }
    }
}

// RESOURCES

#[derive(Clone, Deserialize, Resource, Reflect, Serialize)]
#[reflect(Resource)]
pub struct Preferences {
    pub max_undo_stack_size: NonZeroUsize,

    #[serde(skip)]
    file_path: Option<PathBuf>,
}

impl Default for Preferences {
    fn default() -> Self {
        Preferences {
            max_undo_stack_size: constants::UNDO_STACK_SIZE_DEFAULT,
            file_path: None,
        }
    }
}

// LIB

#[derive(Debug, Error)]
pub enum PreferencesError {
    #[error("Cannot read file '{0}'.")]
    CannotReadFile(PathBuf, IoError),
    #[error("Cannot write file '{0}'.")]
    CannotWriteFile(PathBuf, IoError),
    #[error("Deserialization failed: {0:?}")]
    DeserializeError(#[from] TomlDeserializeError),
    #[error("Serialization failed: {0:?}")]
    SerializeError(#[from] TomlSerializeError),
}

fn read_or_create<P: AsRef<Path>>(file_path: P) -> Result<Preferences, PreferencesError> {
    let mut preferences = if fs::exists(&file_path)
        .map_err(|e| PreferencesError::CannotReadFile(file_path.as_ref().into(), e))?
    {
        read_preferences_from_file(&file_path)
    } else {
        let preferences = Preferences::default();
        write_preferences(&file_path, &preferences)?;
        Ok(preferences)
    }?;
    preferences.file_path = Some(file_path.as_ref().to_path_buf());
    Ok(preferences)
}

fn read_preferences_from_file<P: AsRef<Path>>(
    file_path: P,
) -> Result<Preferences, PreferencesError> {
    debug!("Reading config file: {:?}.", file_path.as_ref());
    let mut file_contents = String::new();
    File::open(&file_path)
        .map_err(|io_error| PreferencesError::CannotReadFile(file_path.as_ref().into(), io_error))?
        .read_to_string(&mut file_contents)
        .map_err(|io_error| {
            PreferencesError::CannotReadFile(file_path.as_ref().into(), io_error)
        })?;
    Ok(toml::from_str::<Preferences>(&file_contents)?)
}

fn write_preferences<P: AsRef<Path>>(
    file_path: P,
    preferences: &Preferences,
) -> Result<(), PreferencesError> {
    debug!("Writing config file: {:?}.", file_path.as_ref());
    if fs::exists(&file_path).expect("Cannot access config file.") {
        let backup_file_path: PathBuf = file_path.as_ref().with_extension(format!(
            "{}{}",
            file_path.as_ref().extension().unwrap().to_str().unwrap(),
            BACKUP_SUFFIX
        ));
        fs::rename(&file_path, &backup_file_path)
            .map_err(|io_error| PreferencesError::CannotWriteFile(backup_file_path, io_error))?;
    }
    let serialized_preferences = toml::to_string_pretty(preferences)?;
    File::create(&file_path)
        .map_err(|io_error| PreferencesError::CannotWriteFile(file_path.as_ref().into(), io_error))?
        .write_all(serialized_preferences.as_bytes())
        .map_err(|io_error| {
            PreferencesError::CannotWriteFile(file_path.as_ref().into(), io_error)
        })?;
    Ok(())
}
