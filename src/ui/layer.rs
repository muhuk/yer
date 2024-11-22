use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_egui::egui;

use crate::layer;
use crate::undo;

#[derive(SystemParam)]
pub struct LayersQuery<'w, 's> {
    layers: Query<'w, 's, (&'static layer::Layer, &'static layer::HeightMap)>,
}

fn draw_ui_for_constant_layer(
    commands: &mut Commands,
    ui: &mut egui::Ui,
    layer: &layer::Layer,
    height_map: &layer::HeightMap,
    parent_layer_id: Option<layer::LayerId>,
) {
    let layer::HeightMap::Constant(original_level) = *height_map;

    ui.group(|ui| {
        ui.label(format!("{}", layer));
        {
            let mut height_level: f32 = original_level;
            if ui
                .add(egui::widgets::DragValue::new(&mut height_level))
                .changed()
                && height_level != original_level
            {
                // FIXME: Typing '50' results in 2 action, one
                //        from 0 to 5, and then a 2nd one from
                //        5 to 50.
                commands.add::<undo::PushAction>(
                    layer::HeightMapConstantUpdateHeightAction::new(
                        layer.id(),
                        original_level,
                        height_level,
                    )
                    .into(),
                );
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
pub fn draw_ui_for_layers(commands: &mut Commands, ui: &mut egui::Ui, layers_query: LayersQuery) {
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
            for (layer, height_map) in layers_query.layers.iter().sort::<&layer::Layer>().rev() {
                match *height_map {
                    layer::HeightMap::Constant(_) => {
                        draw_ui_for_constant_layer(commands, ui, layer, height_map, parent_layer_id)
                    }
                };

                // Set parent's layer_id for the next iteration.
                parent_layer_id = Some(layer.id());
            }
        }
    });
}
