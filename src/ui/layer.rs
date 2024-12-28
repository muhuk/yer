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

use std::time::Duration;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_egui::egui;

use crate::layer;
use crate::undo;

const LATENCY: Duration = Duration::from_millis(100);

#[derive(SystemParam)]
pub struct LayersQuery<'w, 's> {
    layers: Query<'w, 's, (&'static layer::Layer, &'static mut HeightMapUi)>,
}

// PLUGIN

pub struct LayerUiPlugin;

impl Plugin for LayerUiPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<HeightMapUi>().add_systems(
            Update,
            (
                add_height_map_ui_system,
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

// SYSTEMS

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
                        commands.add::<undo::PushAction>(
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
    height_map_ui: &mut HeightMapUi,
    parent_layer_id: Option<layer::LayerId>,
) {
    ui.group(|ui| {
        ui.label(format!("{}", layer));
        {
            let original_height: f32 = {
                let HeightMapUi::Constant { height, .. } = height_map_ui;
                *height
            };
            let mut height_edited: f32 = original_height;
            let widget = egui::widgets::DragValue::new(&mut height_edited)
                .range(layer::HEIGHT_RANGE)
                .update_while_editing(false);
            let response = ui.add(widget);
            if response.changed() && height_edited != original_height {
                match height_map_ui {
                    HeightMapUi::Constant { height, timer } => {
                        *height = height_edited;
                        timer.unpause();
                        timer.reset();
                    }
                }
            }
        }
        {
            let mut layer_preview: bool = layer.enable_preview;
            if ui.toggle_value(&mut layer_preview, "preview").changed()
                && layer_preview != layer.enable_preview
            {
                todo!("update preview");
            }
        }
        {
            let mut layer_baking: bool = layer.enable_baking;
            if ui.toggle_value(&mut layer_baking, "bake").changed()
                && layer_baking != layer.enable_baking
            {
                todo!("update bake");
            }
        }
        if ui.button("Delete").clicked() {
            commands.add::<undo::PushAction>(
                layer::DeleteLayerAction::new(layer.id(), parent_layer_id).into(),
            )
        }
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
                .map(|(layer, _)| layer.id());
            commands.add::<undo::PushAction>(layer::CreateLayerAction::new(top_layer_id).into());
        }
        {
            let mut parent_layer_id: Option<layer::LayerId> = Option::default();

            // We need to iterate layers in reverse order to place the topmost
            // (last applied) layer on top.
            for (layer, mut height_map_ui) in
                layers_query.layers.iter_mut().sort::<&layer::Layer>().rev()
            {
                match *height_map_ui {
                    HeightMapUi::Constant { .. } => draw_ui_for_constant_layer(
                        commands,
                        ui,
                        layer,
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
