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

use bevy::prelude::*;

use crate::undo::{Action, ReflectAction};

use super::components::{HeightMap, Layer, LayerBundle, LayerId, LAYER_SPACING};

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
                        .find(|layer| layer.id() == parent_id)
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
            .find(|(_, layer)| layer.id() == self.id)
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

#[cfg(test)]
mod tests {
    use super::super::LayerPlugin;
    use super::*;
    use crate::undo;

    use uuid::{uuid, Uuid};

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
            .map(|layer| layer.id())
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
            .filter(|layer| !initial_ids.contains(&layer.id()))
            .next()
            .unwrap();
        assert!(new_layer.order > FIRST_LAYER_ORDER);
        assert!(new_layer.order < FIRST_LAYER_ORDER + LAYER_SPACING);
    }
}
