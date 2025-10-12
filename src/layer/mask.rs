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

use std::f32::consts::TAU;

use bevy::ecs::{component::HookContext, world::DeferredWorld};
use bevy::math::Affine2;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::id::{LayerId, MaskId};
// FIXME: Circular dependency
use crate::layer::Layer;
use crate::math::clamp;
use crate::undo::{Action, ReflectAction};

pub const MASK_SPACING: u32 = 100;

// PLUGIN

pub struct MaskPlugin;

impl Plugin for MaskPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Mask>()
            .register_type::<MaskOrder>()
            .register_type::<MaskSource>();
        app.register_type::<NeedsMaskOrderNormalization>();
        app.add_systems(
            PreUpdate,
            (
                mark_for_mask_order_normalization_system,
                normalize_mask_ordering_system
                    .run_if(any_with_component::<NeedsMaskOrderNormalization>),
            )
                .chain(),
        );
    }
}

// BUNDLES

#[derive(Bundle, Clone, Debug, Reflect)]
pub struct MaskBundle {
    pub mask: Mask,
    pub mask_source: MaskSource,
}

// COMPONENTS

#[derive(Clone, Component, Debug, Reflect)]
pub struct Mask {
    pub composition_mode: MaskCompositionMode,
    pub is_enabled: bool,
    pub strength: f32,
    id: MaskId,
}

impl Mask {
    pub fn combine(&self, baseline: f32, sample: f32) -> f32 {
        self.composition_mode
            .combine(baseline, sample * self.strength)
    }

    pub fn id(&self) -> MaskId {
        self.id
    }

    pub(super) fn new_id() -> MaskId {
        MaskId::now_v7()
    }
}

impl Default for Mask {
    fn default() -> Self {
        Self {
            composition_mode: MaskCompositionMode::default(),
            is_enabled: true,
            strength: 1.0,
            id: Self::new_id(),
        }
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
#[component(on_remove = mask_order_on_remove_hook)]
#[require(Mask)]
pub struct MaskOrder(#[deref] u32);

/// This is added to a layer entity when its masks' order need to be
/// normalized.
#[derive(Clone, Component, Debug, Reflect)]
struct NeedsMaskOrderNormalization;

#[derive(Clone, Component, Debug, Reflect)]
#[require(Mask)]
pub enum MaskSource {
    Circle {
        center: Vec2,
        falloff_radius: f32,
        irregularity: f32,
        radius: f32,
        rotation: f32,
    },
    Square {
        center: Vec2,
        falloff_radius: f32,
        irregularity: f32,
        rotation: f32,
        size: f32,
    },
}

impl MaskSource {
    pub fn circle() -> Self {
        Self::Circle {
            center: Vec2::ZERO,
            falloff_radius: 0.5,
            irregularity: 0.0,
            radius: 1.5,
            rotation: 0.0,
        }
    }

    pub fn square() -> Self {
        Self::Square {
            center: Vec2::ZERO,
            falloff_radius: 0.5,
            irregularity: 0.0,
            rotation: 0.0,
            size: 2.0,
        }
    }
}

impl MaskSource {
    pub fn sample(&self, position: Vec2) -> f32 {
        match self {
            Self::Circle {
                center,
                falloff_radius,
                irregularity,
                radius,
                rotation,
            } => {
                let scale = {
                    const BASE: f32 = 10.0;
                    let f = BASE.powf(*irregularity);
                    Vec2::new(1.0 / f, f)
                };
                // Apply scale first, then rotation, then translation.
                // We are multiplying the transforms in reverse order since
                // it is a reverse transform.
                let transform = Affine2::from_scale(-scale)
                    * Affine2::from_angle(-*rotation * TAU)
                    * Affine2::from_translation(-center);
                let distance: f32 = transform.transform_point2(position).length();
                1.0 - clamp((distance - radius) / falloff_radius, 0.0, 1.0)
            }
            Self::Square {
                center,
                falloff_radius,
                irregularity,
                rotation,
                size,
            } => {
                let scale = {
                    const BASE: f32 = 10.0;
                    let f = BASE.powf(*irregularity);
                    Vec2::new(1.0 / f, f)
                };
                // Apply scale first, then rotation, then translation.
                // We are multiplying the transforms in reverse order since
                // it is a reverse transform.
                let transform = Affine2::from_scale(-scale)
                    * Affine2::from_angle(-*rotation * TAU)
                    * Affine2::from_translation(-center);
                let Vec2 { x: dx, y: dy } = transform.transform_point2(position).abs();
                let half_size: f32 = size * 0.5;
                let kx = 1.0 - clamp((dx - half_size) / falloff_radius, 0.0, 1.0);
                let ky = 1.0 - clamp((dy - half_size) / falloff_radius, 0.0, 1.0);
                kx * ky
            }
        }
    }

    fn set_center(&mut self, new_center: Vec2) {
        match self {
            Self::Circle { center, .. } => *center = new_center,
            Self::Square { center, .. } => *center = new_center,
        }
    }

    fn set_falloff_radius(&mut self, new_radius: f32) {
        match self {
            Self::Circle { falloff_radius, .. } => *falloff_radius = new_radius,
            Self::Square { falloff_radius, .. } => *falloff_radius = new_radius,
        }
    }

    fn set_irregularity(&mut self, mut new_irregularity: f32) {
        new_irregularity = new_irregularity.max(-1.0).min(1.0);
        match self {
            Self::Circle { irregularity, .. } => *irregularity = new_irregularity,
            Self::Square { irregularity, .. } => *irregularity = new_irregularity,
        }
    }

    fn set_radius(&mut self, new_radius: f32) {
        match self {
            Self::Circle { radius, .. } => *radius = new_radius,
            Self::Square { .. } => unreachable!(),
        }
    }

    fn set_rotation(&mut self, mut new_rotation: f32) {
        // Constrain rotation to [0.0; 1.0)
        new_rotation = new_rotation.fract();
        match self {
            Self::Circle { rotation, .. } => *rotation = new_rotation,
            Self::Square { rotation, .. } => *rotation = new_rotation,
        }
    }

    fn set_size(&mut self, new_size: f32) {
        match self {
            Self::Circle { .. } => unreachable!(),
            Self::Square { size, .. } => *size = new_size,
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
                .unwrap_or(bottom_mask_order + 2 * MASK_SPACING);
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

#[derive(Debug, Reflect)]
#[reflect(Action)]
pub enum UpdateMaskAction {
    ChangeCompositionMode {
        mask_id: MaskId,
        old_value: MaskCompositionMode,
        new_value: MaskCompositionMode,
    },
    ToggleEnabled {
        mask_id: MaskId,
        new_value: bool,
    },
    UpdateStrength {
        mask_id: MaskId,
        old_value: f32,
        new_value: f32,
    },
}

impl UpdateMaskAction {
    pub fn change_composition_mode(
        mask_id: MaskId,
        old_value: MaskCompositionMode,
        new_value: MaskCompositionMode,
    ) -> Self {
        debug_assert!(old_value != new_value);
        Self::ChangeCompositionMode {
            mask_id,
            old_value,
            new_value,
        }
    }

    pub fn toggle_enabled(mask_id: MaskId, new_value: bool) -> Self {
        Self::ToggleEnabled { mask_id, new_value }
    }

    pub fn update_strength(mask_id: MaskId, old_value: f32, new_value: f32) -> Self {
        Self::UpdateStrength {
            mask_id,
            old_value,
            new_value,
        }
    }

    fn mask_id(&self) -> &MaskId {
        match self {
            Self::ChangeCompositionMode { mask_id, .. } => mask_id,
            Self::ToggleEnabled { mask_id, .. } => mask_id,
            Self::UpdateStrength { mask_id, .. } => mask_id,
        }
    }
}

impl Action for UpdateMaskAction {
    fn apply(&self, world: &mut World) {
        let mut mask = world
            .query::<&mut Mask>()
            .iter_mut(world)
            .find(|mask| mask.id == *self.mask_id())
            .expect(&format!("Mask with id {} not found.", self.mask_id()));
        match self {
            Self::ChangeCompositionMode { new_value, .. } => mask.composition_mode = *new_value,
            Self::ToggleEnabled { new_value, .. } => mask.is_enabled = *new_value,
            Self::UpdateStrength { new_value, .. } => mask.strength = *new_value,
        };
    }

    fn revert(&self, world: &mut World) {
        let reverse_action: Self = match *self {
            Self::ChangeCompositionMode {
                mask_id,
                old_value,
                new_value,
            } => Self::ChangeCompositionMode {
                mask_id,
                old_value: new_value,
                new_value: old_value,
            },
            Self::ToggleEnabled { mask_id, new_value } => Self::ToggleEnabled {
                mask_id,
                new_value: !new_value,
            },
            Self::UpdateStrength {
                mask_id,
                old_value,
                new_value,
            } => Self::UpdateStrength {
                mask_id,
                old_value: new_value,
                new_value: old_value,
            },
        };
        reverse_action.apply(world);
    }
}

#[derive(Debug, Reflect)]
#[reflect(Action)]
pub enum UpdateMaskSourceAction {
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
    UpdateIrregularity {
        mask_id: MaskId,
        old_value: f32,
        new_value: f32,
    },
    UpdateRadius {
        mask_id: MaskId,
        old_value: f32,
        new_value: f32,
    },
    UpdateRotation {
        mask_id: MaskId,
        old_value: f32,
        new_value: f32,
    },
    UpdateSize {
        mask_id: MaskId,
        old_value: f32,
        new_value: f32,
    },
}

impl UpdateMaskSourceAction {
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

    pub fn update_irregularity(mask_id: MaskId, old_value: f32, new_value: f32) -> Self {
        Self::UpdateIrregularity {
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

    pub fn update_rotation(mask_id: MaskId, old_value: f32, new_value: f32) -> Self {
        Self::UpdateRotation {
            mask_id,
            old_value,
            new_value,
        }
    }

    pub fn update_size(mask_id: MaskId, old_value: f32, new_value: f32) -> Self {
        Self::UpdateSize {
            mask_id,
            old_value,
            new_value,
        }
    }

    fn mask_id(&self) -> &MaskId {
        match self {
            Self::UpdateCenter { mask_id, .. } => mask_id,
            Self::UpdateFalloffRadius { mask_id, .. } => mask_id,
            Self::UpdateIrregularity { mask_id, .. } => mask_id,
            Self::UpdateRadius { mask_id, .. } => mask_id,
            Self::UpdateRotation { mask_id, .. } => mask_id,
            Self::UpdateSize { mask_id, .. } => mask_id,
        }
    }
}

impl Action for UpdateMaskSourceAction {
    fn apply(&self, world: &mut World) {
        let mut mask_source = world
            .query::<(&mut Mask, &mut MaskSource)>()
            .iter_mut(world)
            .find(|(mask, _)| mask.id == *self.mask_id())
            .map(|(_, mask_source)| mask_source)
            .expect(&format!("Mask with id {} not found.", self.mask_id()));
        match self {
            Self::UpdateCenter { new_value, .. } => mask_source.set_center(*new_value),
            Self::UpdateFalloffRadius { new_value, .. } => {
                mask_source.set_falloff_radius(*new_value)
            }
            Self::UpdateIrregularity { new_value, .. } => mask_source.set_irregularity(*new_value),
            Self::UpdateRadius { new_value, .. } => mask_source.set_radius(*new_value),
            Self::UpdateRotation { new_value, .. } => mask_source.set_rotation(*new_value),
            Self::UpdateSize { new_value, .. } => mask_source.set_size(*new_value),
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
            Self::UpdateIrregularity {
                mask_id,
                old_value,
                new_value,
            } => Self::UpdateIrregularity {
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
            Self::UpdateRotation {
                mask_id,
                old_value,
                new_value,
            } => Self::UpdateRotation {
                mask_id,
                old_value: new_value,
                new_value: old_value,
            },
            Self::UpdateSize {
                mask_id,
                old_value,
                new_value,
            } => Self::UpdateSize {
                mask_id,
                old_value: new_value,
                new_value: old_value,
            },
        };
        reverse_action.apply(world);
    }
}

// SYSTEMS

fn mark_for_mask_order_normalization_system(
    changed_masks_query: Query<&ChildOf, Changed<MaskOrder>>,
    mut commands: Commands,
) {
    for ChildOf(parent) in changed_masks_query.iter() {
        commands.entity(*parent).insert(NeedsMaskOrderNormalization);
    }
}

fn normalize_mask_ordering_system(
    mut commands: Commands,
    layers_query: Query<Entity, With<NeedsMaskOrderNormalization>>,
    mut masks_query: Query<(&mut MaskOrder, &ChildOf)>,
) {
    trace!("Normalizing mask ordering.");
    for layer_entity in layers_query.iter() {
        masks_query
            .iter_mut()
            // Ideally `sort` should come after `filter`, but this should work too.
            .sort::<&MaskOrder>()
            .filter(|(_, ChildOf(parent))| *parent == layer_entity)
            .enumerate()
            .for_each(|(idx, (mut mask_order, _))| {
                // Start from MASK_SPACING (1-based) and increment for
                // as much as MASK_SPACING at each layer.
                mask_order.bypass_change_detection().0 =
                    u32::try_from(idx + 1).expect("There are too many masks.") * MASK_SPACING;
            });
        commands
            .entity(layer_entity)
            .remove::<NeedsMaskOrderNormalization>();
    }
}

// LIB

#[derive(Clone, Copy, Debug, Default, PartialEq, Reflect)]
pub enum MaskCompositionMode {
    #[default]
    Add,
    Sub,
    Min,
    Max,
}

impl MaskCompositionMode {
    pub const ITEMS: [Self; 4] = [Self::Add, Self::Sub, Self::Min, Self::Max];

    pub fn combine(&self, baseline: f32, sample: f32) -> f32 {
        match self {
            Self::Add => clamp(baseline + sample, 0.0, 1.0),
            Self::Sub => clamp(baseline - sample, 0.0, 1.0),
            Self::Min => clamp(baseline.min(sample), 0.0, 1.0),
            Self::Max => clamp(baseline.max(sample), 0.0, 1.0),
        }
    }
}

impl ToString for MaskCompositionMode {
    fn to_string(&self) -> String {
        (match self {
            Self::Add => "Add",
            Self::Sub => "Sub",
            Self::Min => "Min",
            Self::Max => "Max",
        })
        .into()
    }
}

fn mask_order_on_remove_hook(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    if let Some(ChildOf(parent)) = world.entity(entity).get::<ChildOf>().cloned() {
        // The layer might be deleted, so we cannot assume the parent entity exists
        // when the new command we're adding here gets executed.
        world
            .commands()
            .get_entity(parent)
            .iter_mut()
            .for_each(|entity| {
                entity.insert(NeedsMaskOrderNormalization);
            });
    } else {
        error!("Mask {} has no parent.", entity);
    }
}
