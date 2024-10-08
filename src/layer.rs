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
use uuid::Uuid;

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
            normalize_layer_ordering.run_if(on_timer(Duration::from_millis(
                NORMALIZE_ORDERING_INTERVAL_MS,
            ))),
        );
    }
}

// EVENTS

#[derive(Event, Debug)]
pub enum LayerChange {
    Added(Uuid),
}

// BUNDLES

#[derive(Bundle)]
struct LayerBundle {
    layer: Layer,
    height_map: HeightMap,
}

// COMPONENTS

#[derive(Component, Debug, Reflect)]
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

#[derive(Component, Debug, Eq, Ord, Reflect)]
#[reflect(Component)]
pub struct Layer {
    pub enable_baking: bool,
    pub enable_preview: bool,
    id: Uuid,
    order: u32,
}

impl Layer {
    fn new(order: u32) -> Self {
        Self {
            enable_baking: true,
            enable_preview: true,
            id: Uuid::now_v7(),
            order,
        }
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
        let layer: Layer = match self {
            Self::OnTop => {
                // Find the `order` of the top layer:
                let max_order: u32 = world
                    .query::<&Layer>()
                    .iter(world)
                    .sort::<&Layer>()
                    .last()
                    .map_or(0, |layer| layer.order);
                Layer::new(max_order + LAYER_SPACING)
            }
            Self::Above(id) => {
                let bottom_layer_order = world
                    .query::<&Layer>()
                    .iter(world)
                    .find(|layer| layer.id == id)
                    .map(|layer| layer.order)
                    .unwrap();
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
                Layer::new((bottom_layer_order + top_layer_order) / 2)
            }
        };
        world.send_event(LayerChange::Added(layer.id));
        world.spawn(LayerBundle {
            layer,
            height_map: HeightMap::default(),
        });
    }
}

pub struct DeleteLayer(pub Uuid);

impl Command for DeleteLayer {
    fn apply(self, world: &mut World) {
        match world
            .query::<(Entity, &Layer)>()
            .iter(world)
            .find(|(_, layer)| layer.id == self.0)
        {
            Some((entity, _)) => {
                world.despawn(entity);
            }
            None => warn!(
                "Trying to delete non-existent layer with id '{}'",
                self.0.simple()
            ),
        }
    }
}

// SYSTEMS

fn normalize_layer_ordering(mut layers: Query<&mut Layer>) {
    info!("Normalizing layer ordering.");
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

trait Sample2D {
    fn sample(&self, position: Vec2, height: f32) -> f32;
}

#[cfg(test)]
mod tests {
    use super::*;

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
            Layer::new(FIRST_LAYER_ORDER),
            Layer::new(FIRST_LAYER_ORDER + LAYER_SPACING),
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
