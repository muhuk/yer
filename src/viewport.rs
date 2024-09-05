use bevy::input::common_conditions::input_pressed;
use bevy::input::mouse::MouseMotion;
use bevy::math::{Affine3A, VectorSpace};
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
                    draw_focal_point,
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
    rotation: Quat,
}

impl TargetTransform {
    fn looking_at(&self) -> Vec3 {
        let ray = Ray3d::new(self.translation, self.rotation * Vec3::NEG_Z);
        //println!("{:?}", &ray);
        match ray.intersect_plane(Vec3::ZERO, InfinitePlane3d::new(Vec3::Y)) {
            Some(distance) => return ray.get_point(distance),
            None => panic!("Cannot calculate camera look at target."),
        }
    }

    fn orbit_xy(&mut self, x: f32, y: f32) {
        const MIN_Y: f32 = 1.0;
        const ORBIT_SPEED: f32 = 1.0 / 1000.0;
        let look_at_target: Vec3 = self.looking_at();
        // Translation from look_at_target to camera
        let rev_relative_position = self.translation - look_at_target;
        // Note: X and Y are reversed!
        let orbit_rotation = Quat::from_euler(EulerRot::YXZ, x * ORBIT_SPEED, y * ORBIT_SPEED, 0.0);
        let rotated_rev_rel_pos: Vec3 = orbit_rotation * rev_relative_position;
        self.translation = look_at_target + rotated_rev_rel_pos;
        self.translation.y = f32::max(self.translation.y, MIN_Y);
        // Why do we need inverse here, I don't know.
        self.rotation = Affine3A::look_at_rh(self.translation, look_at_target, Vec3::Y)
            .inverse()
            .to_scale_rotation_translation()
            .1;
    }

    fn pan_xy(&mut self, x: f32, y: f32) {
        const PAN_SPEED: f32 = 1.0 / 96.0;
        let mut delta: Vec3 = Vec3::new(x, 0.0, y) * PAN_SPEED;
        // Take orientation into account but only around Y, we're panning across XZ.
        delta = Quat::from_rotation_y(self.rotation.to_euler(EulerRot::YXZ).0) * delta;
        self.translation += delta;
    }
}

// SYSTEMS

/// Placeholder code to set up a basic 3D viewport.
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    const CAMERA_INITIAL_TRANSLATION: Vec3 = Vec3::new(1.0, 5.0, 4.0);
    const CAMERA_INITIAL_TARGET: Vec3 = Vec3::ZERO;

    // Create camera
    let transform = Transform::from_translation(CAMERA_INITIAL_TRANSLATION)
        .looking_at(CAMERA_INITIAL_TARGET, Vec3::Y);
    commands.spawn((
        Camera3dBundle {
            transform,
            ..default()
        },
        TargetTransform {
            translation: transform.translation,
            rotation: transform.rotation,
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

fn draw_focal_point(
    mut gizmos: Gizmos,
    target_transform_query: Query<&TargetTransform, With<Camera>>,
) {
    if let Ok(target_transform) = target_transform_query.get_single() {
        gizmos.circle(
            target_transform.looking_at(),
            Dir3::Y,
            0.025f32,
            LinearRgba::RED,
        );
    }
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
    mut target_transform_query: Query<&mut TargetTransform, With<Camera>>,
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
            // Pan
            target_transform_query
                .get_single_mut()
                .expect("There must be one and only one camera.")
                .pan_xy(-mouse_motion.x, -mouse_motion.y);
        }
        (true, true, false, true) => println!("mouse zoom"),
        (true, true, false, false) => {
            // Orbit
            target_transform_query
                .get_single_mut()
                .expect("There must be one and only one camera.")
                .orbit_xy(-mouse_motion.x, -mouse_motion.y);
        }
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

        transform.rotation = Quat::interpolate(
            &target_transform.rotation,
            &transform.rotation,
            (1.0f32 - INTERPOLATION_FACTOR).powf(time.delta_seconds() * 60.0),
        );
    }
}
