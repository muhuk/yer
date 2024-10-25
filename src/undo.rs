use std::fmt::Debug;

use bevy::prelude::*;

pub struct UndoPlugin;

impl Plugin for UndoPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<UndoStack>()
            .init_resource::<UndoStack>();
    }
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

    pub fn push(&mut self, action: Box<dyn Action>) {
        self.redo_actions.clear();
        self.undo_actions.push(action);
    }

    pub fn redo(&mut self, world: &mut World) {
        let action = self.redo_actions.pop().unwrap();
        action.apply(world);
        self.undo_actions.push(action);
    }

    pub fn undo(&mut self, world: &mut World) {
        let action = self.undo_actions.pop().unwrap();
        action.revert(world);
        self.redo_actions.push(action);
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

// LIB

#[reflect_trait]
pub trait Action: Reflect + Send + Sync {
    fn apply(&self, world: &mut World);
    fn revert(&self, world: &mut World);
}
