use bevy::prelude::*;

pub struct PreviewPlugin;

impl Plugin for PreviewPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<PreviewMesh>();
    }
}

#[derive(Component, Debug, Reflect)]
struct PreviewMesh;
