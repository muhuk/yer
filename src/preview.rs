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

use std::num::NonZeroU8;
use std::sync::{mpsc, Mutex};

use bevy::ecs::world::Command;
use bevy::prelude::*;
use bevy::render::mesh::{PlaneMeshBuilder, VertexAttributeValues};
use bevy::tasks::{AsyncComputeTaskPool, Task};
use bevy::utils::Duration;
use serde::{Deserialize, Serialize};

use crate::layer::{self, Sample2D};
use crate::undo;
use crate::viewport;

pub const MAX_SUBDIVISIONS: NonZeroU8 = unsafe { NonZeroU8::new_unchecked(12) };
pub const MIN_SUBDIVISIONS: NonZeroU8 = unsafe { NonZeroU8::new_unchecked(3) };
const PREVIEW_TIME_BETWEEN_MS: Duration = Duration::from_millis(100);
// Value is the # of vertices, index is the subdivision level.
const SUBDIVISIONS_SQRT_TABLE: [u32; (MAX_SUBDIVISIONS.get() + 1) as usize] = {
    let mut ns = [0; MAX_SUBDIVISIONS.get() as usize + 1usize];
    let mut idx: u8 = 0;
    while idx <= MAX_SUBDIVISIONS.get() {
        // (2**n + 1) ** 2
        ns[idx as usize] = (2u32.pow(idx as u32) + 1).pow(2);
        idx += 1;
    }
    ns
};

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
    last_preview_completed: Option<Duration>,
    #[reflect(ignore)]
    task: Option<ComputePreview>,
}

impl Preview {
    fn start_new_task(
        &mut self,
        preview_entity: Entity,
        preview_region: PreviewRegion,
        layers: Vec<layer::HeightMap>,
    ) {
        let task = ComputePreview::new(preview_entity, preview_region, layers);
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
    #[serde(with = "name_serde")]
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
            SUBDIVISIONS_SQRT_TABLE
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
        let layers: Vec<layer::HeightMap> = world
            .query::<(&layer::Layer, &layer::HeightMap)>()
            .iter(world)
            .sort::<&layer::Layer>()
            .map(|t| t.1)
            .cloned()
            .collect();
        world
            .resource_mut::<Preview>()
            .start_new_task(entity, preview_region, layers);
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
        let project_has_changed: bool = match preview_resource.last_preview_completed {
            Some(last_preview_completed) => {
                last_preview_completed < preview_resource.last_project_changed
            }
            None => true,
        };

        // If the difference between changed and initiated is small: don't
        // trigger a new preview and don't cancel currently running
        // calculation either.  This is okay because worst case we will
        // trigger a new calculation when the currently running preview is
        // finished.
        let ready_to_trigger: bool = now - preview_resource.last_preview_initiated
            > PREVIEW_TIME_BETWEEN_MS
            && preview_resource.task.is_none();
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
                    // When the task is finished we can set it to None
                    // on Preview.
                    preview_resource.last_preview_completed = Some(now);
                    preview_resource.task = None;
                }
                ComputePreviewResult::Computing => (),
                ComputePreviewResult::Result(entity, preview_grid) => {
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
        target_entity: Entity,
        preview_region: PreviewRegion,
        layers: Vec<layer::HeightMap>,
    ) -> Self {
        let (sender, receiver) = mpsc::channel::<PreviewGrid2D>();
        let task: Task<()> = AsyncComputeTaskPool::get().spawn(async move {
            (MIN_SUBDIVISIONS.get()..=preview_region.subdivisions.get())
                .map(|s| unsafe { NonZeroU8::new_unchecked(s) })
                .for_each(|subdivisions| {
                    sender
                        .send(sample_layers(subdivisions, &preview_region, &layers))
                        .unwrap();
                });
        });
        Self {
            target_entity,
            task,
            receiver: Mutex::new(receiver),
        }
    }

    fn poll(&mut self) -> ComputePreviewResult {
        let result = match self.receiver.try_lock().map(|receiver| receiver.try_recv()) {
            Ok(Ok(preview_grid)) => ComputePreviewResult::Result(self.target_entity, preview_grid),
            Ok(Err(mpsc::TryRecvError::Disconnected)) => ComputePreviewResult::Finished,
            _ => {
                if self.task.is_finished() {
                    ComputePreviewResult::Finished
                } else {
                    ComputePreviewResult::Computing
                }
            }
        };

        if self.receiver.is_poisoned() {
            error_once!("ComputePreview.receiver lock is poisoned.");
        }
        result
    }
}

#[derive(Debug, PartialEq)]
enum ComputePreviewResult {
    Finished,
    Computing,
    Result(Entity, PreviewGrid2D),
}

pub fn create_default_preview_region(world: &mut World) {
    world.spawn(PreviewBundle {
        name: Name::new("Default Preview"),
        active_preview: ActivePreview,
        preview_region: PreviewRegion::default(),
    });
}

fn sample_layers<T>(
    subdivisions: NonZeroU8,
    preview_region: &PreviewRegion,
    layers: &Vec<T>,
) -> PreviewGrid2D
where
    T: Sample2D,
{
    // TODO: Reuse existing samples.
    assert!(subdivisions <= preview_region.subdivisions);
    assert!(subdivisions >= MIN_SUBDIVISIONS);

    // Number of vertices on one axis.
    let k: i32 = 2i32.pow(subdivisions.get().into()) + 1;
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
            let mut h: f32 = 0.0;
            for layer in layers.iter() {
                h = layer.sample(p, h);
            }
            samples.push((p, h));
        }
    }
    PreviewGrid2D::new(samples)
}

/// Serializer and deserializer for bevy::core::Name.
mod name_serde {
    use bevy::core::Name;
    use serde::{de::Error, de::Visitor, Deserializer, Serializer};
    use std::fmt;

    struct NameVisitor;

    impl<'de> Visitor<'de> for NameVisitor {
        type Value = Name;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            write!(formatter, "a string")
        }

        fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
        where
            E: Error,
        {
            Ok(Name::from(s))
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Name, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_string(NameVisitor)
    }

    pub fn serialize<S>(name: &Name, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(name.as_str())
    }
}

#[cfg(test)]
mod tests {
    use std::thread;
    use std::time::Duration;

    use bevy::tasks::TaskPool;

    use super::*;
    use crate::layer;

    #[test]
    fn compute_preview_returns_a_result_and_gets_finished() {
        // Initialize task pool before creating a ComputePreview.
        AsyncComputeTaskPool::get_or_init(|| TaskPool::new());

        let target_entity = Entity::PLACEHOLDER;
        let preview_region = PreviewRegion::default();
        // Note the subdivisions is set to minimum.
        assert_eq!(preview_region.subdivisions(), MIN_SUBDIVISIONS);
        let height = 10.0f32;
        let layers = vec![layer::HeightMap::Constant(height)];
        let mut compute_preview = ComputePreview::new(target_entity, preview_region, layers);
        thread::sleep(Duration::from_millis(50));
        let first_result = compute_preview.poll();
        assert!(matches!(first_result, ComputePreviewResult::Result(..)));
        let second_result = compute_preview.poll();
        assert_eq!(second_result, ComputePreviewResult::Finished);
    }

    #[test]
    fn compute_preview_returns_multiple_results_and_gets_finished() {
        // Initialize task pool before creating a ComputePreview.
        AsyncComputeTaskPool::get_or_init(|| TaskPool::new());

        let subdivisions = unsafe { NonZeroU8::new_unchecked(MIN_SUBDIVISIONS.get() + 2) };
        assert!(subdivisions < MAX_SUBDIVISIONS);
        let target_entity = Entity::PLACEHOLDER;
        let preview_region = PreviewRegion::new(Vec2::ZERO, 1000.0, subdivisions);

        let height = 10.0f32;
        let layers = vec![layer::HeightMap::Constant(height)];
        let mut compute_preview = ComputePreview::new(target_entity, preview_region, layers);
        thread::sleep(Duration::from_millis(50));
        let results = vec![
            compute_preview.poll(),
            compute_preview.poll(),
            compute_preview.poll(),
            compute_preview.poll(),
            compute_preview.poll(),
        ];
        assert!(matches!(results[0], ComputePreviewResult::Result(..)));
        if let ComputePreviewResult::Result(_, ref preview_data) = results[0] {
            assert_eq!(preview_data.subdivisions, MIN_SUBDIVISIONS.get());
        } else {
            unreachable!()
        }
        assert!(matches!(results[1], ComputePreviewResult::Result(..)));
        if let ComputePreviewResult::Result(_, ref preview_data) = results[1] {
            assert_eq!(preview_data.subdivisions, MIN_SUBDIVISIONS.get() + 1);
        } else {
            unreachable!()
        }
        assert!(matches!(results[2], ComputePreviewResult::Result(..)));
        if let ComputePreviewResult::Result(_, ref preview_data) = results[2] {
            assert_eq!(preview_data.subdivisions, MIN_SUBDIVISIONS.get() + 2);
        } else {
            unreachable!()
        }
        assert_eq!(results[3], ComputePreviewResult::Finished);
        assert_eq!(results[4], ComputePreviewResult::Finished);
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
}
