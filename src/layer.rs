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

use std::fmt::{self, Display};
use std::ops::RangeInclusive;
use std::time::Duration;

use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::undo::{self, Action, ReflectAction};

pub type LayerId = uuid::Uuid;

pub const HEIGHT_RANGE: RangeInclusive<f32> = -16000.0..=64000.0;

const DEFAULT_LAYER_NAME: &str = "<unnamed>";
const LAYER_SPACING: u32 = 100;
const NORMALIZE_ORDERING_INTERVAL_MS: u64 = 500;

pub struct LayerPlugin;

impl Plugin for LayerPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<HeightMap>()
            .register_type::<Layer>()
            .register_type::<LayerId>();
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

// BUNDLES

#[derive(Bundle, Deserialize, Serialize)]
pub struct LayerBundle {
    layer: Layer,
    name: Name,
    height_map: HeightMap,
}

impl LayerBundle {
    pub fn extract_all(world: &mut World) -> Vec<Self> {
        let mut layer_bundles = vec![];
        for (layer, name, height_map) in world.query::<(&Layer, &Name, &HeightMap)>().iter(world) {
            layer_bundles.push(Self {
                layer: layer.clone(),
                name: name.clone(),
                height_map: height_map.clone(),
            });
        }
        layer_bundles
    }
}

// COMPONENTS

#[derive(Component, Clone, Debug, Deserialize, Reflect, Serialize)]
#[reflect(Component, Default)]
#[require(Layer)]
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
    pub name: String,
    pub enable_baking: bool,
    pub enable_preview: bool,
    id: LayerId,
    order: u32,
}

impl Layer {
    const DEFAULT_ORDER: u32 = 0;

    pub fn id(&self) -> LayerId {
        self.id
    }

    fn name_component(&self) -> Name {
        Name::new(format!("Layer 0x{}", &self.id.simple().to_string()[25..32]))
    }

    fn new(id: LayerId, order: u32) -> Self {
        Self {
            name: DEFAULT_LAYER_NAME.to_owned(),
            enable_baking: true,
            enable_preview: true,
            id,
            order,
        }
    }

    fn new_id() -> LayerId {
        Uuid::now_v7()
    }
}

impl Default for Layer {
    fn default() -> Self {
        Self::new(Self::new_id(), Self::DEFAULT_ORDER)
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

// SYSTEMS

fn normalize_layer_ordering_system(mut layers: Query<&mut Layer>) {
    trace!("Normalizing layer ordering.");
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
pub struct CreateLayerAction {
    id: LayerId,
    parent_id: Option<LayerId>,
}

impl CreateLayerAction {
    pub fn new(parent_id: Option<LayerId>) -> Self {
        Self {
            id: Layer::new_id(),
            parent_id,
        }
    }
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
        world.spawn(LayerBundle {
            name: layer.name_component(),
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
pub struct DeleteLayerAction {
    id: LayerId,
    parent_id: Option<LayerId>,
}

impl DeleteLayerAction {
    pub fn new(id: LayerId, parent_id: Option<LayerId>) -> Self {
        Self { id, parent_id }
    }
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

#[derive(Debug, Reflect)]
#[reflect(Action)]
pub struct RenameLayerAction {
    layer_id: LayerId,
    old_name: String,
    new_name: String,
}

impl RenameLayerAction {
    pub fn new<A, B>(layer_id: LayerId, old_name: A, new_name: B) -> Self
    where
        A: Into<String>,
        B: Into<String>,
    {
        Self {
            layer_id,
            old_name: old_name.into(),
            new_name: new_name.into(),
        }
    }

    fn rename(&self, world: &mut World, reversed: bool) {
        let (new_name, old_name) = if reversed {
            (&self.old_name, &self.new_name)
        } else {
            (&self.new_name, &self.old_name)
        };
        world
            .query::<&mut Layer>()
            .iter_mut(world)
            .find(|layer| layer.id() == self.layer_id)
            .map(|mut layer| {
                debug_assert!(layer.name == *old_name);
                layer.name = new_name.to_string();
            })
            .expect(&format!("Layer with id {} not found.", self.layer_id));
    }
}

impl Action for RenameLayerAction {
    fn apply(&self, world: &mut World) {
        let reversed = false;
        self.rename(world, reversed);
    }

    fn revert(&self, world: &mut World) {
        let reversed = true;
        self.rename(world, reversed);
    }
}

#[derive(Debug, Reflect)]
#[reflect(Action)]
pub struct HeightMapConstantUpdateHeightAction {
    layer_id: LayerId,
    old_height: f32,
    new_height: f32,
}

impl HeightMapConstantUpdateHeightAction {
    pub fn new(layer_id: LayerId, old_height: f32, new_height: f32) -> Self {
        Self {
            layer_id,
            old_height,
            new_height,
        }
    }
}

impl Action for HeightMapConstantUpdateHeightAction {
    fn apply(&self, world: &mut World) {
        world
            .query::<(&Layer, &mut HeightMap)>()
            .iter_mut(world)
            .find(|(layer, _)| layer.id() == self.layer_id)
            .map(|(_, mut height_map)| match *height_map {
                HeightMap::Constant(ref mut height) => {
                    debug_assert!((*height - self.old_height).abs() < f32::EPSILON);
                    *height = self.new_height;
                }
            })
            .expect(&format!("Layer with id {} not found.", self.layer_id));
    }

    fn revert(&self, world: &mut World) {
        let reverse_action = Self {
            layer_id: self.layer_id,
            old_height: self.new_height,
            new_height: self.old_height,
        };
        reverse_action.apply(world);
    }
}

pub trait Sample2D: Send + Sync {
    fn sample(&self, position: Vec2, height: f32) -> f32;
}

#[derive(Debug, Reflect)]
#[reflect(Action)]
pub struct UpdateLayerAction {
    layer_id: LayerId,
    old_enable_baking: bool,
    new_enable_baking: bool,
    old_enable_preview: bool,
    new_enable_preview: bool,
}

impl UpdateLayerAction {
    pub fn toggle_enable_baking(layer: &Layer) -> Self {
        Self {
            layer_id: layer.id(),
            old_enable_baking: layer.enable_baking,
            new_enable_baking: !layer.enable_baking,
            old_enable_preview: layer.enable_preview,
            new_enable_preview: layer.enable_preview,
        }
    }

    pub fn toggle_enable_preview(layer: &Layer) -> Self {
        Self {
            layer_id: layer.id(),
            old_enable_baking: layer.enable_baking,
            new_enable_baking: layer.enable_baking,
            old_enable_preview: layer.enable_preview,
            new_enable_preview: !layer.enable_preview,
        }
    }
}

impl Action for UpdateLayerAction {
    fn apply(&self, world: &mut World) {
        world
            .query::<&mut Layer>()
            .iter_mut(world)
            .find(|layer| layer.id() == self.layer_id)
            .map(|mut layer| {
                layer.enable_baking = self.new_enable_baking;
                layer.enable_preview = self.new_enable_preview;
            })
            .expect(&format!("Layer with id {} not found.", self.layer_id));
    }

    fn revert(&self, world: &mut World) {
        let reverse_action = Self {
            layer_id: self.layer_id,
            old_enable_baking: self.new_enable_baking,
            new_enable_baking: self.old_enable_baking,
            old_enable_preview: self.new_enable_preview,
            new_enable_preview: self.old_enable_preview,
        };
        reverse_action.apply(world);
    }
}

/// This is intended to be called to create the initial layer only.  It does
/// not emit LayerChange::Added event.
pub fn create_initial_layer(world: &mut World) {
    let layer = Layer::default();
    world.spawn(LayerBundle {
        name: layer.name_component(),
        layer,
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
        app.world_mut()
            .commands()
            .queue(undo::PushAction::from(CreateLayerAction::new(None)));
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
            .queue(undo::PushAction::from(CreateLayerAction::new(Some(
                initial_ids[0],
            ))));
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
