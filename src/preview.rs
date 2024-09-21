use bevy::prelude::*;
use bevy::utils::Duration;

use crate::layer::LayerChange;

const PREVIEW_TIME_BETWEEN_MS: Duration = Duration::from_millis(100);

pub struct PreviewPlugin;

impl Plugin for PreviewPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Preview>()
            .register_type::<PreviewMesh>();
        app.init_resource::<Preview>();
        app.add_systems(Update, trigger_preview);
    }
}

// RESOURCES

// when the calculation is finished; if changed > completed, trigger a new calculation.
#[derive(Debug, Default, Reflect, Resource)]
struct Preview {
    last_project_changed: Duration,
    last_preview_initiated: Duration,
    last_preview_completed: Duration,
}

// COMPONENTS

/// Marker trait for preview meshes.
#[derive(Component, Debug, Reflect)]
struct PreviewMesh;

// SYSTEMS

fn trigger_preview(
    layer_change_events: EventReader<LayerChange>,
    mut preview_resource: ResMut<Preview>,
    time: Res<Time>,
) {
    if !layer_change_events.is_empty() {
        let now: Duration = time.elapsed();
        preview_resource.last_project_changed = now;
        // If the difference between changed and initiated is small: don't
        // trigger a new preview and don't cancel currently running
        // calculation either.  This is okay because worst case we will
        // trigger a new calculation when the currently running preview is
        // finished.
        if now - preview_resource.last_preview_initiated > PREVIEW_TIME_BETWEEN_MS {
            info!("Triggering preview");
            // FIXME: Cancel currently running calculation.
            // FIXME: Start new calculation.
            preview_resource.last_preview_initiated = now;
        }
    }
}
