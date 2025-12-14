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

use std::collections::VecDeque;
use std::fmt::Debug;
use std::num::NonZeroUsize;

use bevy::prelude::*;

use crate::constants;
use crate::preferences::Preferences;

// PLUGIN

pub struct UndoPlugin;

impl Plugin for UndoPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<UndoEvent>();
        app.register_type::<UndoStack>()
            .insert_resource(UndoStack::new(constants::DEFAULT_UNDO_STACK_SIZE));
        app.add_systems(
            FixedUpdate,
            update_stack_size_when_preferences_change_system
                .run_if(resource_exists_and_changed::<Preferences>),
        );
    }
}

// EVENTS

#[derive(Debug, Message)]
pub enum UndoEvent {
    ActionPushed { old_action_dropped: bool },
    ActionReapplied,
    ActionReverted,
    StackCleared,
    StackSizeChanged { old_size: usize, new_size: usize },
}

// RESOURCES

#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct UndoStack {
    max_actions: NonZeroUsize,

    // We can reflect these two fields when Box<dyn Trait> is supported.
    // See https://github.com/bevyengine/bevy/pull/15532
    #[reflect(ignore)]
    undo_actions: VecDeque<Box<dyn Action>>,
    #[reflect(ignore)]
    redo_actions: VecDeque<Box<dyn Action>>,
}

impl UndoStack {
    pub fn new(max_actions: NonZeroUsize) -> Self {
        Self {
            max_actions,
            undo_actions: VecDeque::new(),
            redo_actions: VecDeque::new(),
        }
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_actions.is_empty()
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_actions.is_empty()
    }

    fn adjust_stack_size(&mut self, new_value: impl Into<NonZeroUsize>) {
        self.max_actions = new_value.into();
        if self.undo_actions.len() + self.redo_actions.len() > self.max_actions.get() {
            if self.max_actions.get() >= self.redo_actions.len() {
                let excess =
                    self.undo_actions.len() + self.redo_actions.len() - self.max_actions.get();
                for _ in 0..excess {
                    self.undo_actions.pop_front().unwrap();
                }
            } else {
                let excess_redo = self
                    .redo_actions
                    .len()
                    .saturating_sub(self.max_actions.get());
                self.undo_actions.clear();
                for _ in 0..excess_redo {
                    self.redo_actions.pop_front().unwrap();
                }
            }
        }
    }

    #[must_use]
    fn push_action(&mut self, action: Box<dyn Action>) -> bool {
        let mut old_action_dropped = false;
        // The new action is pushed as a result of user input.  Therefore any
        // actions undoed before are no longer redoable.
        self.redo_actions.clear();
        if self.undo_actions.len() >= self.max_actions.get() {
            self.undo_actions.pop_front().unwrap();
            old_action_dropped = true;
        }
        self.undo_actions.push_back(action);
        return old_action_dropped;
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
        world.write_message(UndoEvent::StackCleared);
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
        let old_action_dropped = world.resource_mut::<UndoStack>().push_action(action);
        world.write_message(UndoEvent::ActionPushed { old_action_dropped });
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
            .pop_front()
            .unwrap();
        action.apply(world);
        world
            .resource_mut::<UndoStack>()
            .undo_actions
            .push_back(action);
        world.write_message(UndoEvent::ActionReapplied);
    }
}

pub struct SetUndoStackSize(NonZeroUsize);

impl Command for SetUndoStackSize {
    fn apply(self, world: &mut World) -> () {
        let mut undo_stack = world.resource_mut::<UndoStack>();

        if undo_stack.max_actions == self.0 {
            // Nothing to do, earyly exit.
            return;
        }

        let old_size: usize = undo_stack.max_actions.get();
        let new_size: usize = self.0.get();

        undo_stack.adjust_stack_size(self.0);
        world.write_message(UndoEvent::StackSizeChanged { old_size, new_size });
    }
}

pub struct UndoAction;

impl Command for UndoAction {
    fn apply(self, world: &mut World) {
        let action = world
            .resource_mut::<UndoStack>()
            .undo_actions
            .pop_back()
            .unwrap();
        action.revert(world);
        world
            .resource_mut::<UndoStack>()
            .redo_actions
            .push_front(action);
        world.write_message(UndoEvent::ActionReverted);
    }
}

// SYSTEMS

fn update_stack_size_when_preferences_change_system(
    preferences: Res<Preferences>,
    mut undo_stack: ResMut<UndoStack>,
) {
    if undo_stack.max_actions != preferences.max_undo_stack_size {
        undo_stack.adjust_stack_size(preferences.max_undo_stack_size);
    }
}

// LIB

#[reflect_trait]
pub trait Action: Reflect + Debug + Send + Sync {
    fn apply(&self, world: &mut World);
    fn revert(&self, world: &mut World);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Reflect)]
    struct MockAction(pub usize);

    impl Action for MockAction {
        fn apply(&self, _world: &mut World) {}
        fn revert(&self, _world: &mut World) {}
    }

    #[test]
    fn adding_an_action_beyond_max_actions_drop_from_the_stack() {
        let mut app = App::new();
        app.add_plugins(UndoPlugin);
        app.world_mut()
            .resource_mut::<UndoStack>()
            .adjust_stack_size(NonZeroUsize::new(2).unwrap());
        for idx in 0..5 {
            app.world_mut()
                .commands()
                .queue(PushAction(Box::new(MockAction(idx))));
        }
        app.update();
        assert_eq!(
            app.world().resource::<UndoStack>().undo_actions[0]
                .as_any()
                .downcast_ref::<MockAction>()
                .unwrap(),
            &MockAction(3)
        );
        assert_eq!(
            app.world().resource::<UndoStack>().undo_actions[1]
                .as_any()
                .downcast_ref::<MockAction>()
                .unwrap(),
            &MockAction(4)
        );
    }

    #[test]
    fn when_stack_size_is_decreased_beyond_undo_actions_redo_actions_are_dropped() {
        let mut app = App::new();
        app.add_plugins(UndoPlugin);
        app.world_mut()
            .resource_mut::<UndoStack>()
            .adjust_stack_size(NonZeroUsize::new(10).unwrap());
        for idx in 0..8 {
            app.world_mut()
                .commands()
                .queue(PushAction(Box::new(MockAction(idx))));
        }
        app.update();
        for _ in 0..3 {
            app.world_mut().commands().queue(UndoAction);
        }
        app.update();

        assert_eq!(app.world().resource::<UndoStack>().max_actions.get(), 10);
        assert_eq!(app.world().resource::<UndoStack>().undo_actions.len(), 5);
        assert_eq!(app.world().resource::<UndoStack>().redo_actions.len(), 3);

        app.world_mut()
            .commands()
            .queue(SetUndoStackSize(NonZeroUsize::new(2).unwrap()));
        app.update();
        assert_eq!(app.world().resource::<UndoStack>().max_actions.get(), 2);
        assert_eq!(app.world().resource::<UndoStack>().undo_actions.len(), 0);
        assert_eq!(app.world().resource::<UndoStack>().redo_actions.len(), 2);
    }

    #[test]
    fn when_stack_size_is_decreased_first_undo_actions_are_dropped() {
        let mut app = App::new();
        app.add_plugins(UndoPlugin);
        app.world_mut()
            .resource_mut::<UndoStack>()
            .adjust_stack_size(NonZeroUsize::new(10).unwrap());
        for idx in 0..8 {
            app.world_mut()
                .commands()
                .queue(PushAction(Box::new(MockAction(idx))));
        }
        app.update();
        for _ in 0..3 {
            app.world_mut().commands().queue(UndoAction);
        }
        app.update();

        assert_eq!(app.world().resource::<UndoStack>().max_actions.get(), 10);
        assert_eq!(app.world().resource::<UndoStack>().undo_actions.len(), 5);
        assert_eq!(app.world().resource::<UndoStack>().redo_actions.len(), 3);

        app.world_mut()
            .commands()
            .queue(SetUndoStackSize(NonZeroUsize::new(4).unwrap()));
        app.update();
        assert_eq!(app.world().resource::<UndoStack>().max_actions.get(), 4);
        assert_eq!(app.world().resource::<UndoStack>().undo_actions.len(), 1);
        assert_eq!(app.world().resource::<UndoStack>().redo_actions.len(), 3);
    }

    #[test]
    fn when_stack_size_is_increased_no_action_is_dropped() {
        let mut app = App::new();
        app.add_plugins(UndoPlugin);
        app.world_mut()
            .resource_mut::<UndoStack>()
            .adjust_stack_size(NonZeroUsize::new(2).unwrap());
        app.world_mut()
            .commands()
            .queue(PushAction(Box::new(MockAction(1))));
        app.world_mut()
            .commands()
            .queue(PushAction(Box::new(MockAction(2))));
        app.update();
        app.world_mut().commands().queue(UndoAction);
        app.update();

        assert_eq!(app.world().resource::<UndoStack>().max_actions.get(), 2);
        assert_eq!(app.world().resource::<UndoStack>().undo_actions.len(), 1);
        assert_eq!(app.world().resource::<UndoStack>().redo_actions.len(), 1);

        app.world_mut()
            .commands()
            .queue(SetUndoStackSize(NonZeroUsize::new(3).unwrap()));
        app.update();
        assert_eq!(app.world().resource::<UndoStack>().max_actions.get(), 3);
        assert_eq!(app.world().resource::<UndoStack>().undo_actions.len(), 1);
        assert_eq!(app.world().resource::<UndoStack>().redo_actions.len(), 1);
    }
}
