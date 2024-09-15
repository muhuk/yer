use bevy::ecs::world::Command;
use bevy::prelude::*;

const LAYER_SPACING: u32 = 100;

pub struct LayerPlugin;

impl Plugin for LayerPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Layer>();
    }
}

// COMPONENTS

#[derive(Component, Eq, Ord, PartialEq, PartialOrd, Reflect)]
struct Layer {
    order: u32,
}

// COMMANDS

pub enum CreateLayer {
    OnTop,
}

impl Command for CreateLayer {
    fn apply(self, world: &mut World) {
        // Find the `order` of the top layer:
        let max_order: u32 = world
            .query::<&Layer>()
            .iter(world)
            .sort::<&Layer>()
            .last()
            .map_or(0, |layer| layer.order);

        world.spawn(Layer {
            order: max_order + LAYER_SPACING,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_new_layer_on_top() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, LayerPlugin));
        app.finish();
        app.cleanup();
        app.update();

        assert_eq!(
            app.world_mut()
                .query::<&Layer>()
                .iter(app.world_mut())
                .count(),
            0
        );
        app.world_mut().commands().push(CreateLayer::OnTop);
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&Layer>()
                .iter(app.world_mut())
                .count(),
            1
        );
    }
}
