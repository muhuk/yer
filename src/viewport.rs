use bevy::prelude::*;

pub struct ViewportPlugin;

impl Plugin for ViewportPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, draw_grid);
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
        .spawn((
            TransformBundle {
                local: Transform::from_rotation(Quat::from_rotation_x(
                    -std::f32::consts::FRAC_PI_2,
                )),
                ..default()
            },
            VisibilityBundle::default(),
        ))
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

fn draw_grid(mut gizmos: Gizmos) {
    gizmos
        .grid_3d(
            Vec3::ZERO,            // position
            Quat::IDENTITY,        // rotation
            UVec3::new(10, 0, 10), // cells
            Vec3::splat(1.0),      // spacing
            LinearRgba::GREEN.with_alpha(0.8),
        )
        .outer_edges();

    gizmos
        .grid_3d(
            Vec3::ZERO,              // position
            Quat::IDENTITY,          // rotation
            UVec3::new(100, 0, 100), // cells
            Vec3::splat(0.1),        // spacing
            LinearRgba::GREEN.with_alpha(0.15),
        )
        .outer_edges();
}
