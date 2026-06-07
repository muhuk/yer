// Copyright © 2024-2026 Atamert Ölçgen.
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

use std::ops::RangeInclusive;
use std::time::Duration;

use bevy::ecs::{query::QueryData, system::SystemParam};
use bevy::prelude::*;
use bevy_egui::egui;

use crate::bitmap::BitmapServer;
use crate::id::{LayerId, MaskId};
use crate::layer::{self, HeightMap};
use crate::math::{ApproxEq, Transform2D};
use crate::theme;
use crate::undo;

use super::egui_ext::{draw_ui_editable_f32, ToColor32};
use super::file_dialog::{DialogClosed, DialogResult, LoadImageDialog};
use super::state::UiState;

const LATENCY: Duration = Duration::from_millis(100);
const LAYER_SELECTION_BOX_WIDTH: f32 = 24.0f32;
const MINUS_ONE_TO_ONE: RangeInclusive<f32> = -1.0..=1.0;
const SCALE_LIMITS: RangeInclusive<f32> = 0.0001..=1000000.0;
const ZERO_TO_POSITIVE_INFINITY: RangeInclusive<f32> = 0.0..=f32::INFINITY;
const ZERO_TO_ONE: RangeInclusive<f32> = 0.0..=1.0;
const ZERO_TO_ONE_INCREMENT: f32 = 0.0025;

// PLUGIN

pub struct LayerUiPlugin;

impl Plugin for LayerUiPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<HeightMapUi>()
            .register_type::<LayerUi>()
            .register_type::<MaskUi>()
            .register_type::<MaskSourceUi>()
            .register_type::<Selected>();
        app.add_systems(
            Update,
            (
                add_layer_ui_system,
                add_mask_ui_system,
                update_height_map_ui_system,
                update_mask_ui_system,
                reset_height_map_ui_system,
                reset_mask_ui_system,
            ),
        );
    }
}

// SYSTEM PARAM & QUERY DATA

#[derive(QueryData)]
#[query_data(mutable)]
pub(super) struct LayerQuery {
    pub entity: Entity,
    pub name: &'static Name,
    pub layer: &'static layer::Layer,
    pub layer_order: &'static layer::LayerOrder,
    pub layer_ui: &'static mut LayerUi,
    pub height_map: &'static layer::HeightMap,
    pub height_map_ui: &'static mut HeightMapUi,
    pub is_selected: Has<Selected>,
}

impl<'w, 's> LayerQueryItem<'w, 's> {
    fn to_layer_bundle(&self) -> layer::LayerBundle {
        layer::LayerBundle {
            layer: self.layer.clone(),
            name: self.name.clone(),
            height_map: self.height_map.clone(),
        }
    }
}

#[derive(Deref, DerefMut, SystemParam)]
pub(super) struct Layers<'w, 's> {
    #[deref]
    pub layers: Query<'w, 's, LayerQuery>,
}

#[derive(QueryData)]
#[query_data(mutable)]
pub(super) struct MaskQuery {
    pub entity: Entity,
    pub child_of: &'static ChildOf,
    pub mask: &'static layer::Mask,
    pub mask_ui: &'static mut MaskUi,
    pub mask_order: &'static layer::MaskOrder,
    pub mask_source: &'static layer::MaskSource,
    pub mask_source_ui: &'static mut MaskSourceUi,
}

impl<'w, 's> MaskQueryItem<'w, 's> {
    fn to_mask_bundle(&self) -> layer::MaskBundle {
        layer::MaskBundle {
            mask: self.mask.clone(),
            mask_source: self.mask_source.clone(),
        }
    }
}

#[derive(SystemParam)]
pub(super) struct Masks<'w, 's> {
    masks: Query<'w, 's, MaskQuery>,
}

impl<'w, 's> Masks<'w, 's> {
    pub fn masks_for_layer(
        &mut self,
        layer: Entity,
    ) -> impl Iterator<Item = MaskQueryItem<'_, '_>> {
        self.masks
            .iter_mut()
            .sort::<&layer::MaskOrder>()
            .filter(move |m| m.child_of.0 == layer)
    }
}

// COMPONENTS

#[derive(Component, Debug, Reflect)]
pub(super) enum HeightMapUi {
    Bitmap {
        // TODO: Enable this after converting to ref-images.
        //
        // #[reflect(ignore)]
        // bitmap: layer::Image,
        transform: Transform2D,
        repeat_mode: layer::BitmapRepeatMode,
        timer: Timer,
    },
    Constant {
        height: f32,
        timer: Timer,
    },
}

impl From<&layer::HeightMap> for HeightMapUi {
    fn from(value: &layer::HeightMap) -> Self {
        match value {
            layer::HeightMap::Bitmap {
                transform,
                repeat_mode,
                ..
            } => Self::Bitmap {
                transform: transform.clone(),
                repeat_mode: *repeat_mode,
                timer: Timer::new(LATENCY, TimerMode::Once),
            },
            layer::HeightMap::Constant(height) => Self::Constant {
                height: *height,
                timer: Timer::new(LATENCY, TimerMode::Once),
            },
        }
    }
}

#[derive(Component, Debug, Reflect)]
pub(super) struct LayerUi {
    name: String,
}

impl From<&layer::Layer> for LayerUi {
    fn from(layer: &layer::Layer) -> Self {
        Self {
            name: layer.name.clone(),
        }
    }
}

#[derive(Component, Debug, Reflect)]
pub(super) struct MaskUi {
    strength: f32,
    timer: Timer,
}

impl From<&layer::Mask> for MaskUi {
    fn from(mask: &layer::Mask) -> Self {
        Self {
            strength: mask.strength,
            timer: Timer::new(LATENCY, TimerMode::Once),
        }
    }
}

#[derive(Component, Debug, Reflect)]
pub(super) enum MaskSourceUi {
    Circle {
        center: Vec2,
        falloff_radius: f32,
        irregularity: f32,
        radius: f32,
        rotation: f32,
        smoothness: f32,
        timer: Timer,
    },
    Square {
        center: Vec2,
        falloff_radius: f32,
        irregularity: f32,
        rotation: f32,
        size: f32,
        smoothness: f32,
        timer: Timer,
    },
}

impl From<&layer::MaskSource> for MaskSourceUi {
    fn from(value: &layer::MaskSource) -> Self {
        match value {
            layer::MaskSource::Circle {
                center,
                falloff_radius,
                irregularity,
                radius,
                rotation,
                smoothness,
                ..
            } => Self::Circle {
                center: *center,
                falloff_radius: *falloff_radius,
                irregularity: *irregularity,
                radius: *radius,
                rotation: *rotation,
                smoothness: *smoothness,
                timer: Timer::new(LATENCY, TimerMode::Once),
            },
            layer::MaskSource::Square {
                center,
                falloff_radius,
                irregularity,
                rotation,
                size,
                smoothness,
                ..
            } => Self::Square {
                center: *center,
                falloff_radius: *falloff_radius,
                irregularity: *irregularity,
                rotation: *rotation,
                size: *size,
                smoothness: *smoothness,
                timer: Timer::new(LATENCY, TimerMode::Once),
            },
        }
    }
}

#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
#[require(layer::Layer)]
pub(super) struct Selected;

// COMMANDS

struct SelectLayer(Entity);

impl Command for SelectLayer {
    fn apply(self, world: &mut World) {
        let already_selected: Vec<Entity> = world
            .query_filtered::<Entity, With<Selected>>()
            .iter_mut(world)
            .collect();
        for entity in already_selected {
            world.entity_mut(entity).remove::<Selected>();
        }
        world.entity_mut(self.0).insert(Selected);
    }
}

// OBSERVERS

fn finalize_add_bitmap_layer_observer(
    closed: On<DialogClosed>,
    mut commands: Commands,
    bitmap_server: Res<BitmapServer>,
    layers_query: Layers,
    mut ui_state_next: ResMut<NextState<UiState>>,
) {
    match &closed.result {
        DialogResult::PickedFile(path) => {
            ui_state_next.set(UiState::Interactive);

            let top_layer_id: Option<crate::id::LayerId> = layers_query
                .layers
                .iter()
                .sort::<&crate::layer::LayerOrder>()
                .last()
                .map(|l| l.layer.id());
            let handle = bitmap_server.load(path, crate::bitmap::LoadMode::Linked);
            debug!("Loading image: {handle:?}.");
            commands.queue(crate::undo::PushAction::from(
                crate::layer::CreateLayerAction::new(
                    crate::layer::LayerBundle::new_bitmap(handle),
                    top_layer_id,
                ),
            ));
        }
        _ => (),
    }
}

// SYSTEMS

/// Add a LayerUi & a HeightMapUi component to each entity with a Layer component added.
fn add_layer_ui_system(
    mut commands: Commands,
    layers: Query<(Entity, &layer::Layer, Option<&layer::HeightMap>), Added<layer::Layer>>,
) {
    for (entity, layer, maybe_height_map) in layers.iter() {
        match maybe_height_map {
            Some(height_map) => {
                commands
                    .entity(entity)
                    .insert((LayerUi::from(layer), HeightMapUi::from(height_map)));
            }
            None => {
                error!("Layer without height map: '{}'.", &layer);
            }
        }
    }
}

/// Add an MaskSourceUi component to each entity with a MaskSource added.
fn add_mask_ui_system(
    mut commands: Commands,
    mask_query: Query<(Entity, &layer::Mask, &layer::MaskSource), Added<layer::MaskSource>>,
) {
    for (entity, mask, mask_source) in mask_query.iter() {
        commands
            .entity(entity)
            .insert((MaskUi::from(mask), MaskSourceUi::from(mask_source)));
    }
}

/// Update HeightMap based on UI changes.
///
/// This [HeightMapUi] to [HeightMap](layer::HeightMap) update is triggered only after a short
/// duration.  When there are frequent updates to HeightMapUi (such as
/// dragging the input) only the last one gets triggered.  See [LATENCY].
///
/// See also [HeightMapConstantUpdateHeightAction](layer::HeightMapConstantUpdateHeightAction).
fn update_height_map_ui_system(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut layers: Query<(&layer::Layer, &layer::HeightMap, &mut HeightMapUi)>,
) {
    for (layer, height_map, mut height_map_ui) in layers.iter_mut() {
        match height_map_ui.as_mut() {
            HeightMapUi::Bitmap {
                transform,
                repeat_mode,
                timer,
            } => {
                if !timer.is_finished() {
                    timer.tick(time.delta());
                    match height_map {
                        layer::HeightMap::Bitmap {
                            transform: original_transform,
                            repeat_mode: original_repeat_mode,
                            ..
                        } => {
                            if timer.just_finished() && transform != original_transform {
                                commands.queue(undo::PushAction::from(
                                    layer::HeightMapBitmapUpdateTransformAction::new(
                                        layer.id(),
                                        original_transform.clone(),
                                        transform.clone(),
                                    ),
                                ));
                            }

                            // TODO: Handle repeat_mode changes.
                        }
                        layer::HeightMap::Constant(_) => unreachable!(),
                    }
                }
            }
            HeightMapUi::Constant { height, timer } => {
                if !timer.is_finished() {
                    timer.tick(time.delta());
                    match height_map {
                        layer::HeightMap::Bitmap { .. } => unreachable!(),

                        layer::HeightMap::Constant(original_height) => {
                            if timer.just_finished() && !original_height.approx_eq(*height, None) {
                                commands.queue(undo::PushAction::from(
                                    layer::HeightMapConstantUpdateHeightAction::new(
                                        layer.id(),
                                        *original_height,
                                        *height,
                                    ),
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Update MaskSource based on UI changes.
fn update_mask_ui_system(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut mask_query: Query<(
        &layer::Mask,
        &layer::MaskSource,
        &mut MaskUi,
        &mut MaskSourceUi,
    )>,
) {
    for (mask, mask_source, mut mask_ui, mut mask_source_ui) in mask_query.iter_mut() {
        if !mask_ui.timer.is_finished() {
            mask_ui.timer.tick(time.delta());
            if mask_ui.timer.just_finished() && !mask.strength.approx_eq(mask_ui.strength, None) {
                commands.queue(undo::PushAction::from(
                    layer::UpdateMaskAction::update_strength(
                        mask.id(),
                        mask.strength,
                        mask_ui.strength,
                    ),
                ));
            }
        }

        match (mask_source, mask_source_ui.as_mut()) {
            (
                layer::MaskSource::Circle {
                    center: original_center,
                    falloff_radius: original_falloff_radius,
                    irregularity: original_irregularity,
                    radius: original_radius,
                    rotation: original_rotation,
                    smoothness: original_smoothness,
                    ..
                },
                &mut MaskSourceUi::Circle {
                    ref center,
                    ref falloff_radius,
                    ref irregularity,
                    ref radius,
                    ref rotation,
                    ref smoothness,
                    ref mut timer,
                },
            ) => {
                if !timer.is_finished() {
                    timer.tick(time.delta());
                    if timer.just_finished()
                        && !original_center.distance(*center).approx_eq(0.0, None)
                    {
                        commands.queue(undo::PushAction::from(
                            layer::UpdateMaskSourceAction::update_center(
                                mask.id(),
                                *original_center,
                                *center,
                            ),
                        ));
                    }
                    if timer.just_finished()
                        && !original_falloff_radius.approx_eq(*falloff_radius, None)
                    {
                        commands.queue(undo::PushAction::from(
                            layer::UpdateMaskSourceAction::update_falloff_radius(
                                mask.id(),
                                *original_falloff_radius,
                                *falloff_radius,
                            ),
                        ));
                    }
                    if timer.just_finished()
                        && !original_irregularity.approx_eq(*irregularity, None)
                    {
                        commands.queue(undo::PushAction::from(
                            layer::UpdateMaskSourceAction::update_irregularity(
                                mask.id(),
                                *original_irregularity,
                                *irregularity,
                            ),
                        ));
                    }
                    if timer.just_finished() && !original_radius.approx_eq(*radius, None) {
                        commands.queue(undo::PushAction::from(
                            layer::UpdateMaskSourceAction::update_radius(
                                mask.id(),
                                *original_radius,
                                *radius,
                            ),
                        ));
                    }
                    if timer.just_finished() && !original_rotation.approx_eq(*rotation, None) {
                        commands.queue(undo::PushAction::from(
                            layer::UpdateMaskSourceAction::update_rotation(
                                mask.id(),
                                *original_rotation,
                                *rotation,
                            ),
                        ));
                    }
                    if timer.just_finished() && !original_smoothness.approx_eq(*smoothness, None) {
                        commands.queue(undo::PushAction::from(
                            layer::UpdateMaskSourceAction::update_smoothness(
                                mask.id(),
                                *original_smoothness,
                                *smoothness,
                            ),
                        ));
                    }
                }
            }
            (layer::MaskSource::Circle { .. }, _) => unreachable!(),
            (
                layer::MaskSource::Square {
                    center: original_center,
                    falloff_radius: original_falloff_radius,
                    irregularity: original_irregularity,
                    rotation: original_rotation,
                    size: original_size,
                    smoothness: original_smoothness,
                    ..
                },
                &mut MaskSourceUi::Square {
                    ref center,
                    ref falloff_radius,
                    ref irregularity,
                    ref rotation,
                    ref size,
                    ref smoothness,
                    ref mut timer,
                },
            ) => {
                if !timer.is_finished() {
                    timer.tick(time.delta());

                    if timer.just_finished()
                        && !original_center.distance(*center).approx_eq(0.0, None)
                    {
                        commands.queue(undo::PushAction::from(
                            layer::UpdateMaskSourceAction::update_center(
                                mask.id(),
                                *original_center,
                                *center,
                            ),
                        ));
                    }
                    if timer.just_finished()
                        && !original_falloff_radius.approx_eq(*falloff_radius, None)
                    {
                        commands.queue(undo::PushAction::from(
                            layer::UpdateMaskSourceAction::update_falloff_radius(
                                mask.id(),
                                *original_falloff_radius,
                                *falloff_radius,
                            ),
                        ));
                    }
                    if timer.just_finished()
                        && !original_irregularity.approx_eq(*irregularity, None)
                    {
                        commands.queue(undo::PushAction::from(
                            layer::UpdateMaskSourceAction::update_irregularity(
                                mask.id(),
                                *original_irregularity,
                                *irregularity,
                            ),
                        ));
                    }
                    if timer.just_finished() && !original_size.approx_eq(*size, None) {
                        commands.queue(undo::PushAction::from(
                            layer::UpdateMaskSourceAction::update_size(
                                mask.id(),
                                *original_size,
                                *size,
                            ),
                        ));
                    }
                    if timer.just_finished() && !original_rotation.approx_eq(*rotation, None) {
                        commands.queue(undo::PushAction::from(
                            layer::UpdateMaskSourceAction::update_rotation(
                                mask.id(),
                                *original_rotation,
                                *rotation,
                            ),
                        ));
                    }
                    if timer.just_finished() && !original_smoothness.approx_eq(*smoothness, None) {
                        commands.queue(undo::PushAction::from(
                            layer::UpdateMaskSourceAction::update_smoothness(
                                mask.id(),
                                *original_smoothness,
                                *smoothness,
                            ),
                        ));
                    }
                }
            }
            (layer::MaskSource::Square { .. }, _) => unreachable!(),
        }
    }
}

/// Update HeightMapUi based on changes to HeightMap.
///
/// This gets triggered when undo/redo changes [HeightMap](layer::HeightMap).
fn reset_height_map_ui_system(
    mut layers: Query<(&layer::HeightMap, &mut HeightMapUi), Changed<layer::HeightMap>>,
) {
    for (height_map, mut height_map_ui) in layers.iter_mut() {
        match height_map {
            layer::HeightMap::Bitmap {
                // bitmap: original_bitmap,
                transform: original_transform,
                repeat_mode: original_repeat_mode,
                ..
            } => {
                if let HeightMapUi::Bitmap {
                    transform,
                    repeat_mode,
                    timer,
                } = height_map_ui.as_mut()
                {
                    *transform = original_transform.clone();
                    *repeat_mode = *original_repeat_mode;
                    timer.pause();
                } else {
                    unreachable!()
                }
            }
            layer::HeightMap::Constant(original_height) => {
                if let HeightMapUi::Constant { height, timer } = height_map_ui.as_mut() {
                    *height = *original_height;
                    timer.pause();
                } else {
                    unreachable!();
                }
            }
        }
    }
}

/// Update MaskSourceUi based on changes to MaskSource.
///
/// This gets triggered when undo/redo changes [MaskSource](layer::MaskSource).
fn reset_mask_ui_system(
    mut mask_query: Query<(&layer::MaskSource, &mut MaskSourceUi), Changed<layer::MaskSource>>,
) {
    for (mask_source, mut mask_source_ui) in mask_query.iter_mut() {
        match (mask_source, mask_source_ui.as_mut()) {
            (
                layer::MaskSource::Circle {
                    center: original_center,
                    falloff_radius: original_falloff_radius,
                    irregularity: original_irregularity,
                    radius: original_radius,
                    rotation: original_rotation,
                    smoothness: original_smoothness,
                    ..
                },
                MaskSourceUi::Circle {
                    center,
                    falloff_radius,
                    irregularity,
                    radius,
                    rotation,
                    smoothness,
                    timer,
                },
            ) => {
                *center = *original_center;
                *falloff_radius = *original_falloff_radius;
                *irregularity = *original_irregularity;
                *radius = *original_radius;
                *rotation = *original_rotation;
                *smoothness = *original_smoothness;
                timer.pause();
            }
            (layer::MaskSource::Circle { .. }, _) => unreachable!(),
            (
                layer::MaskSource::Square {
                    center: original_center,
                    falloff_radius: original_falloff_radius,
                    irregularity: original_irregularity,
                    rotation: original_rotation,
                    size: original_size,
                    smoothness: original_smoothness,
                    ..
                },
                MaskSourceUi::Square {
                    center,
                    falloff_radius,
                    irregularity,
                    rotation,
                    size,
                    smoothness,
                    timer,
                },
            ) => {
                *center = *original_center;
                *falloff_radius = *original_falloff_radius;
                *irregularity = *original_irregularity;
                *rotation = *original_rotation;
                *size = *original_size;
                *smoothness = *original_smoothness;
                timer.pause();
            }
            (layer::MaskSource::Square { .. }, _) => unreachable!(),
        }
    }
}

// LIB

fn draw_ui_for_layer_common_bottom(
    commands: &mut Commands,
    masks_query: &mut Masks,
    layer_query_item: &LayerQueryItem,
    ui: &mut egui::Ui,
) {
    let frame = egui::containers::Frame::group(ui.style());
    frame.show(ui, |ui| {
        ui.heading("Masks");

        // This is a tuple of previous mask id & `MaskQueryItem` in reverse
        // MaskOrder.
        //
        // We have reversed iteration order of masks, so that masks with
        // highter MaskOrder appear above then ones with lower MaskOrder.
        let mut masks_in_reverse_order: Vec<(Option<MaskId>, MaskQueryItem)> = {
            let mut masks: Vec<_> = masks_query
                .masks_for_layer(layer_query_item.entity)
                .fold(
                    (None::<MaskId>, vec![]),
                    |(previous_mask_id, mut acc), m| {
                        let new_previous_mask_id = m.mask.id().clone();
                        acc.push((previous_mask_id, m));
                        (Some(new_previous_mask_id), acc)
                    },
                )
                .1;
            masks.reverse();
            masks
        };
        let topmost_mask_id: Option<MaskId> = masks_in_reverse_order
            .first()
            .map(|(_, m)| m.mask.id().clone());

        if ui.button("Add circle mask").clicked() {
            let mask_bundle: layer::MaskBundle = layer::MaskBundle {
                mask: layer::Mask::default(),
                mask_source: layer::MaskSource::circle(),
            };
            let layer_id: LayerId = layer_query_item.layer.id();
            commands.queue(undo::PushAction::from(layer::CreateMaskAction::new(
                mask_bundle,
                layer_id,
                topmost_mask_id,
            )));
        }

        if ui.button("Add square mask").clicked() {
            let mask_bundle: layer::MaskBundle = layer::MaskBundle {
                mask: layer::Mask::default(),
                mask_source: layer::MaskSource::square(),
            };
            let layer_id: LayerId = layer_query_item.layer.id();
            commands.queue(undo::PushAction::from(layer::CreateMaskAction::new(
                mask_bundle,
                layer_id,
                topmost_mask_id,
            )));
        }

        for (previous_mask_id, m) in masks_in_reverse_order.iter_mut() {
            draw_ui_for_mask(
                commands,
                layer_query_item.layer.id(),
                m,
                *previous_mask_id,
                ui,
            );
        }
    });
}

fn draw_ui_for_layer_common_top(
    commands: &mut Commands,
    ui: &mut egui::Ui,
    layer_query_item: &mut LayerQueryItem,
    masks_query: &mut Masks,
    parent_layer_id: Option<LayerId>,
) {
    const LAYER_NAME_CHAR_LIMIT: usize = 20;
    {
        let widget = egui::widgets::TextEdit::singleline(&mut layer_query_item.layer_ui.name)
            .char_limit(LAYER_NAME_CHAR_LIMIT);
        let mut output = widget.show(ui);
        // Select everything when the widget first gains focus.
        if output.response.gained_focus() {
            output
                .state
                .cursor
                .set_char_range(Some(egui::text_selection::CCursorRange::two(
                    egui::text::CCursor::new(0),
                    egui::text::CCursor::new(layer_query_item.layer_ui.name.len()),
                )));
            output.state.store(ui.ctx(), output.response.id);
        }
        if output.response.lost_focus()
            && layer_query_item.layer_ui.name != layer_query_item.layer.name
        {
            commands.queue(undo::PushAction::from(layer::RenameLayerAction::new(
                layer_query_item.layer.id(),
                &layer_query_item.layer.name,
                &layer_query_item.layer_ui.name,
            )));
        }
    }
    {
        ui.horizontal(|ui| {
            {
                let mut layer_preview: bool = layer_query_item.layer.enable_preview;
                if ui.toggle_value(&mut layer_preview, "Preview").changed()
                    && layer_preview != layer_query_item.layer.enable_preview
                {
                    commands.queue(undo::PushAction::from(
                        layer::UpdateLayerAction::toggle_enable_preview(layer_query_item.layer),
                    ));
                }
            }
            {
                let mut layer_baking: bool = layer_query_item.layer.enable_baking;
                if ui.toggle_value(&mut layer_baking, "Bake").changed()
                    && layer_baking != layer_query_item.layer.enable_baking
                {
                    commands.queue(undo::PushAction::from(
                        layer::UpdateLayerAction::toggle_enable_baking(layer_query_item.layer),
                    ));
                }
            }
            ui.separator();
            if ui.button("Delete").clicked() {
                let masks: Vec<layer::MaskBundle> = masks_query
                    .masks_for_layer(layer_query_item.entity)
                    .map(|m| m.to_mask_bundle())
                    .collect();

                commands.queue(undo::PushAction::from(layer::DeleteLayerAction::new(
                    layer_query_item.to_layer_bundle(),
                    masks,
                    parent_layer_id,
                )))
            }
        });
    };
}

fn draw_ui_for_bitmap_layer(
    ui: &mut egui::Ui,
    bitmap_server: &BitmapServer,
    height_map: &HeightMap,
    height_map_ui: &mut HeightMapUi,
) {
    match height_map_ui {
        HeightMapUi::Bitmap {
            transform,
            repeat_mode,
            timer,
        } => {
            let size = {
                if let HeightMap::Bitmap {
                    bitmap_handle,
                    transform,
                    ..
                } = height_map
                {
                    bitmap_server.get(bitmap_handle).unwrap().size().as_vec2() * transform.scale
                } else {
                    unreachable!()
                }
            };

            if let Some(new_transform) = draw_ui_for_transform_2d(ui, transform) {
                *transform = new_transform;
                timer.unpause();
                timer.reset();
            }
            ui.horizontal(|ui| {
                ui.label(format!("Size: {:.2}×{:.2}", size.x, size.y));
            });
        }
        HeightMapUi::Constant { .. } => unreachable!(),
    }
}

fn draw_ui_for_constant_layer(ui: &mut egui::Ui, height_map_ui: &mut HeightMapUi) {
    match height_map_ui {
        HeightMapUi::Bitmap { .. } => unreachable!(),
        HeightMapUi::Constant { height, timer } => {
            ui.horizontal(|ui| {
                ui.label("Height:");

                ui.with_layout(
                    egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                    |ui| {
                        if let Some(new_height) =
                            draw_ui_editable_f32(Some(layer::HEIGHT_RANGE), None, ui, *height)
                        {
                            *height = new_height;
                            timer.unpause();
                            timer.reset();
                        }
                    },
                );
            });
        }
    }
}

/// Draw the UI for the stack of layers in the project.
pub fn draw_ui_for_layers(
    bitmap_server: &BitmapServer,
    commands: &mut Commands,
    theme_colors: &theme::ThemeColors,
    ui: &mut egui::Ui,
    layers_query: &mut Layers,
    masks_query: &mut Masks,
) {
    egui::containers::ScrollArea::vertical().show(ui, |ui| {
        ui.heading("Layers");

        let top_layer_id: Option<LayerId> = layers_query
            .layers
            .iter()
            .sort::<&layer::LayerOrder>()
            .last()
            .map(|l| l.layer.id());

        if ui.button("Test").clicked() {
            info!("Adding bitmap layer.");
            commands.queue(|world: &mut World| {
                let entity = LoadImageDialog::spawn(world);
                world
                    .commands()
                    .entity(entity)
                    .observe(finalize_add_bitmap_layer_observer);
            });
        }

        if ui.button("New Layer").clicked() {
            commands.queue(undo::PushAction::from(layer::CreateLayerAction::new(
                layer::LayerBundle::new_constant(),
                top_layer_id,
            )));
        }
        {
            let layer_ids: Vec<LayerId> = layers_query
                .iter()
                .sort::<&layer::LayerOrder>()
                .rev()
                .map(|l| l.layer.id())
                .collect();
            // We need to iterate layers in reverse order to place the topmost
            // (last applied) layer on top.
            for (idx, mut l) in layers_query
                .iter_mut()
                .sort::<&layer::LayerOrder>()
                .rev()
                .enumerate()
            {
                let parent_layer_id = layer_ids.get(idx + 1).cloned();
                draw_ui_for_layer(
                    bitmap_server,
                    commands,
                    theme_colors,
                    ui,
                    masks_query,
                    &mut l,
                    parent_layer_id,
                );
            }
        }
    });
}

fn draw_ui_for_layer(
    bitmap_server: &BitmapServer,
    commands: &mut Commands,
    theme_colors: &theme::ThemeColors,
    ui: &mut egui::Ui,
    masks_query: &mut Masks,
    layer_query_item: &mut LayerQueryItem,
    parent_layer_id: Option<LayerId>,
) {
    let mut frame = egui::containers::Frame::group(ui.style());
    if layer_query_item.is_selected {
        frame = frame.fill(ui.style().visuals.widgets.noninteractive.weak_bg_fill);
    }
    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            let height_id = ui.id().with("height").with(layer_query_item.layer.id());
            {
                let height: f32 = ui.data(|map| map.get_temp(height_id).unwrap_or(32.0));
                let (response, painter) = ui.allocate_painter(
                    [LAYER_SELECTION_BOX_WIDTH, height].into(),
                    egui::Sense::click(),
                );
                painter.rect_filled(
                    response.rect,
                    4.0,
                    if layer_query_item.is_selected {
                        theme_colors.secondary_color.to_color32()
                    } else {
                        theme_colors.bg_alt_color.to_color32()
                    },
                );

                if response.clicked() && !layer_query_item.is_selected {
                    commands.queue(SelectLayer(layer_query_item.entity));
                }
            }
            let actual_height: f32 = ui
                .vertical_centered_justified(|ui| {
                    match layer_query_item.height_map_ui.as_ref() {
                        HeightMapUi::Bitmap { .. } => {
                            draw_ui_for_layer_common_top(
                                commands,
                                ui,
                                layer_query_item,
                                masks_query,
                                parent_layer_id,
                            );
                            ui.separator();
                            draw_ui_for_bitmap_layer(
                                ui,
                                &bitmap_server,
                                layer_query_item.height_map,
                                layer_query_item.height_map_ui.as_mut(),
                            );
                            ui.separator();
                            draw_ui_for_layer_common_bottom(
                                commands,
                                masks_query,
                                layer_query_item,
                                ui,
                            );
                        }
                        HeightMapUi::Constant { .. } => {
                            draw_ui_for_layer_common_top(
                                commands,
                                ui,
                                layer_query_item,
                                masks_query,
                                parent_layer_id,
                            );
                            ui.separator();
                            draw_ui_for_constant_layer(ui, layer_query_item.height_map_ui.as_mut());
                            ui.separator();
                            draw_ui_for_layer_common_bottom(
                                commands,
                                masks_query,
                                layer_query_item,
                                ui,
                            );
                        }
                    };
                })
                .response
                .rect
                .height();

            // Save the actual height for the next frame.
            ui.data_mut(|map| map.insert_temp(height_id, actual_height));
        });
    });
}

fn draw_ui_for_mask(
    commands: &mut Commands,
    layer_id: LayerId,
    mask: &mut <MaskQuery as QueryData>::Item<'_, '_>,
    previous_mask_id: Option<MaskId>,
    ui: &mut egui::Ui,
) {
    let frame = egui::containers::Frame::group(ui.style());
    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(format!("Mask: {:?}", mask.entity));
            let mut is_enabled: bool = mask.mask.is_enabled;
            if ui.toggle_value(&mut is_enabled, "Enabled").changed()
                && is_enabled != mask.mask.is_enabled
            {
                commands.queue(undo::PushAction::from(
                    layer::UpdateMaskAction::toggle_enabled(mask.mask.id(), is_enabled),
                ));
            }
            if ui.button("Delete").clicked() {
                let mask_bundle = layer::MaskBundle {
                    mask: mask.mask.clone(),
                    mask_source: mask.mask_source.clone(),
                };
                commands.queue(undo::PushAction::from(layer::DeleteMaskAction::new(
                    mask_bundle,
                    layer_id,
                    previous_mask_id,
                )));
            }
        });

        ui.horizontal(|ui| {
            let MaskUi {
                ref mut strength,
                ref mut timer,
            } = *mask.mask_ui;
            ui.label("Strength:");
            if let Some(new_strength) = draw_ui_editable_f32(
                Some(ZERO_TO_ONE),
                Some(ZERO_TO_ONE_INCREMENT),
                ui,
                mask.mask.strength,
            ) {
                debug!("Opacity changed to {}.", new_strength);
                *strength = new_strength;
                timer.unpause();
                timer.reset();
            }
        });

        ui.separator();

        match *mask.mask_source_ui {
            MaskSourceUi::Circle {
                ref mut center,
                ref mut falloff_radius,
                ref mut irregularity,
                ref mut radius,
                ref mut rotation,
                ref mut smoothness,
                ref mut timer,
            } => {
                ui.horizontal(|ui| {
                    ui.label("Center:");
                    if let Some(new_x) = draw_ui_editable_f32(None, None, ui, center.x) {
                        center.x = new_x;
                        timer.unpause();
                        timer.reset();
                    }
                    if let Some(new_y) = draw_ui_editable_f32(None, None, ui, center.y) {
                        center.y = new_y;
                        timer.unpause();
                        timer.reset();
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Radius:");
                    if let Some(new_radius) =
                        draw_ui_editable_f32(Some(ZERO_TO_POSITIVE_INFINITY), None, ui, *radius)
                    {
                        *radius = new_radius;
                        timer.unpause();
                        timer.reset();
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Falloff Radius:");
                    if let Some(new_falloff_radius) = draw_ui_editable_f32(
                        Some(ZERO_TO_POSITIVE_INFINITY),
                        None,
                        ui,
                        *falloff_radius,
                    ) {
                        *falloff_radius = new_falloff_radius;
                        timer.unpause();
                        timer.reset();
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Smoothness:");
                    if let Some(new_smoothness) = draw_ui_editable_f32(
                        Some(ZERO_TO_ONE),
                        Some(ZERO_TO_ONE_INCREMENT),
                        ui,
                        *smoothness,
                    ) {
                        *smoothness = new_smoothness;
                        timer.unpause();
                        timer.reset();
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Rotation:");
                    if let Some(new_rotation) = draw_ui_editable_f32(
                        Some(ZERO_TO_ONE),
                        Some(ZERO_TO_ONE_INCREMENT),
                        ui,
                        *rotation,
                    ) {
                        *rotation = new_rotation;
                        timer.unpause();
                        timer.reset();
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Irregularity:");
                    if let Some(new_irregularity) = draw_ui_editable_f32(
                        Some(MINUS_ONE_TO_ONE),
                        Some(ZERO_TO_ONE_INCREMENT),
                        ui,
                        *irregularity,
                    ) {
                        *irregularity = new_irregularity;
                        timer.unpause();
                        timer.reset();
                    }
                });
            }
            MaskSourceUi::Square {
                ref mut center,
                ref mut falloff_radius,
                ref mut irregularity,
                ref mut rotation,
                ref mut size,
                ref mut smoothness,
                ref mut timer,
            } => {
                ui.horizontal(|ui| {
                    ui.label("Center:");
                    if let Some(new_x) = draw_ui_editable_f32(None, None, ui, center.x) {
                        center.x = new_x;
                        timer.unpause();
                        timer.reset();
                    }
                    if let Some(new_y) = draw_ui_editable_f32(None, None, ui, center.y) {
                        center.y = new_y;
                        timer.unpause();
                        timer.reset();
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Size:");
                    if let Some(new_size) =
                        draw_ui_editable_f32(Some(ZERO_TO_POSITIVE_INFINITY), None, ui, *size)
                    {
                        *size = new_size;
                        timer.unpause();
                        timer.reset();
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Falloff Radius:");
                    if let Some(new_falloff_radius) = draw_ui_editable_f32(
                        Some(ZERO_TO_POSITIVE_INFINITY),
                        None,
                        ui,
                        *falloff_radius,
                    ) {
                        *falloff_radius = new_falloff_radius;
                        timer.unpause();
                        timer.reset();
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Smoothness:");
                    if let Some(new_smoothness) = draw_ui_editable_f32(
                        Some(ZERO_TO_ONE),
                        Some(ZERO_TO_ONE_INCREMENT),
                        ui,
                        *smoothness,
                    ) {
                        *smoothness = new_smoothness;
                        timer.unpause();
                        timer.reset();
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Rotation:");
                    if let Some(new_rotation) = draw_ui_editable_f32(
                        Some(ZERO_TO_ONE),
                        Some(ZERO_TO_ONE_INCREMENT),
                        ui,
                        *rotation,
                    ) {
                        *rotation = new_rotation;
                        timer.unpause();
                        timer.reset();
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Irregularity:");
                    if let Some(new_irregularity) = draw_ui_editable_f32(
                        Some(MINUS_ONE_TO_ONE),
                        Some(ZERO_TO_ONE_INCREMENT),
                        ui,
                        *irregularity,
                    ) {
                        *irregularity = new_irregularity;
                        timer.unpause();
                        timer.reset();
                    }
                });
            }
        }

        if previous_mask_id.is_some() {
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Composition Mode:");

                let mut mode_edited = mask.mask.composition_mode;
                for mode in layer::MaskCompositionMode::ITEMS.into_iter() {
                    ui.selectable_value(&mut mode_edited, mode, mode.to_string());
                }
                if mode_edited != mask.mask.composition_mode {
                    commands.queue(undo::PushAction::from(
                        layer::UpdateMaskAction::change_composition_mode(
                            mask.mask.id(),
                            mask.mask.composition_mode,
                            mode_edited,
                        ),
                    ));
                }
            });
        }
    });
}

fn draw_ui_for_transform_2d(ui: &mut egui::Ui, transform: &Transform2D) -> Option<Transform2D> {
    let new_translation = ui
        .horizontal(|ui| {
            ui.label("Translation:");
            let translation_x = draw_ui_editable_f32(None, None, ui, transform.translation.x);
            let translation_y = draw_ui_editable_f32(None, None, ui, transform.translation.y);

            match (translation_x, translation_y) {
                (Some(translation_x), Some(translation_y)) => {
                    Some(Vec2::new(translation_x, translation_y))
                }
                (Some(translation_x), None) => {
                    Some(Vec2::new(translation_x, transform.translation.y))
                }
                (None, Some(translation_y)) => {
                    Some(Vec2::new(transform.translation.x, translation_y))
                }
                (None, None) => None,
            }
        })
        .inner;

    let new_rotation = ui
        .horizontal(|ui| {
            ui.label("Rotation:");
            draw_ui_editable_f32(None, None, ui, transform.rotation)
        })
        .inner;

    let new_scale = ui
        .horizontal(|ui| {
            ui.label("Scale:");
            draw_ui_editable_f32(Some(SCALE_LIMITS), None, ui, transform.scale)
        })
        .inner;

    if new_scale.is_some() || new_translation.is_some() || new_rotation.is_some() {
        Some(Transform2D {
            scale: new_scale.unwrap_or(transform.scale),
            translation: new_translation.unwrap_or(transform.translation),
            rotation: new_rotation.unwrap_or(transform.rotation),
        })
    } else {
        None
    }
}
