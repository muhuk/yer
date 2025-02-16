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
#[cfg(feature = "embed-assets")]
use bevy_embedded_assets::{self, EmbeddedAssetPlugin};

mod constants;
mod layer;
mod preview;
mod session;
mod ui;
mod undo;
mod viewport;

fn main() {
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
        preview::PreviewPlugin,
        session::SessionPlugin,
        ui::UiPlugin,
        undo::UndoPlugin,
        viewport::ViewportPlugin,
    ));
    app.run();
}
