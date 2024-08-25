use bevy::prelude::*;

pub struct ViewportPlugin;

impl Plugin for ViewportPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup);
    }
}

/// Placeholder code to set up a basic 3D viewport.
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Create camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(1.0, 5.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // A pivot point so we can work in Z-up coords.
    commands
        .spawn(TransformBundle {
            local: Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
            ..default()
        })
        .with_children(|parent| {
            // Create ground quad
            parent.spawn(PbrBundle {
                mesh: meshes.add(Rectangle::new(1.0, 1.0)),
                material: materials.add(Color::WHITE),
                ..default()
            });
        });

    // Add light.
    commands.spawn(DirectionalLightBundle {
        transform: Transform::from_xyz(-3.0, 5.0, -4.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}
