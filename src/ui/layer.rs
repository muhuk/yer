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

use crate::layer;
use crate::undo;

const LATENCY: Duration = Duration::from_millis(100);

#[derive(SystemParam)]
pub struct LayersQuery<'w, 's> {
    layers: Query<
        'w,
        's,
        (
            &'static layer::Layer,
            &'static mut LayerUi,
            &'static mut HeightMapUi,
        ),
    >,
}

// PLUGIN

pub struct LayerUiPlugin;

impl Plugin for LayerUiPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<HeightMapUi>()
            .register_type::<LayerUi>()
            .add_systems(
                Update,
                (
                    add_height_map_ui_system,
                    add_layer_ui_system,
                    update_height_map_ui_system,
                    reset_height_map_ui_system,
                ),
            );
    }
}

// COMPONENTS

#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
enum HeightMapUi {
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
#[reflect(Component)]
struct LayerUi {
    name: String,
}

impl From<&layer::Layer> for LayerUi {
    fn from(layer: &layer::Layer) -> Self {
        Self {
            name: layer.name.clone(),
        }
    }
}

// SYSTEMS

/// Add a HeightMapUi component to each entity that got HeightMap component added.
fn add_height_map_ui_system(
    mut commands: Commands,
    layers: Query<(Entity, &layer::HeightMap), Added<layer::HeightMap>>,
) {
    for (entity, height_map) in layers.iter() {
        commands
            .entity(entity)
            .insert(HeightMapUi::from(height_map));
    }
}

/// Add a LayerUi component to each entity that got Layer component added.
fn add_layer_ui_system(
    mut commands: Commands,
    layers: Query<(Entity, &layer::Layer), Added<layer::Layer>>,
) {
    for (entity, layer) in layers.iter() {
        commands.entity(entity).insert(LayerUi::from(layer));
    }
}

/// Update HeightMap based on Ui changes.
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
                ref mut height,
                ref mut timer,
            } => {
                if !timer.finished() {
                    timer.tick(time.delta());
                    let layer::HeightMap::Constant(original_height) = height_map;
                    if timer.just_finished() && *original_height != *height {
                        commands.queue::<undo::PushAction>(
                            layer::HeightMapConstantUpdateHeightAction::new(
                                layer.id(),
                                *original_height,
                                *height,
                            )
                            .into(),
                        );
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

// LIB

fn draw_ui_for_constant_layer(
    commands: &mut Commands,
    ui: &mut egui::Ui,
    layer: &layer::Layer,
    layer_ui: &mut LayerUi,
    height_map_ui: &mut HeightMapUi,
    parent_layer_id: Option<layer::LayerId>,
) {
    const LAYER_NAME_CHAR_LIMIT: usize = 20;

    ui.group(|ui| {
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
                commands.queue::<undo::PushAction>(
                    layer::RenameLayerAction::new(layer.id(), &layer.name, &layer_ui.name).into(),
                );
            }
        }
        {
            ui.horizontal(|ui| {
                {
                    let mut layer_preview: bool = layer.enable_preview;
                    if ui.toggle_value(&mut layer_preview, "Preview").changed()
                        && layer_preview != layer.enable_preview
                    {
                        commands.queue::<undo::PushAction>(
                            layer::UpdateLayerAction::toggle_enable_preview(layer).into(),
                        );
                    }
                }
                {
                    let mut layer_baking: bool = layer.enable_baking;
                    if ui.toggle_value(&mut layer_baking, "Bake").changed()
                        && layer_baking != layer.enable_baking
                    {
                        commands.queue::<undo::PushAction>(
                            layer::UpdateLayerAction::toggle_enable_baking(layer).into(),
                        );
                    }
                }
                ui.separator();
                if ui.button("Delete").clicked() {
                    commands.queue::<undo::PushAction>(
                        layer::DeleteLayerAction::new(layer.id(), parent_layer_id).into(),
                    )
                }
            });
        };
        ui.separator();
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
            let response = ui.add_sized(ui.available_size_before_wrap(), widget);
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
    });
}

/// Draw the UI for the stack of layers in the project.
pub fn draw_ui_for_layers(
    commands: &mut Commands,
    ui: &mut egui::Ui,
    mut layers_query: LayersQuery,
) {
    egui::containers::ScrollArea::vertical().show(ui, |ui| {
        ui.heading("Layers");
        if ui.button("New Layer").clicked() {
            let top_layer_id: Option<layer::LayerId> = layers_query
                .layers
                .iter()
                .sort::<&layer::Layer>()
                .last()
                .map(|(layer, _, _)| layer.id());
            commands.queue::<undo::PushAction>(layer::CreateLayerAction::new(top_layer_id).into());
        }
        {
            let mut parent_layer_id: Option<layer::LayerId> = Option::default();

            // We need to iterate layers in reverse order to place the topmost
            // (last applied) layer on top.
            for (layer, mut layer_ui, mut height_map_ui) in
                layers_query.layers.iter_mut().sort::<&layer::Layer>().rev()
            {
                match *height_map_ui {
                    HeightMapUi::Constant { .. } => draw_ui_for_constant_layer(
                        commands,
                        ui,
                        layer,
                        layer_ui.as_mut(),
                        height_map_ui.as_mut(),
                        parent_layer_id,
                    ),
                };

                // Set parent's layer_id for the next iteration.
                parent_layer_id = Some(layer.id());
            }
        }
    });
}
