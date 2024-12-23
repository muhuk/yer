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
use bevy::utils::Duration;

use crate::layer::LayerChange;

const PREVIEW_TIME_BETWEEN_MS: Duration = Duration::from_millis(100);

pub struct PreviewPlugin;

impl Plugin for PreviewPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Preview>()
            .register_type::<PreviewMesh>();
        app.init_resource::<Preview>();
        app.add_systems(Update, trigger_preview_system);
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

fn trigger_preview_system(
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
