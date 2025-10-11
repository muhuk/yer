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

use std::ops::RangeInclusive;
use std::time::Duration;

use bevy::ecs::{query::QueryData, system::SystemParam};
use bevy::prelude::*;
use bevy_egui::egui;

use crate::id::{LayerId, MaskId};
use crate::layer;
use crate::math::{approx_eq, ONE_IN_TEN_THOUSAND};
use crate::theme;
use crate::undo;

use super::egui_ext::{draw_ui_editable_f32, ToColor32};

const LATENCY: Duration = Duration::from_millis(100);
const LAYER_SELECTION_BOX_WIDTH: f32 = 24.0f32;
const MINUS_ONE_TO_ONE: RangeInclusive<f32> = -1.0..=1.0;
const ZERO_TO_POSITIVE_INFINITY: RangeInclusive<f32> = 0.0..=f32::INFINITY;
const ZERO_TO_ONE: RangeInclusive<f32> = 0.0..=1.0;
const ZERO_TO_ONE_INCREMENT: f32 = 0.025;

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
    pub layer: &'static layer::Layer,
    pub layer_order: &'static layer::LayerOrder,
    pub layer_ui: &'static mut LayerUi,
    pub height_map_ui: &'static mut HeightMapUi,
    pub is_selected: Has<Selected>,
}

#[derive(Deref, DerefMut, SystemParam)]
pub(super) struct Layers<'w, 's> {
    #[deref]
    layers: Query<'w, 's, LayerQuery>,
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

#[derive(SystemParam)]
pub(super) struct Masks<'w, 's> {
    masks: Query<'w, 's, MaskQuery>,
}

impl<'w, 's> Masks<'w, 's> {
    pub fn masks_for_layer(&mut self, layer: Entity) -> impl Iterator<Item = MaskQueryItem<'_>> {
        self.masks
            .iter_mut()
            .sort::<&layer::MaskOrder>()
            .rev()
            .filter(move |m| m.child_of.0 == layer)
    }
}

// COMPONENTS

#[derive(Component, Debug, Reflect)]
pub(super) enum HeightMapUi {
    Constant { height: f32, timer: Timer },
}

impl From<&layer::HeightMap> for HeightMapUi {
    fn from(value: &layer::HeightMap) -> Self {
        match value {
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
        timer: Timer,
    },
    Square {
        center: Vec2,
        falloff_radius: f32,
        irregularity: f32,
        rotation: f32,
        size: f32,
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
            } => Self::Circle {
                center: *center,
                falloff_radius: *falloff_radius,
                irregularity: *irregularity,
                radius: *radius,
                rotation: *rotation,
                timer: Timer::new(LATENCY, TimerMode::Once),
            },
            layer::MaskSource::Square {
                center,
                falloff_radius,
                irregularity,
                rotation,
                size,
            } => Self::Square {
                center: *center,
                falloff_radius: *falloff_radius,
                irregularity: *irregularity,
                rotation: *rotation,
                size: *size,
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
        match *height_map_ui {
            HeightMapUi::Constant {
                height,
                ref mut timer,
            } => {
                if !timer.finished() {
                    timer.tick(time.delta());
                    let layer::HeightMap::Constant(original_height) = height_map;
                    if timer.just_finished()
                        && !approx_eq(*original_height, height, ONE_IN_TEN_THOUSAND)
                    {
                        commands.queue(undo::PushAction::from(
                            layer::HeightMapConstantUpdateHeightAction::new(
                                layer.id(),
                                *original_height,
                                height,
                            ),
                        ));
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
        if !mask_ui.timer.finished() {
            mask_ui.timer.tick(time.delta());
            if mask_ui.timer.just_finished()
                && !approx_eq(mask.strength, mask_ui.strength, ONE_IN_TEN_THOUSAND)
            {
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
                },
                &mut MaskSourceUi::Circle {
                    ref center,
                    ref falloff_radius,
                    ref irregularity,
                    ref radius,
                    ref rotation,
                    ref mut timer,
                },
            ) => {
                if !timer.finished() {
                    timer.tick(time.delta());
                    if timer.just_finished()
                        && !approx_eq(original_center.distance(*center), 0.0, ONE_IN_TEN_THOUSAND)
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
                        && !approx_eq(
                            *original_falloff_radius,
                            *falloff_radius,
                            ONE_IN_TEN_THOUSAND,
                        )
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
                        && !approx_eq(*original_irregularity, *irregularity, ONE_IN_TEN_THOUSAND)
                    {
                        commands.queue(undo::PushAction::from(
                            layer::UpdateMaskSourceAction::update_irregularity(
                                mask.id(),
                                *original_irregularity,
                                *irregularity,
                            ),
                        ));
                    }
                    if timer.just_finished()
                        && !approx_eq(*original_radius, *radius, ONE_IN_TEN_THOUSAND)
                    {
                        commands.queue(undo::PushAction::from(
                            layer::UpdateMaskSourceAction::update_radius(
                                mask.id(),
                                *original_radius,
                                *radius,
                            ),
                        ));
                    }
                    if timer.just_finished()
                        && !approx_eq(*original_rotation, *rotation, ONE_IN_TEN_THOUSAND)
                    {
                        commands.queue(undo::PushAction::from(
                            layer::UpdateMaskSourceAction::update_rotation(
                                mask.id(),
                                *original_rotation,
                                *rotation,
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
                },
                &mut MaskSourceUi::Square {
                    ref center,
                    ref falloff_radius,
                    ref irregularity,
                    ref rotation,
                    ref size,
                    ref mut timer,
                },
            ) => {
                if !timer.finished() {
                    timer.tick(time.delta());

                    if timer.just_finished()
                        && !approx_eq(original_center.distance(*center), 0.0, ONE_IN_TEN_THOUSAND)
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
                        && !approx_eq(
                            *original_falloff_radius,
                            *falloff_radius,
                            ONE_IN_TEN_THOUSAND,
                        )
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
                        && !approx_eq(*original_irregularity, *irregularity, ONE_IN_TEN_THOUSAND)
                    {
                        commands.queue(undo::PushAction::from(
                            layer::UpdateMaskSourceAction::update_irregularity(
                                mask.id(),
                                *original_irregularity,
                                *irregularity,
                            ),
                        ));
                    }
                    if timer.just_finished()
                        && !approx_eq(*original_size, *size, ONE_IN_TEN_THOUSAND)
                    {
                        commands.queue(undo::PushAction::from(
                            layer::UpdateMaskSourceAction::update_size(
                                mask.id(),
                                *original_size,
                                *size,
                            ),
                        ));
                    }
                    if timer.just_finished()
                        && !approx_eq(*original_rotation, *rotation, ONE_IN_TEN_THOUSAND)
                    {
                        commands.queue(undo::PushAction::from(
                            layer::UpdateMaskSourceAction::update_rotation(
                                mask.id(),
                                *original_rotation,
                                *rotation,
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
            layer::HeightMap::Constant(original_height) => {
                let HeightMapUi::Constant {
                    ref mut height,
                    ref mut timer,
                } = *height_map_ui;
                *height = *original_height;
                timer.pause();
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
                },
                MaskSourceUi::Circle {
                    center,
                    falloff_radius,
                    irregularity,
                    radius,
                    rotation,
                    timer,
                },
            ) => {
                *center = *original_center;
                *falloff_radius = *original_falloff_radius;
                *irregularity = *original_irregularity;
                *radius = *original_radius;
                *rotation = *original_rotation;
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
                },
                MaskSourceUi::Square {
                    center,
                    falloff_radius,
                    irregularity,
                    rotation,
                    size,
                    timer,
                },
            ) => {
                *center = *original_center;
                *falloff_radius = *original_falloff_radius;
                *irregularity = *original_irregularity;
                *rotation = *original_rotation;
                *size = *original_size;
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
    entity: Entity,
    layer: &layer::Layer,
    ui: &mut egui::Ui,
) {
    let frame = egui::containers::Frame::group(ui.style());
    frame.show(ui, |ui| {
        ui.heading("Masks");

        let mask_ids: Vec<MaskId> = masks_query
            .masks_for_layer(entity)
            .map(|m| m.mask.id())
            .collect();

        if ui.button("Add circle mask").clicked() {
            let mask_bundle: layer::MaskBundle = layer::MaskBundle {
                mask: layer::Mask::default(),
                mask_source: layer::MaskSource::circle(),
            };
            let layer_id: LayerId = layer.id();
            let previous_mask_id: Option<MaskId> = mask_ids.first().cloned();
            commands.queue(undo::PushAction::from(layer::CreateMaskAction::new(
                mask_bundle,
                layer_id,
                previous_mask_id,
            )));
        }

        if ui.button("Add square mask").clicked() {
            let mask_bundle: layer::MaskBundle = layer::MaskBundle {
                mask: layer::Mask::default(),
                mask_source: layer::MaskSource::square(),
            };
            let layer_id: LayerId = layer.id();
            let previous_mask_id: Option<MaskId> = mask_ids.first().cloned();
            commands.queue(undo::PushAction::from(layer::CreateMaskAction::new(
                mask_bundle,
                layer_id,
                previous_mask_id,
            )));
        }

        for (idx, mut m) in masks_query.masks_for_layer(entity).enumerate() {
            let previous_mask_id: Option<MaskId> = mask_ids.get(idx + 1).cloned();
            draw_ui_for_mask(commands, layer.id(), &mut m, previous_mask_id, ui);
        }
    });
}

fn draw_ui_for_layer_common_top(
    commands: &mut Commands,
    ui: &mut egui::Ui,
    layer: &layer::Layer,
    layer_ui: &mut LayerUi,
    parent_layer_id: Option<LayerId>,
) {
    const LAYER_NAME_CHAR_LIMIT: usize = 20;
    {
        let widget = egui::widgets::TextEdit::singleline(&mut layer_ui.name)
            .char_limit(LAYER_NAME_CHAR_LIMIT);
        let mut output = widget.show(ui);
        // Select everything when the widget first gains focus.
        if output.response.gained_focus() {
            output
                .state
                .cursor
                .set_char_range(Some(egui::text_selection::CCursorRange::two(
                    egui::text::CCursor::new(0),
                    egui::text::CCursor::new(layer_ui.name.len()),
                )));
            output.state.store(ui.ctx(), output.response.id);
        }
        if output.response.lost_focus() && layer_ui.name != layer.name {
            commands.queue(undo::PushAction::from(layer::RenameLayerAction::new(
                layer.id(),
                &layer.name,
                &layer_ui.name,
            )));
        }
    }
    {
        ui.horizontal(|ui| {
            {
                let mut layer_preview: bool = layer.enable_preview;
                if ui.toggle_value(&mut layer_preview, "Preview").changed()
                    && layer_preview != layer.enable_preview
                {
                    commands.queue(undo::PushAction::from(
                        layer::UpdateLayerAction::toggle_enable_preview(layer),
                    ));
                }
            }
            {
                let mut layer_baking: bool = layer.enable_baking;
                if ui.toggle_value(&mut layer_baking, "Bake").changed()
                    && layer_baking != layer.enable_baking
                {
                    commands.queue(undo::PushAction::from(
                        layer::UpdateLayerAction::toggle_enable_baking(layer),
                    ));
                }
            }
            ui.separator();
            if ui.button("Delete").clicked() {
                commands.queue(undo::PushAction::from(layer::DeleteLayerAction::new(
                    layer.clone(),
                    parent_layer_id,
                )))
            }
        });
    };
}

fn draw_ui_for_constant_layer(ui: &mut egui::Ui, height_map_ui: &mut HeightMapUi) {
    ui.horizontal(|ui| {
        ui.label("Height:");
        match height_map_ui {
            HeightMapUi::Constant { height, timer } => {
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
            }
        }
    });
}

/// Draw the UI for the stack of layers in the project.
pub fn draw_ui_for_layers(
    commands: &mut Commands,
    theme_colors: &theme::ThemeColors,
    ui: &mut egui::Ui,
    layers_query: &mut Layers,
    masks_query: &mut Masks,
) {
    egui::containers::ScrollArea::vertical().show(ui, |ui| {
        ui.heading("Layers");
        if ui.button("New Layer").clicked() {
            let top_layer_id: Option<LayerId> = layers_query
                .layers
                .iter()
                .sort::<&layer::LayerOrder>()
                .last()
                .map(|l| l.layer.id());
            commands.queue(undo::PushAction::from(layer::CreateLayerAction::new(
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
                    commands,
                    theme_colors,
                    ui,
                    masks_query,
                    parent_layer_id,
                    l.entity,
                    l.layer,
                    l.layer_ui.as_mut(),
                    l.height_map_ui.as_mut(),
                    l.is_selected,
                );
            }
        }
    });
}

fn draw_ui_for_layer(
    commands: &mut Commands,
    theme_colors: &theme::ThemeColors,
    ui: &mut egui::Ui,
    masks_query: &mut Masks,
    parent_layer_id: Option<LayerId>,
    entity: Entity,
    layer: &layer::Layer,
    layer_ui: &mut LayerUi,
    height_map_ui: &mut HeightMapUi,
    is_selected: bool,
) {
    let mut frame = egui::containers::Frame::group(ui.style());
    if is_selected {
        frame = frame.fill(ui.style().visuals.widgets.noninteractive.weak_bg_fill);
    }
    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            let height_id = ui.id().with("height").with(layer.id());
            {
                let height: f32 = ui.data(|map| map.get_temp(height_id).unwrap_or(32.0));
                let (response, painter) = ui.allocate_painter(
                    [LAYER_SELECTION_BOX_WIDTH, height].into(),
                    egui::Sense::click(),
                );
                painter.rect_filled(
                    response.rect,
                    4.0,
                    if is_selected {
                        theme_colors.secondary_color.to_color32()
                    } else {
                        theme_colors.bg_alt_color.to_color32()
                    },
                );

                if response.clicked() && !is_selected {
                    commands.queue(SelectLayer(entity));
                }
            }
            let actual_height: f32 = ui
                .vertical_centered_justified(|ui| {
                    match *height_map_ui {
                        HeightMapUi::Constant { .. } => {
                            draw_ui_for_layer_common_top(
                                commands,
                                ui,
                                layer,
                                layer_ui,
                                parent_layer_id,
                            );
                            ui.separator();
                            draw_ui_for_constant_layer(ui, height_map_ui);
                            ui.separator();
                            draw_ui_for_layer_common_bottom(
                                commands,
                                masks_query,
                                entity,
                                layer,
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
    mask: &mut <MaskQuery as QueryData>::Item<'_>,
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
