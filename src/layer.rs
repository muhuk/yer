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

use std::fmt::{self, Display};

use bevy::ecs::world::Command;
use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
use bevy::utils::Duration;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::undo::{self, Action, ReflectAction};

const LAYER_SPACING: u32 = 100;
const NORMALIZE_ORDERING_INTERVAL_MS: u64 = 500;

pub struct LayerPlugin;

impl Plugin for LayerPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<HeightMap>()
            .register_type::<Layer>()
            .register_type::<Uuid>()
            .add_event::<LayerChange>();
        app.add_systems(
            Update,
            normalize_layer_ordering_system.run_if(on_timer(Duration::from_millis(
                NORMALIZE_ORDERING_INTERVAL_MS,
            ))),
        );
    }

    fn finish(&self, app: &mut App) {
        if !app.is_plugin_added::<undo::UndoPlugin>() {
            app.add_plugins(undo::UndoPlugin);
        }
    }
}

// EVENTS

#[derive(Event, Debug)]
pub enum LayerChange {
    Created(Uuid),
    Deleted(Uuid),
}

// BUNDLES

#[derive(Bundle, Deserialize, Serialize)]
pub struct LayerBundle {
    layer: Layer,
    height_map: HeightMap,
}

impl LayerBundle {
    pub fn extract_all(world: &mut World) -> Vec<Self> {
        let mut layer_bundles = vec![];
        for (layer, height_map) in world.query::<(&Layer, &HeightMap)>().iter(world) {
            layer_bundles.push(Self {
                layer: layer.clone(),
                height_map: height_map.clone(),
            });
        }
        layer_bundles
    }
}

// COMPONENTS

#[derive(Component, Clone, Debug, Deserialize, Reflect, Serialize)]
#[reflect(Component, Default)]
pub enum HeightMap {
    Constant(f32),
}

impl Default for HeightMap {
    fn default() -> Self {
        Self::Constant(0.0)
    }
}

impl Sample2D for HeightMap {
    fn sample(&self, _position: Vec2, _height: f32) -> f32 {
        match self {
            Self::Constant(value) => *value,
        }
    }
}

#[derive(Component, Clone, Debug, Deserialize, Eq, Ord, Reflect, Serialize)]
#[reflect(Component)]
pub struct Layer {
    pub enable_baking: bool,
    pub enable_preview: bool,
    id: Uuid,
    order: u32,
}

impl Layer {
    fn new(id: Uuid, order: u32) -> Self {
        Self {
            enable_baking: true,
            enable_preview: true,
            id,
            order,
        }
    }

    fn new_id() -> Uuid {
        Uuid::now_v7()
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
}

impl Display for Layer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}-{}",
            self.order,
            // Last 7 digits of uuid.
            &self.id.simple().to_string()[25..32]
        )
    }
}

impl PartialEq for Layer {
    fn eq(&self, other: &Self) -> bool {
        self.order == other.order
    }
}

impl PartialOrd for Layer {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.order.partial_cmp(&other.order)
    }
}

// COMMANDS

pub enum CreateLayer {
    OnTop,
    Above(Uuid),
}

impl Command for CreateLayer {
    fn apply(self, world: &mut World) {
        let above: Option<Uuid> = match self {
            Self::OnTop => world
                .query::<&Layer>()
                .iter(world)
                .sort::<&Layer>()
                .last()
                .map(|layer| layer.id),
            Self::Above(id) => Some(id),
        };

        let action = CreateLayerAction {
            id: Layer::new_id(),
            parent_id: above,
        };
        undo::PushAction(Box::new(action)).apply(world);
    }
}

pub struct DeleteLayer(pub Uuid);

impl Command for DeleteLayer {
    fn apply(self, world: &mut World) {
        let layer_order: Option<u32> = world
            .query::<&Layer>()
            .iter(world)
            .find(|layer| layer.id == self.0)
            .map(|layer| layer.order);

        match layer_order {
            Some(layer_order) => {
                let parent_id: Option<Uuid> = world
                    .query::<&Layer>()
                    .iter(world)
                    .sort::<&Layer>()
                    .filter(|layer| layer.order < layer_order)
                    .last()
                    .map(|layer| layer.id);
                let action = DeleteLayerAction {
                    id: self.0,
                    parent_id,
                };
                undo::PushAction(Box::new(action)).apply(world);
            }
            None => error!(
                "Trying to delete non-existent layer with id '{}'",
                self.0.simple()
            ),
        }
    }
}

// SYSTEMS

fn normalize_layer_ordering_system(mut layers: Query<&mut Layer>) {
    debug!("Normalizing layer ordering.");
    layers
        .iter_mut()
        .sort::<&Layer>()
        .enumerate()
        .for_each(|(idx, mut layer)| {
            // Start from LAYER_SPACING (1-based) and increment for
            // as much as LAYER_SPACING at each layer.
            layer.order =
                u32::try_from(idx + 1).expect("There are too many layers.") * LAYER_SPACING;
        });
}

// LIB

#[derive(Debug, Reflect)]
#[reflect(Action)]
struct CreateLayerAction {
    id: Uuid,
    parent_id: Option<Uuid>,
}

impl Action for CreateLayerAction {
    fn apply(&self, world: &mut World) {
        let layer: Layer = {
            let bottom_layer_order = self
                .parent_id
                .map(|parent_id| {
                    world
                        .query::<&Layer>()
                        .iter(world)
                        .find(|layer| layer.id == parent_id)
                        .map(|layer| layer.order)
                        .unwrap()
                })
                .unwrap_or(0);
            // In case bottom layer is the topmost layer (no other layer
            // above it), we end up with the order of bottom_layer_order +
            // LAYER_SPACING for the new layer, just like Self::OnTop.
            let top_layer_order = world
                .query::<&Layer>()
                .iter(world)
                .sort::<&Layer>()
                .filter(|layer| layer.order > bottom_layer_order)
                .next()
                .map_or(bottom_layer_order + 2 * LAYER_SPACING, |layer| layer.order);
            Layer::new(self.id, (bottom_layer_order + top_layer_order) / 2)
        };
        world.send_event(LayerChange::Created(self.id));
        world.spawn(LayerBundle {
            layer,
            height_map: HeightMap::default(),
        });
    }

    fn revert(&self, world: &mut World) {
        DeleteLayerAction {
            id: self.id,
            parent_id: None,
        }
        .apply(world)
    }
}

#[derive(Debug, Reflect)]
#[reflect(Action)]
struct DeleteLayerAction {
    id: Uuid,
    parent_id: Option<Uuid>,
}

impl Action for DeleteLayerAction {
    fn apply(&self, world: &mut World) {
        match world
            .query::<(Entity, &Layer)>()
            .iter(world)
            .find(|(_, layer)| layer.id == self.id)
        {
            Some((entity, _)) => {
                world.despawn(entity);
                world.send_event(LayerChange::Deleted(self.id));
            }
            None => warn!(
                "Trying to delete non-existent layer with id '{}'",
                self.id.simple()
            ),
        }
    }

    fn revert(&self, world: &mut World) {
        CreateLayerAction {
            id: self.id,
            parent_id: self.parent_id,
        }
        .apply(world)
    }
}

trait Sample2D {
    fn sample(&self, position: Vec2, height: f32) -> f32;
}

/// This is intended to be called to create the initial layer only.  It does
/// not emit LayerChange::Added event.
pub fn create_initial_layer(world: &mut World) {
    const ORDER: u32 = 0;
    world.spawn(LayerBundle {
        layer: Layer::new(Layer::new_id(), ORDER),
        height_map: HeightMap::default(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    use uuid::uuid;

    const A: Uuid = uuid!("0192bf46-8e52-7dc5-b6f5-05bc9ae3aaa3");
    const B: Uuid = uuid!("0192bf47-0c63-79a3-983c-92445e2b56a9");

    macro_rules! assert_layer_count {
        ($app:expr, $expected:expr) => {
            assert_eq!(
                $app.world_mut()
                    .query::<&Layer>()
                    .iter($app.world())
                    .count(),
                $expected
            )
        };
    }

    #[test]
    fn add_new_layer_on_top() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, LayerPlugin));
        app.finish();
        app.cleanup();
        app.update();

        assert_layer_count!(app, 0);
        app.world_mut().commands().push(CreateLayer::OnTop);
        app.update();
        assert_layer_count!(app, 1);
    }

    #[test]
    fn add_new_layer_in_between() {
        const FIRST_LAYER_ORDER: u32 = 3000;

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, LayerPlugin));
        app.finish();
        app.cleanup();
        app.update();

        app.world_mut().commands().spawn_batch([
            Layer::new(A, FIRST_LAYER_ORDER),
            Layer::new(B, FIRST_LAYER_ORDER + LAYER_SPACING),
        ]);
        app.update();
        assert_layer_count!(app, 2);

        let initial_ids: Vec<Uuid> = app
            .world_mut()
            .query::<&Layer>()
            .iter(app.world())
            .map(|layer| layer.id)
            .collect();
        app.world_mut()
            .commands()
            .push(CreateLayer::Above(initial_ids[0]));
        app.update();
        assert_layer_count!(app, 3);

        let new_layer = app
            .world_mut()
            .query::<&Layer>()
            .iter(app.world())
            .filter(|layer| !initial_ids.contains(&layer.id))
            .next()
            .unwrap();
        assert!(new_layer.order > FIRST_LAYER_ORDER);
        assert!(new_layer.order < FIRST_LAYER_ORDER + LAYER_SPACING);
    }
}
