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

use std::num::NonZeroU8;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_egui::egui;

use crate::preview;

#[derive(SystemParam)]
pub struct PreviewQuery<'w, 's> {
    preview_regions: Query<'w, 's, (Entity, &'static preview::PreviewRegion)>,
    update_preview_region_events: EventWriter<'w, preview::UpdatePreviewRegion>,
}

pub fn draw_ui_for_preview(ui: &mut egui::Ui, mut preview_query: PreviewQuery) {
    ui.heading("Preview");
    if let Ok((entity, preview_region)) = preview_query.preview_regions.get_single() {
        ui.horizontal(|ui| {
            ui.label("Center");
            let mut center: Vec2 = preview_region.center();
            ui.add(egui::widgets::DragValue::new(&mut center.x));
            ui.add(egui::widgets::DragValue::new(&mut center.y));
            if center != preview_region.center() {
                preview_query
                    .update_preview_region_events
                    .send(preview::UpdatePreviewRegion::SetCenter(entity, center));
            }
        });

        ui.horizontal(|ui| {
            ui.label("Size");
            let mut size: f32 = preview_region.size();
            ui.add(egui::widgets::DragValue::new(&mut size));
            if size != preview_region.size() {
                preview_query
                    .update_preview_region_events
                    .send(preview::UpdatePreviewRegion::SetSize(entity, size));
            }
        });

        ui.horizontal(|ui| {
            ui.label("Subdivisions");
            let mut subdivisions: u8 = preview_region.subdivisions().get();
            ui.add(egui::widgets::DragValue::new(&mut subdivisions));
            if NonZeroU8::new(subdivisions).unwrap() != preview_region.subdivisions() {
                preview_query.update_preview_region_events.send(
                    preview::UpdatePreviewRegion::SetSubdivisions(
                        entity,
                        NonZeroU8::new(subdivisions).unwrap(),
                    ),
                );
            }
        });
    }
}
