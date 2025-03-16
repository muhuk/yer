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

use std::num::NonZeroU8;
use std::sync::{mpsc, Arc, Mutex, TryLockError};

use bevy::ecs::world::Command;
use bevy::prelude::*;
use bevy::render::mesh::{PlaneMeshBuilder, VertexAttributeValues};
use bevy::tasks::{futures_lite::future, AsyncComputeTaskPool, Task, TaskPool};
use bevy::utils::Duration;
use serde::{Deserialize, Serialize};

use crate::layer;
use crate::undo;
use crate::viewport;

pub const MAX_SUBDIVISIONS: NonZeroU8 = unsafe { NonZeroU8::new_unchecked(12) };
pub const MIN_SUBDIVISIONS: NonZeroU8 = unsafe { NonZeroU8::new_unchecked(3) };
const PREVIEW_TIME_BETWEEN_MS: Duration = Duration::from_millis(100);
// Value is the # of vertices in a row or column, index is the subvidision level.
const SUBDIVISIONS_SQRT_VERTS_TABLE: [u32; (MAX_SUBDIVISIONS.get() + 1) as usize] = {
    let mut ns = [0; MAX_SUBDIVISIONS.get() as usize + 1usize];
    let mut idx: u8 = 0;
    while idx <= MAX_SUBDIVISIONS.get() {
        // (2**n + 1) ** 2
        ns[idx as usize] = 2u32.pow(idx as u32) + 1;
        idx += 1;
    }
    ns
};
// Value is the # of vertices, index is the subdivision level.
const SUBDIVISIONS_VERTS_TABLE: [u32; (MAX_SUBDIVISIONS.get() + 1) as usize] = {
    let mut ns = [0; MAX_SUBDIVISIONS.get() as usize + 1usize];
    let mut idx: u8 = 0;
    while idx <= MAX_SUBDIVISIONS.get() {
        ns[idx as usize] = SUBDIVISIONS_SQRT_VERTS_TABLE[idx as usize].pow(2);
        idx += 1;
    }
    ns
};

type Layers = Arc<[Box<dyn layer::Sample2D>]>;

pub struct PreviewPlugin;

impl Plugin for PreviewPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<ActivePreview>()
            // See: https://github.com/jakobhellermann/bevy-inspector-egui/issues/217
            .register_type::<NonZeroU8>()
            .register_type::<Preview>()
            .register_type::<PreviewGrid2D>()
            .register_type::<PreviewRegion>();
        app.add_event::<UpdatePreviewRegion>();
        app.init_resource::<Preview>();
        app.add_systems(
            Update,
            (
                manage_preview_system,
                update_preview_region_system.run_if(on_event::<UpdatePreviewRegion>),
            ),
        );
    }
}

// EVENTS

#[derive(Debug, Event)]
pub enum UpdatePreviewRegion {
    SetCenter(Entity, Vec2),
    SetSize(Entity, f32),
    SetSubdivisions(Entity, NonZeroU8),
}

// RESOURCES

#[derive(Debug, Default, Reflect, Resource)]
#[reflect(Resource)]
struct Preview {
    last_project_changed: Duration,
    last_preview_initiated: Duration,
    last_preview_updated: Option<Duration>,
    #[reflect(ignore)]
    task: Option<ComputePreview>,
}

impl Preview {
    fn start_new_task(
        &mut self,
        task_pool: &TaskPool,
        preview_entity: Entity,
        preview_region: PreviewRegion,
        layers: Layers,
    ) {
        let task = ComputePreview::new(task_pool, preview_entity, preview_region, layers);
        if let Some(previous_task) = self.task.replace(task) {
            // TODO: We might want to use the result of previous task while
            //       the new task is running.
            drop(previous_task.task.cancel());
        }
    }
}

// BUNDLES

#[derive(Bundle, Deserialize, Serialize)]
pub struct PreviewBundle {
    name: Name,
    // ActivePreview should only be on the active preview but
    // since we're having only one preview region now, this
    // should be okay.
    active_preview: ActivePreview,
    preview_region: PreviewRegion,
}

impl PreviewBundle {
    pub fn extract_all(world: &mut World) -> Vec<Self> {
        world
            .query::<(&Name, &ActivePreview, &PreviewRegion)>()
            .iter(world)
            .map(|(name, active_preview, preview_region)| Self {
                name: name.to_owned(),
                active_preview: *active_preview,
                preview_region: preview_region.clone(),
            })
            .collect()
    }
}

// COMPONENTS

/// Marker trait for active preview.
#[derive(Clone, Component, Copy, Debug, Deserialize, Reflect, Serialize)]
#[reflect(Component)]
struct ActivePreview;

#[derive(Clone, Component, Debug, PartialEq, Reflect)]
#[reflect(Component)]
struct PreviewGrid2D {
    bounds: Rect,
    samples: Vec<(Vec2, f32)>,
    subdivisions: u8,
}

impl PreviewGrid2D {
    fn new(samples: Vec<(Vec2, f32)>) -> Self {
        let subdivisions = {
            let samples_count: u32 = u32::try_from(samples.len()).unwrap();
            SUBDIVISIONS_VERTS_TABLE
                .iter()
                .enumerate()
                .find(|(_, w)| **w == samples_count)
                .map(|(idx, _)| idx.try_into().unwrap())
                .expect(
                    format!(
                        "PreviewGrid2D has {} samples.  It corresponds to an invalid subdivision",
                        samples_count
                    )
                    .as_str(),
                )
        };
        let bounds: Rect = {
            let (mut x_min, mut y_min, mut x_max, mut y_max) = (
                f32::INFINITY,
                f32::INFINITY,
                f32::NEG_INFINITY,
                f32::NEG_INFINITY,
            );
            for (x, y) in samples.iter().map(|(p, _)| (p.x, p.y)) {
                x_min = x_min.min(x);
                y_min = y_min.min(y);
                x_max = x_max.max(x);
                y_max = y_max.max(y);
            }
            Rect::new(x_min, y_min, x_max, y_max)
        };
        Self {
            bounds,
            samples,
            subdivisions,
        }
    }

    fn build_mesh(&self) -> Mesh {
        let mut mesh = PlaneMeshBuilder::new(Dir3::Z, self.bounds.size())
            .subdivisions(2u32.pow(self.subdivisions.into()) - 1)
            .build();
        if let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
        {
            for (idx, p) in positions.iter_mut().enumerate() {
                // Preview mesh is Z-up.
                p[0] = self.samples[idx].0.x;
                p[1] = self.samples[idx].0.y;
                p[2] = self.samples[idx].1;
            }
        } else {
            panic!("Cannot build preview mesh.");
        }
        mesh
    }
}

#[derive(Component, Clone, Debug, Deserialize, Reflect, Serialize)]
#[reflect(Component)]
pub struct PreviewRegion {
    center: Vec2,
    size: f32,
    subdivisions: NonZeroU8,
}

impl PreviewRegion {
    fn new(center: Vec2, size: f32, subdivisions: NonZeroU8) -> Self {
        assert!(
            subdivisions >= MIN_SUBDIVISIONS,
            "Subdivisions cannot be less than {}",
            MIN_SUBDIVISIONS
        );
        assert!(
            subdivisions <= MAX_SUBDIVISIONS,
            "Subdivisions cannot be greater than {}",
            MAX_SUBDIVISIONS
        );
        Self {
            center,
            size,
            subdivisions,
        }
    }

    pub fn center(&self) -> Vec2 {
        self.center
    }

    pub fn size(&self) -> f32 {
        self.size
    }

    pub fn subdivisions(&self) -> NonZeroU8 {
        self.subdivisions
    }
}

impl Default for PreviewRegion {
    fn default() -> Self {
        Self::new(Vec2::ZERO, 100.0, MIN_SUBDIVISIONS)
    }
}

// COMMANDS

struct CalculatePreview;

impl Command for CalculatePreview {
    fn apply(self, world: &mut World) {
        debug!("Calculating preview...");
        let (entity, preview_region) = world
            .query_filtered::<(Entity, &PreviewRegion), With<ActivePreview>>()
            .iter(world)
            .map(|(e, p)| (e, p.clone()))
            .next()
            .unwrap();
        // Currently we have only HeightMap's that implement Sample2D.
        let layers: Vec<Box<dyn layer::Sample2D>> = world
            .query::<(&layer::Layer, &layer::HeightMap)>()
            .iter(world)
            .sort::<&layer::Layer>()
            .map(|(_, height_map)| Box::new(height_map.clone()) as Box<dyn layer::Sample2D>)
            .collect();
        let task_pool = AsyncComputeTaskPool::get();
        world.resource_mut::<Preview>().start_new_task(
            task_pool,
            entity,
            preview_region,
            layers.into(),
        );
    }
}

struct UpdatePreviewMesh(Entity);

impl Command for UpdatePreviewMesh {
    fn apply(self, world: &mut World) {
        debug!(
            "Replacing mesh. Subdivisions is {}",
            world
                .query_filtered::<&PreviewGrid2D, With<ActivePreview>>()
                .get(world, self.0)
                .map(|preview_data| preview_data.subdivisions)
                .unwrap()
        );
        let mesh: Mesh = world
            // We want to update the mesh only if the preview region is still
            // the active region, hence With<ActivePreview>.
            .query_filtered::<&PreviewGrid2D, With<ActivePreview>>()
            .get(world, self.0)
            .map(|preview_data| preview_data.build_mesh())
            .unwrap();

        let preview_mesh_entity: Entity = world
            .query_filtered::<Entity, With<viewport::PreviewMesh>>()
            .single(world);
        let mesh_handle: Handle<Mesh> = world.resource_mut::<Assets<Mesh>>().add(mesh);
        world
            .commands()
            .entity(preview_mesh_entity)
            .insert(Mesh3d(mesh_handle));
    }
}

// SYSTEMS

fn manage_preview_system(
    mut commands: Commands,
    mut undo_events: EventReader<undo::UndoEvent>,
    mut preview_resource: ResMut<Preview>,
    time: Res<Time>,
    mut update_preview_region_events: EventReader<UpdatePreviewRegion>,
) {
    let now: Duration = time.elapsed();

    // Update project's last change time.
    {
        // FIXME: Setting last_project_changed on all PreviewRegion
        //        updates works for now, but it will trigger unnecessary
        //        preview renders once we have multiple regions.
        if !undo_events.is_empty() || !update_preview_region_events.is_empty() {
            undo_events.clear();
            update_preview_region_events.clear();
            preview_resource.last_project_changed = now;
        }
    }

    // Trigger a new preview if necessary.
    {
        let project_has_changed: bool = match preview_resource.last_preview_updated {
            Some(last_preview_updated) => {
                last_preview_updated < preview_resource.last_project_changed
            }
            None => true,
        };

        // If the difference between changed and initiated is small: don't
        // trigger a new preview and don't cancel currently running
        // calculation either.  This is okay because worst case we will
        // trigger a new calculation when the currently running preview is
        // finished.
        let ready_to_trigger: bool =
            now - preview_resource.last_preview_initiated > PREVIEW_TIME_BETWEEN_MS;
        if project_has_changed && ready_to_trigger {
            preview_resource.last_preview_initiated = now;
            commands.queue(CalculatePreview);
        }
    }

    // Update preview region if the task is finished.
    {
        if let Some(ref mut task) = preview_resource.task {
            match task.poll() {
                ComputePreviewResult::Finished => {
                    // When the task is finished we can drop it.
                    preview_resource.task = None;
                }
                ComputePreviewResult::Computing => (),
                ComputePreviewResult::Result(entity, preview_grid) => {
                    preview_resource.last_preview_updated = Some(now);
                    commands.entity(entity).insert(preview_grid);
                    commands.queue(UpdatePreviewMesh(entity));
                }
            }
        }
    }
}

fn update_preview_region_system(
    mut preview_regions: Query<&mut PreviewRegion>,
    mut update_preview_region_events: EventReader<UpdatePreviewRegion>,
) {
    for event in update_preview_region_events.read() {
        match event {
            UpdatePreviewRegion::SetCenter(entity, new_center) => {
                if let Ok(mut preview_region) = preview_regions.get_mut(*entity) {
                    preview_region.center = *new_center;
                }
            }
            UpdatePreviewRegion::SetSize(entity, new_size) => {
                if let Ok(mut preview_region) = preview_regions.get_mut(*entity) {
                    preview_region.size = *new_size;
                }
            }
            UpdatePreviewRegion::SetSubdivisions(entity, new_subdivisions) => {
                if let Ok(mut preview_region) = preview_regions.get_mut(*entity) {
                    preview_region.subdivisions = *new_subdivisions;
                }
            }
        }
    }
}

// LIB

#[derive(Debug)]
struct ComputePreview {
    target_entity: Entity,
    task: Task<()>,
    receiver: Mutex<mpsc::Receiver<PreviewGrid2D>>,
}

impl ComputePreview {
    fn new(
        task_pool: &TaskPool,
        target_entity: Entity,
        preview_region: PreviewRegion,
        layers: Layers,
    ) -> Self {
        let (sender, receiver) = mpsc::channel::<PreviewGrid2D>();
        let task: Task<()> = task_pool.spawn(Self::run(sender, preview_region, layers));
        Self {
            target_entity,
            task,
            receiver: Mutex::new(receiver),
        }
    }

    fn poll(&mut self) -> ComputePreviewResult {
        let result = match self
            .receiver
            .try_lock()
            .map(|receiver| receiver.try_iter().last())
        {
            Ok(Some(preview_grid)) => {
                ComputePreviewResult::Result(self.target_entity, preview_grid)
            }
            Err(TryLockError::WouldBlock) => {
                error!("ComputePreview.receiver lock cannot be acquired.");
                ComputePreviewResult::Computing
            }
            Err(TryLockError::Poisoned(poison_error)) => {
                error_once!("ComputePreview.receiver lock is poisoned: {}", poison_error);
                ComputePreviewResult::Computing
            }
            _ => {
                if self.task.is_finished() {
                    ComputePreviewResult::Finished
                } else {
                    ComputePreviewResult::Computing
                }
            }
        };
        result
    }

    async fn run(
        sender: mpsc::Sender<PreviewGrid2D>,
        preview_region: PreviewRegion,
        layers: Layers,
    ) {
        let mut subdivisions = MIN_SUBDIVISIONS;
        let mut preview: Option<PreviewGrid2D> = None;
        while subdivisions <= preview_region.subdivisions {
            if let Some(preview) = preview.as_ref() {
                debug!(
                    "Reusing {} samples from previous preview.",
                    preview.samples.len()
                );
            }
            preview =
                Some(sample_layers(subdivisions, &preview_region, &layers, preview.as_ref()).await);
            sender.send(preview.clone().unwrap()).unwrap();
            subdivisions = subdivisions.checked_add(1).unwrap();
            future::yield_now().await;
        }
    }
}

#[derive(Debug, PartialEq)]
enum ComputePreviewResult {
    /// This is returned **after** the last result is polled (read).
    Finished,
    /// This is returned when the task is not finished and also there are no
    /// results available to read.
    Computing,
    /// A result produced by task.
    ///
    /// The `PreviewGrid2D` is to be inserted to the `Entity`, when world
    /// access is available.
    Result(Entity, PreviewGrid2D),
}

pub fn create_default_preview_region(world: &mut World) {
    world.spawn(PreviewBundle {
        name: Name::new("Default Preview"),
        active_preview: ActivePreview,
        preview_region: PreviewRegion::default(),
    });
}

#[inline]
fn even(x: u32) -> bool {
    x % 2 == 0
}

async fn sample_layers(
    subdivisions: NonZeroU8,
    preview_region: &PreviewRegion,
    layers: &Layers,
    previous_preview: Option<&PreviewGrid2D>,
) -> PreviewGrid2D {
    // TODO: Reuse existing samples.
    assert!(subdivisions <= preview_region.subdivisions);
    assert!(subdivisions >= MIN_SUBDIVISIONS);
    assert!(previous_preview
        .map(|p| p.subdivisions + 1 == subdivisions.get())
        .unwrap_or(true));

    // Number of vertices on one axis.
    let k: u32 = 2u32.pow(subdivisions.get().into()) + 1;
    let start: Vec2 = {
        let hs: f32 = preview_region.size / 2.0;
        // Y is inverted.
        preview_region.center + Vec2::new(-hs, hs)
    };
    let gap: Vec2 = {
        let g: f32 = preview_region.size / (k - 1) as f32;
        Vec2::new(g, -g)
    };

    let mut samples: Vec<(Vec2, f32)> = vec![];
    for y in 0..k {
        for x in 0..k {
            // Y is inverted.
            let p = start + Vec2::new(x as f32, y as f32) * gap;
            if previous_preview.is_some() && even(y) && even(x) {
                if let Some(PreviewGrid2D {
                    samples: previous_samples,
                    subdivisions,
                    ..
                }) = previous_preview
                {
                    let idx: usize = ((y / 2)
                        * SUBDIVISIONS_SQRT_VERTS_TABLE[*subdivisions as usize] as u32
                        + (x / 2)) as usize;
                    let h: f32 = previous_samples[idx].1;
                    samples.push((p, h));
                } else {
                    unreachable!()
                }
            } else {
                let mut h: f32 = 0.0;
                for layer in layers.iter() {
                    h = layer.sample(p, h);
                }
                samples.push((p, h));
                // Yield control at every calculated sample.
                future::yield_now().await;
            }
        }
    }
    PreviewGrid2D::new(samples)
}

#[cfg(test)]
mod tests {
    use std::thread;
    use std::time::Duration;

    use bevy::tasks::{block_on, TaskPool};

    use super::*;
    use crate::layer;

    #[test]
    fn compute_preview_returns_a_result_and_gets_finished() {
        let target_entity = Entity::PLACEHOLDER;
        let preview_region = PreviewRegion::default();
        // Note the subdivisions is set to minimum.
        assert_eq!(preview_region.subdivisions(), MIN_SUBDIVISIONS);
        let height = 10.0f32;
        let layers: Layers =
            vec![Box::new(layer::HeightMap::Constant(height)) as Box<dyn layer::Sample2D>].into();
        let task_pool = AsyncComputeTaskPool::get_or_init(|| TaskPool::new());
        let mut compute_preview =
            ComputePreview::new(task_pool, target_entity, preview_region, layers);
        thread::sleep(Duration::from_millis(50));
        let first_result = compute_preview.poll();
        assert!(matches!(first_result, ComputePreviewResult::Result(..)));
        let second_result = compute_preview.poll();
        assert_eq!(second_result, ComputePreviewResult::Finished);
    }

    #[test]
    fn compute_preview_returns_the_last_result_in_channel_and_gets_finished() {
        let subdivisions = unsafe { NonZeroU8::new_unchecked(MIN_SUBDIVISIONS.get() + 2) };
        assert!(subdivisions < MAX_SUBDIVISIONS);
        let target_entity = Entity::PLACEHOLDER;
        let preview_region = PreviewRegion::new(Vec2::ZERO, 1000.0, subdivisions);

        let height = 10.0f32;
        let layers: Layers =
            vec![Box::new(layer::HeightMap::Constant(height)) as Box<dyn layer::Sample2D>].into();
        let task_pool = AsyncComputeTaskPool::get_or_init(|| TaskPool::new());
        let mut compute_preview =
            ComputePreview::new(task_pool, target_entity, preview_region, layers);
        thread::sleep(Duration::from_millis(50));
        let results = vec![
            compute_preview.poll(),
            compute_preview.poll(),
            compute_preview.poll(),
        ];
        assert!(matches!(results[0], ComputePreviewResult::Result(..)));
        if let ComputePreviewResult::Result(_, ref preview_data) = results[0] {
            assert_eq!(preview_data.subdivisions, MIN_SUBDIVISIONS.get() + 2);
        } else {
            unreachable!()
        }
        assert_eq!(results[1], ComputePreviewResult::Finished);
        assert_eq!(results[2], ComputePreviewResult::Finished);
    }

    #[test]
    fn preview_grid_subdivisions_are_calculated_correctly() {
        let p = (Vec2::ZERO, 0.0f32);
        let cases: Vec<(usize, u8)> = vec![
            (4, 0),
            (9, 1),
            (25, 2),
            (81, 3),
            (289, 4),
            (1089, 5),
            (4225, 6),
            (16785409, 12),
        ];
        for (vertex_count, subdivisions) in cases.iter() {
            assert_eq!(
                PreviewGrid2D::new([p].repeat(*vertex_count)).subdivisions,
                *subdivisions
            );
        }
    }

    #[test]
    fn sample_layers_can_calculate_first_pass() {
        let subdivisions = MIN_SUBDIVISIONS;
        let preview_region = PreviewRegion::default();
        let layers: Layers = Arc::new([Box::new(layer::HeightMap::Constant(0.0))]);
        assert_eq!(
            block_on(sample_layers(subdivisions, &preview_region, &layers, None)).subdivisions,
            subdivisions.get(),
        );
    }

    #[test]
    fn sample_layers_reuse_previous_previews_samples() {
        const PREVIOUS_HEIGHT: f32 = 1.0;
        const HEIGHT: f32 = 0.0;

        let previous_subdivisions = MIN_SUBDIVISIONS;
        let subdivisions = previous_subdivisions.checked_add(1).unwrap();
        let preview_region = PreviewRegion {
            subdivisions,
            ..default()
        };
        // Changing the level on previous layers to be able check reuise.
        let previous_layers: Layers =
            Arc::new([Box::new(layer::HeightMap::Constant(PREVIOUS_HEIGHT))]);
        let layers: Layers = Arc::new([Box::new(layer::HeightMap::Constant(HEIGHT))]);
        let previous_preview = block_on(sample_layers(
            previous_subdivisions,
            &preview_region,
            &previous_layers,
            None,
        ));
        let preview = block_on(sample_layers(
            subdivisions,
            &preview_region,
            &layers,
            Some(&previous_preview),
        ));
        assert_eq!(preview.samples[0].1, PREVIOUS_HEIGHT);
        assert_eq!(
            preview.samples[preview.samples.len() - 1].1,
            PREVIOUS_HEIGHT
        );
        assert_eq!(
            preview
                .samples
                .iter()
                .filter(|(_, h)| (h - PREVIOUS_HEIGHT).abs() <= f32::EPSILON)
                .count(),
            previous_preview.samples.len()
        );
    }
}
