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
use std::fs;
use std::path::{Path, PathBuf};

use bevy::prelude::*;
#[cfg(feature = "embed-assets")]
use bevy_embedded_assets::{self, EmbeddedAssetPlugin};

mod constants;
mod id;
mod layer;
mod math;
mod preferences;
mod preview;
mod session;
mod theme;
mod ui;
mod undo;
mod viewport;

fn main() {
    let data_dir: PathBuf = find_data_dir();
    ensure_dir(&data_dir.join(constants::VERSION));

    let mut app = App::new();
    #[cfg(feature = "embed-assets")]
    {
        // This needs to happen before AssetPlugin (DefaultPlugins) is added.
        app.add_plugins(EmbeddedAssetPlugin {
            mode: bevy_embedded_assets::PluginMode::ReplaceDefault,
        });
    }
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: constants::APPLICATION_TITLE.to_owned(),
            ..Default::default()
        }),
        ..Default::default()
    }));
    app.add_plugins((
        layer::LayerPlugin,
        preferences::PreferencesPlugin {
            config_file_path: data_dir.join(constants::VERSION).join("config.toml"),
        },
        preview::PreviewPlugin,
        session::SessionPlugin,
        theme::ThemePlugin,
        ui::UiPlugin,
        undo::UndoPlugin,
        viewport::ViewportPlugin,
    ));
    app.run();
}

// LIB

fn ensure_dir<P: AsRef<Path>>(path: P) {
    if let Err(e) = fs::create_dir_all(&path) {
        error!("Cannot create data directory: {}", e);
    }
}

/// Return the root directory where all the user files for all versions live.
fn find_data_dir() -> PathBuf {
    let project_dirs = directories::ProjectDirs::from("", "", env!("CARGO_PKG_NAME"));
    let data_dir: PathBuf = project_dirs.unwrap().data_local_dir().to_owned();
    info!("Using data dir: '{}'.", data_dir.to_str().unwrap());
    data_dir
}
