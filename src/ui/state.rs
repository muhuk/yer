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

use bevy::prelude::*;

// PLUGIN

pub struct UiStatePlugin;

impl Plugin for UiStatePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<UiState>().init_state::<UiState>();
    }
}

// STATES

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq, Reflect, States)]
pub enum UiState {
    #[default]
    Interactive,
    ShowingLoadFileDialog,
    ShowingLoadImageDialog,
    ShowingPreferencesDialog,
    ShowingSaveFileDialog,
}

impl UiState {
    pub fn is_interactive(&self) -> bool {
        matches!(self, UiState::Interactive)
    }
}
