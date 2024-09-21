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
