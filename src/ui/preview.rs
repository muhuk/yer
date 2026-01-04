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

use std::num::NonZeroU8;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_egui::egui;

use crate::preview;

#[derive(SystemParam)]
pub struct PreviewQuery<'w, 's> {
    preview_regions: Query<'w, 's, (Entity, &'static preview::PreviewRegion)>,
    update_preview_region_events: MessageWriter<'w, preview::UpdatePreviewRegion>,
}

// LIB

pub fn draw_ui_for_preview(ui: &mut egui::Ui, mut preview_query: PreviewQuery) {
    ui.heading("Preview");
    if let Ok((entity, preview_region)) = preview_query.preview_regions.single() {
        ui.horizontal(|ui| {
            ui.label("Center");
            let mut center: Vec2 = preview_region.center();
            ui.add(egui::widgets::DragValue::new(&mut center.x).update_while_editing(false));
            ui.add(egui::widgets::DragValue::new(&mut center.y).update_while_editing(false));
            if center != preview_region.center() {
                preview_query
                    .update_preview_region_events
                    .write(preview::UpdatePreviewRegion::SetCenter(entity, center));
            }
        });

        ui.horizontal(|ui| {
            ui.label("Size");
            let mut size: f32 = preview_region.size();
            ui.add(egui::widgets::DragValue::new(&mut size).update_while_editing(false));
            if size != preview_region.size() {
                preview_query
                    .update_preview_region_events
                    .write(preview::UpdatePreviewRegion::SetSize(entity, size));
            }
        });

        ui.horizontal(|ui| {
            ui.label("Subdivisions");
            let mut subdivisions: u8 = preview_region.subdivisions().get();
            egui::ComboBox::from_id_salt(format!("preview-subdivisions-{}", entity))
                .selected_text(format!("{:?}", subdivisions))
                .show_ui(ui, |ui| {
                    for value in preview::MIN_SUBDIVISIONS.get()..=preview::MAX_SUBDIVISIONS.get() {
                        ui.selectable_value(
                            &mut subdivisions,
                            value,
                            format!(
                                "{} ({} × {})",
                                value,
                                2u32.pow(value.into()) + 1,
                                2u32.pow(value.into()) + 1
                            ),
                        );
                    }
                });
            if subdivisions != preview_region.subdivisions().get() {
                preview_query.update_preview_region_events.write(
                    preview::UpdatePreviewRegion::SetSubdivisions(
                        entity,
                        NonZeroU8::new(subdivisions).unwrap(),
                    ),
                );
            }
        });
    } else {
        panic!("There are multiple preview regions, or none.");
    }
}
