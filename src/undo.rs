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

use std::fmt::Debug;

use bevy::ecs::world::Command;
use bevy::prelude::*;

pub struct UndoPlugin;

impl Plugin for UndoPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<UndoStack>()
            .add_event::<UndoEvent>()
            .init_resource::<UndoStack>();
    }
}

// EVENTS

#[derive(Debug, Event)]
pub enum UndoEvent {
    ActionPushed,
    ActionReapplied,
    ActionReverted,
    StackCleared,
}

// RESOURCES

#[derive(Default, Resource, Reflect)]
#[reflect(Resource)]
pub struct UndoStack {
    // We can reflect these two fields when Box<dyn Trait> is supported.
    // See https://github.com/bevyengine/bevy/pull/15532
    #[reflect(ignore)]
    undo_actions: Vec<Box<dyn Action>>,
    #[reflect(ignore)]
    redo_actions: Vec<Box<dyn Action>>,
}

impl UndoStack {
    pub fn can_redo(&self) -> bool {
        !self.redo_actions.is_empty()
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_actions.is_empty()
    }
}

impl Debug for UndoStack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "UndoStack(undo: {}, redo: {}",
            self.undo_actions.len(),
            self.redo_actions.len()
        )
    }
}

// COMMANDS

pub struct ClearStack;

impl Command for ClearStack {
    fn apply(self, world: &mut World) {
        let mut undo_stack = world.resource_mut::<UndoStack>();
        undo_stack.undo_actions.clear();
        undo_stack.redo_actions.clear();
        world.send_event(UndoEvent::StackCleared);
    }
}

/// Push an action onto undo stack.
///
/// Do not call `apply` on the action.  This command will apply the action.
pub struct PushAction(pub Box<dyn Action>);

impl Command for PushAction {
    fn apply(self, world: &mut World) {
        let action = self.0;
        debug!("Pushing new action: {:?}", &action);
        action.apply(world);
        let mut undo_stack = world.resource_mut::<UndoStack>();
        // The new action is pushed as a result of user input.  Therefore any
        // actions undoed before are no longer redoable.
        undo_stack.redo_actions.clear();
        undo_stack.undo_actions.push(action);
        world.send_event(UndoEvent::ActionPushed);
    }
}

impl<T: Action> From<T> for PushAction {
    fn from(action: T) -> Self {
        PushAction(Box::new(action))
    }
}

pub struct RedoAction;

impl Command for RedoAction {
    fn apply(self, world: &mut World) {
        let action = world
            .resource_mut::<UndoStack>()
            .redo_actions
            .pop()
            .unwrap();
        action.apply(world);
        world.resource_mut::<UndoStack>().undo_actions.push(action);
        world.send_event(UndoEvent::ActionReapplied);
    }
}

pub struct UndoAction;

impl Command for UndoAction {
    fn apply(self, world: &mut World) {
        let action = world
            .resource_mut::<UndoStack>()
            .undo_actions
            .pop()
            .unwrap();
        action.revert(world);
        world.resource_mut::<UndoStack>().redo_actions.push(action);
        world.send_event(UndoEvent::ActionReverted);
    }
}

// LIB

#[reflect_trait]
pub trait Action: Reflect + Debug + Send + Sync {
    fn apply(&self, world: &mut World);
    fn revert(&self, world: &mut World);
}
