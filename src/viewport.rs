use bevy::input::common_conditions::input_pressed;
use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;

pub struct ViewportPlugin;

impl Plugin for ViewportPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<TargetTransform>()
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (
                    draw_grid,
                    middle_mouse_actions.run_if(input_pressed(MouseButton::Middle)),
                    update_camera,
                ),
            );
    }
}

// COMPONENTS

#[derive(Component, Reflect)]
struct TargetTransform {
    translation: Vec3,
}

// SYSTEMS

/// Placeholder code to set up a basic 3D viewport.
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    const CAMERA_INITIAL_TRANSLATION: Vec3 = Vec3::new(1.0, 5.0, 4.0);

    // Create camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_translation(CAMERA_INITIAL_TRANSLATION)
                .looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        TargetTransform {
            translation: CAMERA_INITIAL_TRANSLATION,
        },
        Name::new("Camera"),
    ));

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
            Name::new("Pivot Z-Up"),
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

fn middle_mouse_actions(
    mut mouse_motion_reader: EventReader<MouseMotion>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut target_transform_query: Query<(&mut TargetTransform, &Transform), With<Camera>>,
) {
    // We are reading these events without fear becuase this system must be
    // run only when MMB is pressed or it will panic.  Otherwise consuming
    // these events would prevent another system to read them.
    let mouse_motion: Vec2 = mouse_motion_reader
        .read()
        .fold(Vec2::ZERO, |acc, ev| acc + ev.delta);

    match (
        mouse_button.pressed(MouseButton::Middle),
        mouse_motion.length() > f32::EPSILON,
        keyboard_input.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]),
        keyboard_input.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]),
    ) {
        (true, true, true, false) => {
            println!("mouse pan -- {}", &mouse_motion);
            if let Ok((mut target_transform, transform)) = target_transform_query.get_single_mut() {
                const PAN_SPEED: f32 = 1.0 / 96.0;
                let mut delta: Vec3 = Vec3::new(-mouse_motion.x, 0.0, -mouse_motion.y) * PAN_SPEED;
                // Take orientation into account but only around Y, we're panning across XZ.
                delta = Quat::from_rotation_y(transform.rotation.to_euler(EulerRot::YXZ).0) * delta;
                target_transform.as_mut().translation += delta;
            }
        }
        (true, true, false, true) => println!("mouse zoom"),
        (true, true, false, false) => println!("mouse orbit"),
        (true, _, true, true) => (), // Do nothing if both shift and control is pressed.
        (true, false, _, _) => (),   // Do nothing if there is no mouse movement.
        (false, _, _, _) => unreachable!(),
    }
}

fn update_camera(
    time: Res<Time>,
    mut camera_query: Query<(&TargetTransform, &mut Transform), With<Camera>>,
) {
    const INTERPOLATION_FACTOR: f32 = 1.0f32 / 3.0f32;

    if let Ok((target_transform, mut transform)) = camera_query.get_single_mut() {
        // https://www.reddit.com/r/gamedev/comments/cayb4f/basic_smooth_spring_movement/
        //
        // pow keeps the interpolation factor about the
        // same, but adds frame-rate sensitivity.
        transform.translation = Vec3::interpolate(
            &target_transform.translation,
            &transform.translation,
            (1.0f32 - INTERPOLATION_FACTOR).powf(time.delta_seconds() * 60.0),
        );
    }
}
