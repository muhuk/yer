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
use serde::{Deserialize, Serialize};

use crate::id::{LayerId, MaskId};
use crate::layer::Layer;
use crate::undo::{Action, ReflectAction};

pub const LAYER_SPACING: u32 = 100;

// PLUGIN

pub struct MaskPlugin;

impl Plugin for MaskPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Mask>()
            .register_type::<MaskOrder>()
            .register_type::<SdfMask>();
    }
}

// BUNDLES

#[derive(Bundle, Clone, Debug, Default, Reflect)]
pub struct MaskBundle {
    pub mask: Mask,
    pub sdf_mask: SdfMask,
}

// COMPONENTS

#[derive(Clone, Component, Debug, Reflect)]
pub struct Mask {
    id: MaskId,
}

impl Default for Mask {
    fn default() -> Self {
        Self { id: Self::new_id() }
    }
}

impl Mask {
    pub fn id(&self) -> MaskId {
        self.id
    }

    pub(super) fn new(id: MaskId) -> Self {
        Self { id }
    }

    pub(super) fn new_id() -> MaskId {
        MaskId::now_v7()
    }
}

#[derive(
    Component,
    Copy,
    Clone,
    Debug,
    Deref,
    Deserialize,
    Eq,
    Ord,
    PartialEq,
    PartialOrd,
    Reflect,
    Serialize,
)]
#[require(Mask)]
pub struct MaskOrder(#[deref] u32);

#[derive(Clone, Component, Debug, Default, Reflect)]
#[require(Mask)]
pub struct SdfMask;

impl SdfMask {
    pub fn sample(&self, position: Vec2) -> f32 {
        // FIXME: use actual mask data.
        //
        // Inside the circle on origin, with r=10.
        if (position - Vec2::ZERO).length() < 10.0 {
            1.0
        } else {
            0.0
        }
    }
}

// ACTIONS

#[derive(Debug, Reflect)]
#[reflect(Action)]
pub struct CreateMaskAction {
    mask_bundle: MaskBundle,
    layer: LayerId,
    previous_mask: Option<MaskId>,
}

impl CreateMaskAction {
    pub fn new(mask_bundle: MaskBundle, layer: LayerId, previous_mask: Option<MaskId>) -> Self {
        Self {
            mask_bundle,
            layer,
            previous_mask,
        }
    }
}

impl Action for CreateMaskAction {
    fn apply(&self, world: &mut World) {
        // FIXME: This creates a circular dependency if layer module
        //        depends back on mask module.
        let parent: Entity = world
            .query::<(Entity, &Layer)>()
            .iter(world)
            .find(|(_, layer)| layer.id() == self.layer)
            .map(|(entity, _)| entity)
            .unwrap();
        let mask_order: MaskOrder = {
            let masks = world
                .entity(parent)
                .get::<Children>()
                .map(|children| children.to_vec())
                .unwrap_or_default();
            let bottom_mask_order: u32 = self
                .previous_mask
                .map(|previous_mask_id| {
                    world
                        .entity(masks.as_slice())
                        .iter()
                        .find_map(|entity| {
                            if entity.get::<Mask>().unwrap().id() == previous_mask_id {
                                Some(entity.get::<MaskOrder>().unwrap().0)
                            } else {
                                None
                            }
                        })
                        // self.previous_mask might be None, but the query fetching
                        // the layer order for a given previous_mask_id cannot be None.
                        .unwrap()
                })
                .unwrap_or(0);
            let top_mask_order: u32 = world
                .entity(masks.as_slice())
                .iter()
                .map(|entity| entity.get::<MaskOrder>().unwrap().0)
                .filter(|order| *order > bottom_mask_order)
                .max()
                .unwrap_or(bottom_mask_order + 2 * LAYER_SPACING);
            MaskOrder((bottom_mask_order + top_mask_order) / 2)
        };
        world.spawn((self.mask_bundle.clone(), mask_order, ChildOf(parent)));
    }

    fn revert(&self, world: &mut World) {
        DeleteMaskAction {
            mask_bundle: self.mask_bundle.clone(),
            layer: self.layer,
            previous_mask: self.previous_mask,
        }
        .apply(world);
    }
}

#[derive(Debug, Reflect)]
#[reflect(Action)]
pub struct DeleteMaskAction {
    mask_bundle: MaskBundle,
    layer: LayerId,
    previous_mask: Option<MaskId>,
}

impl DeleteMaskAction {
    pub fn new(mask_bundle: MaskBundle, layer: LayerId, previous_mask: Option<MaskId>) -> Self {
        Self {
            mask_bundle,
            layer,
            previous_mask,
        }
    }
}

impl Action for DeleteMaskAction {
    fn apply(&self, world: &mut World) {
        match world
            .query::<(Entity, &Mask)>()
            .iter(world)
            .find(|(_, mask)| mask.id() == self.mask_bundle.mask.id())
        {
            Some((entity, _)) => {
                world.despawn(entity);
            }
            None => warn!(
                "Trying to delete non-existent layer with id '{}'",
                self.mask_bundle.mask.id().simple()
            ),
        }
    }

    fn revert(&self, world: &mut World) {
        CreateMaskAction {
            mask_bundle: self.mask_bundle.clone(),
            layer: self.layer,
            previous_mask: self.previous_mask,
        }
        .apply(world);
    }
}
