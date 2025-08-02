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
use crate::math::clamp;
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

#[derive(Clone, Component, Debug, Reflect)]
#[require(Mask)]
pub enum SdfMask {
    Circle {
        center: Vec2,
        radius: f32,
        falloff_radius: f32,
    },
}

impl SdfMask {
    pub fn sample(&self, position: Vec2) -> f32 {
        match self {
            Self::Circle {
                center,
                radius,
                falloff_radius,
            } => {
                let distance: f32 = (position - center).length();
                1.0 - clamp((distance - radius) / falloff_radius, 0.0, 1.0)
            }
        }
    }

    fn set_center(&mut self, new_center: Vec2) {
        match self {
            Self::Circle { center, .. } => *center = new_center,
        }
    }

    fn set_falloff_radius(&mut self, new_radius: f32) {
        match self {
            Self::Circle { falloff_radius, .. } => *falloff_radius = new_radius,
        }
    }

    fn set_radius(&mut self, new_radius: f32) {
        match self {
            Self::Circle { radius, .. } => *radius = new_radius,
        }
    }
}

impl Default for SdfMask {
    fn default() -> Self {
        Self::Circle {
            center: Vec2::ZERO,
            radius: 1.5,
            falloff_radius: 0.5,
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

// TODO: Rename this as UpdateSdfMaskAction
#[derive(Debug, Reflect)]
#[reflect(Action)]
pub enum UpdateMaskAction {
    UpdateCenter {
        mask_id: MaskId,
        old_value: Vec2,
        new_value: Vec2,
    },
    UpdateFalloffRadius {
        mask_id: MaskId,
        old_value: f32,
        new_value: f32,
    },
    UpdateRadius {
        mask_id: MaskId,
        old_value: f32,
        new_value: f32,
    },
}

impl UpdateMaskAction {
    pub fn update_center(mask_id: MaskId, old_value: Vec2, new_value: Vec2) -> Self {
        Self::UpdateCenter {
            mask_id,
            old_value,
            new_value,
        }
    }

    pub fn update_falloff_radius(mask_id: MaskId, old_value: f32, new_value: f32) -> Self {
        Self::UpdateFalloffRadius {
            mask_id,
            old_value,
            new_value,
        }
    }

    pub fn update_radius(mask_id: MaskId, old_value: f32, new_value: f32) -> Self {
        Self::UpdateRadius {
            mask_id,
            old_value,
            new_value,
        }
    }

    fn mask_id(&self) -> &MaskId {
        match self {
            Self::UpdateCenter { mask_id, .. } => mask_id,
            Self::UpdateFalloffRadius { mask_id, .. } => mask_id,
            Self::UpdateRadius { mask_id, .. } => mask_id,
        }
    }
}

impl Action for UpdateMaskAction {
    fn apply(&self, world: &mut World) {
        let mut sdf_mask = world
            .query::<(&Mask, &mut SdfMask)>()
            .iter_mut(world)
            .find(|(mask, _)| mask.id == *self.mask_id())
            .map(|(_, sdf_mask)| sdf_mask)
            .expect(&format!("Mask with id {} not found.", self.mask_id()));
        match self {
            Self::UpdateCenter { new_value, .. } => sdf_mask.set_center(*new_value),
            Self::UpdateFalloffRadius { new_value, .. } => sdf_mask.set_falloff_radius(*new_value),
            Self::UpdateRadius { new_value, .. } => sdf_mask.set_radius(*new_value),
        };
    }

    fn revert(&self, world: &mut World) {
        let reverse_action: Self = match *self {
            Self::UpdateCenter {
                mask_id,
                old_value,
                new_value,
            } => Self::UpdateCenter {
                mask_id,
                old_value: new_value,
                new_value: old_value,
            },
            Self::UpdateFalloffRadius {
                mask_id,
                old_value,
                new_value,
            } => Self::UpdateFalloffRadius {
                mask_id,
                old_value: new_value,
                new_value: old_value,
            },
            Self::UpdateRadius {
                mask_id,
                old_value,
                new_value,
            } => Self::UpdateRadius {
                mask_id,
                old_value: new_value,
                new_value: old_value,
            },
        };
        reverse_action.apply(world);
    }
}
