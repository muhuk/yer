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

use std::time::Duration;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_egui::egui;

use crate::id::{LayerId, MaskId};
use crate::layer;
use crate::math::{approx_eq, ONE_IN_TEN_THOUSAND};
use crate::theme;
use crate::undo;

use super::egui_ext::ToColor32;

const LATENCY: Duration = Duration::from_millis(100);
const LAYER_SELECTION_BOX_WIDTH: f32 = 24.0f32;

#[derive(SystemParam)]
pub struct LayersQuery<'w, 's> {
    pub layers: Query<
        'w,
        's,
        (
            Entity,
            &'static layer::Layer,
            &'static layer::LayerOrder,
            &'static mut LayerUi,
            &'static mut HeightMapUi,
            Has<Selected>,
        ),
    >,
}

#[derive(SystemParam)]
pub struct MasksQuery<'w, 's> {
    pub masks: Query<
        'w,
        's,
        (
            Entity,
            &'static layer::Mask,
            &'static layer::MaskOrder,
            &'static layer::SdfMask,
        ),
    >,
}

// PLUGIN

pub struct LayerUiPlugin;

impl Plugin for LayerUiPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<HeightMapUi>()
            .register_type::<LayerUi>()
            .register_type::<Selected>()
            .register_type::<SdfMaskUi>();
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
pub(super) enum SdfMaskUi {
    Circle {
        center: Vec2,
        radius: f32,
        falloff_radius: f32,
        timer: Timer,
    },
}

impl From<&layer::SdfMask> for SdfMaskUi {
    fn from(value: &layer::SdfMask) -> Self {
        match value {
            layer::SdfMask::Circle {
                center,
                radius,
                falloff_radius,
            } => Self::Circle {
                center: *center,
                radius: *radius,
                falloff_radius: *falloff_radius,
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

/// Add an SdfMaskUi component to each entity with a SdfMask added.
fn add_mask_ui_system(
    mut commands: Commands,
    masks: Query<(Entity, &layer::SdfMask), Added<layer::SdfMask>>,
) {
    for (entity, sdf_mask) in masks.iter() {
        commands.entity(entity).insert(SdfMaskUi::from(sdf_mask));
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

/// Update SdfMask based on UI changes.
fn update_mask_ui_system(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut masks: Query<(&layer::Mask, &layer::SdfMask, &mut SdfMaskUi)>,
) {
    for (mask, sdf_mask, mut sdf_mask_ui) in masks.iter_mut() {
        match *sdf_mask_ui {
            SdfMaskUi::Circle {
                center,
                radius,
                falloff_radius,
                ref mut timer,
            } => {
                if !timer.finished() {
                    timer.tick(time.delta());
                    let layer::SdfMask::Circle {
                        center: original_center,
                        radius: original_radius,
                        falloff_radius: original_falloff_radius,
                    } = *sdf_mask;
                    if timer.just_finished() && original_center.distance(center) <= f32::EPSILON {
                        commands.queue(undo::PushAction::from(
                            layer::UpdateMaskAction::update_center(
                                mask.id(),
                                original_center,
                                center,
                            ),
                        ));
                    }
                    if timer.just_finished()
                        && !approx_eq(original_falloff_radius, falloff_radius, ONE_IN_TEN_THOUSAND)
                    {
                        commands.queue(undo::PushAction::from(
                            layer::UpdateMaskAction::update_falloff_radius(
                                mask.id(),
                                original_falloff_radius,
                                falloff_radius,
                            ),
                        ));
                    }
                    if timer.just_finished()
                        && !approx_eq(original_radius, radius, ONE_IN_TEN_THOUSAND)
                    {
                        commands.queue(undo::PushAction::from(
                            layer::UpdateMaskAction::update_radius(
                                mask.id(),
                                original_radius,
                                radius,
                            ),
                        ));
                    }
                }
            }
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

/// Update SdfMaskUi based on changes to SdfMask.
///
/// This gets triggered when undo/redo changes [SdfMask](layer::SdfMask).
fn reset_mask_ui_system(
    mut masks: Query<(&layer::SdfMask, &mut SdfMaskUi), Changed<layer::SdfMask>>,
) {
    for (sdf_mask, mut sdf_mask_ui) in masks.iter_mut() {
        match sdf_mask {
            layer::SdfMask::Circle {
                center: original_center,
                radius: original_radius,
                falloff_radius: original_falloff_radius,
            } => {
                let SdfMaskUi::Circle {
                    ref mut center,
                    ref mut radius,
                    ref mut falloff_radius,
                    ref mut timer,
                } = *sdf_mask_ui;
                *center = *original_center;
                *radius = *original_radius;
                *falloff_radius = *original_falloff_radius;
                timer.pause();
            }
        }
    }
}

// LIB

fn draw_ui_for_layer_common_bottom(
    commands: &mut Commands,
    children_query: &Query<&Children>,
    masks_query: &mut MasksQuery,
    entity: Entity,
    layer: &layer::Layer,
    ui: &mut egui::Ui,
) {
    let frame = egui::containers::Frame::group(ui.style());
    frame.show(ui, |ui| {
        ui.heading("Masks");

        if ui.button("Add mask").clicked() {
            let mask_bundle: layer::MaskBundle = layer::MaskBundle::default();
            let layer_id: LayerId = layer.id();
            // FIXME: Find out the topmost mask instead of just passing None.
            let previous_mask_id: Option<MaskId> = None;
            commands.queue(undo::PushAction::from(layer::CreateMaskAction::new(
                mask_bundle,
                layer_id,
                previous_mask_id,
            )));
        }

        if let Ok(children) = children_query.get(entity) {
            // FIXME: We are not ordering masks using MaskOrder.
            for mask_entity in children.iter() {
                frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(format!("Mask: {:?}", mask_entity));
                        if ui.button("Delete").clicked() {
                            let mask_bundle: layer::MaskBundle = {
                                let (_, mask, _, sdf_mask) =
                                    masks_query.masks.get(mask_entity).unwrap();
                                layer::MaskBundle {
                                    mask: mask.clone(),
                                    sdf_mask: sdf_mask.clone(),
                                }
                            };
                            let layer_id: LayerId = layer.id();
                            // FIXME: Use actual previous mask id instead of None.
                            let previous_mask_id: Option<MaskId> = None;
                            commands.queue(undo::PushAction::from(layer::DeleteMaskAction::new(
                                mask_bundle,
                                layer_id,
                                previous_mask_id,
                            )));
                        }
                    });
                });
            }
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
        let original_height: f32 = {
            let HeightMapUi::Constant { height, .. } = height_map_ui;
            *height
        };
        let mut height_edited: f32 = original_height;
        let widget = egui::widgets::DragValue::new(&mut height_edited)
            .range(layer::HEIGHT_RANGE)
            .update_while_editing(false);
        let response = ui
            .with_layout(
                egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                |ui| ui.add(widget),
            )
            .inner;
        if response.changed() && height_edited != original_height {
            match height_map_ui {
                HeightMapUi::Constant { height, timer } => {
                    *height = height_edited;
                    timer.unpause();
                    timer.reset();
                }
            }
        }
    });
}

/// Draw the UI for the stack of layers in the project.
pub fn draw_ui_for_layers(
    commands: &mut Commands,
    theme_colors: &theme::ThemeColors,
    ui: &mut egui::Ui,
    children_query: &Query<&Children>,
    layers_query: &mut LayersQuery,
    masks_query: &mut MasksQuery,
) {
    egui::containers::ScrollArea::vertical().show(ui, |ui| {
        ui.heading("Layers");
        if ui.button("New Layer").clicked() {
            let top_layer_id: Option<LayerId> = layers_query
                .layers
                .iter()
                .sort::<&layer::LayerOrder>()
                .last()
                .map(|(_, layer, _, _, _, _)| layer.id());
            commands.queue(undo::PushAction::from(layer::CreateLayerAction::new(
                top_layer_id,
            )));
        }
        {
            let layer_ids: Vec<LayerId> = layers_query
                .layers
                .iter()
                .sort::<&layer::LayerOrder>()
                .rev()
                .map(|(_, layer, _, _, _, _)| layer.id())
                .collect();
            // We need to iterate layers in reverse order to place the topmost
            // (last applied) layer on top.
            for (idx, (entity, layer, _, mut layer_ui, mut height_map_ui, is_selected)) in
                layers_query
                    .layers
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
                    children_query,
                    masks_query,
                    parent_layer_id,
                    entity,
                    layer,
                    layer_ui.as_mut(),
                    height_map_ui.as_mut(),
                    is_selected,
                );
            }
        }
    });
}

fn draw_ui_for_layer(
    commands: &mut Commands,
    theme_colors: &theme::ThemeColors,
    ui: &mut egui::Ui,
    children_query: &Query<&Children>,
    masks_query: &mut MasksQuery,
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
            let height_id = ui.id().with("height");
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
                                children_query,
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
