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

mod layer;
mod preview;
mod ui;
mod viewport;

const WINDOW_TITLE: &str = "yer - Terrain Generation Toolkit";

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: WINDOW_TITLE.to_owned(),
                ..Default::default()
            }),
            ..Default::default()
        }))
        .add_plugins((
            layer::LayerPlugin,
            preview::PreviewPlugin,
            ui::UiPlugin,
            viewport::ViewportPlugin,
        ))
        .run();
}
