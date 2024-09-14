use bevy::prelude::*;

pub struct LayerPlugin;

impl Plugin for LayerPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Layer>();
    }
}

// COMPONENTS

#[derive(Component, Reflect)]
struct Layer;
