use bevy::prelude::*;

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
        .add_plugins((ui::UiPlugin, viewport::ViewportPlugin))
        .run();
}
