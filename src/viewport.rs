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

use bevy::input::{common_conditions::input_pressed, mouse::MouseMotion};
use bevy::math::Affine3A;
use bevy::pbr::wireframe::{Wireframe, WireframeColor, WireframePlugin};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

// PLUGIN

pub struct ViewportPlugin;

impl Plugin for ViewportPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<WireframePlugin>() {
            app.add_plugins(WireframePlugin);
        }
        app.register_type::<TargetTransform>()
            .register_type::<ViewportRegion>()
            .init_resource::<ViewportRegion>()
            .add_systems(Startup, startup_system)
            .add_systems(
                Update,
                (
                    draw_grid_system,
                    draw_focal_point_system,
                    middle_mouse_actions_system
                        .run_if(input_pressed(MouseButton::Middle))
                        .after(mouse_over_viewport_system),
                    mouse_over_viewport_system,
                    keyboard_actions_system,
                    update_camera_system,
                ),
            );
    }
}

// RESOURCES

/// Unclaimed area of primary window is where the 3D viewport is visible.
#[derive(Debug, Default, Reflect, Resource)]
#[reflect(Resource)]
pub struct ViewportRegion {
    rect: Rect,
    mouse_position: Vec2,
}

impl ViewportRegion {
    pub fn set_rect(&mut self, rect: Rect) {
        self.rect = rect;
    }

    fn is_mouse_over(&self) -> bool {
        self.rect.contains(self.mouse_position)
    }
}

// COMPONENTS

#[derive(Component, Reflect)]
#[reflect(Component)]
/// Marker component for preview mesh.
pub struct PreviewMesh;

#[derive(Component, Reflect)]
#[reflect(Component)]
struct TargetTransform {
    translation: Vec3,
    rotation: Quat,
}

impl TargetTransform {
    const DEFAULT_ZOOM: f32 = 5.0;

    fn dolly(&mut self, factor: f32) {
        const MIN_DIST: f32 = 1.0;
        const MAX_DIST: f32 = 500.0;
        const DOLLY_SPEED: f32 = 0.5;
        let look_at_target: Vec3 = self.looking_at();
        // Translation from look_at_target to camera
        let rev_relative_position = self.translation - look_at_target;
        let new_distance = f32::max(
            MIN_DIST,
            f32::min(
                MAX_DIST,
                rev_relative_position.length() + factor * DOLLY_SPEED,
            ),
        );
        self.translation = look_at_target + rev_relative_position.normalize() * new_distance;
    }

    fn looking_at(&self) -> Vec3 {
        let ray = Ray3d::new(self.translation, self.rotation * Dir3::NEG_Z);
        //println!("{:?}", &ray);
        match ray.intersect_plane(Vec3::ZERO, InfinitePlane3d::new(Vec3::Y)) {
            Some(distance) => return ray.get_point(distance),
            None => panic!("Cannot calculate camera look at target."),
        }
    }

    fn orbit_xy(&mut self, x: f32, y: f32) {
        const MIN_Y: f32 = 1.0;
        const ORBIT_SPEED: f32 = 1.0 / 500.0;
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
        const PAN_SPEED: f32 = 0.35;
        let mut delta: Vec3 = Vec3::new(x, 0.0, y) * PAN_SPEED;
        // Take orientation into account but only around Y, we're panning across XZ.
        delta = Quat::from_rotation_y(self.rotation.to_euler(EulerRot::YXZ).0) * delta;
        self.translation += delta;
    }

    /// Reset focal point and zoom, but keep the orbit.
    fn reset(&mut self) {
        // Reset focal point.
        self.translation -= self.looking_at();

        // Reset zoom.
        self.translation = self.translation.normalize() * Self::DEFAULT_ZOOM;
    }
}

// SYSTEMS

fn draw_focal_point_system(
    mut gizmos: Gizmos,
    target_transform_query: Query<&TargetTransform, With<Camera>>,
) {
    if let Ok(target_transform) = target_transform_query.get_single() {
        gizmos.circle(
            Isometry3d::new(
                target_transform.looking_at(),
                Quat::from_rotation_x(-90.0f32.to_radians()),
            ),
            2.5f32,
            LinearRgba::RED,
        );
    }
}

fn draw_grid_system(mut gizmos: Gizmos) {
    gizmos
        .grid_3d(
            Isometry3d::IDENTITY,
            UVec3::new(10, 0, 10), // cells
            Vec3::splat(100.0),    // spacing
            LinearRgba::GREEN.with_alpha(0.8),
        )
        .outer_edges();

    gizmos
        .grid_3d(
            Isometry3d::IDENTITY,
            UVec3::new(100, 0, 100), // cells
            Vec3::splat(10.0),       // spacing
            LinearRgba::GREEN.with_alpha(0.15),
        )
        .outer_edges();
}

fn keyboard_actions_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut target_transform_query: Query<&mut TargetTransform, With<Camera>>,
) {
    if keyboard_input.just_pressed(KeyCode::Home) {
        target_transform_query.single_mut().reset();
    }
}

fn mouse_over_viewport_system(
    mut viewport: ResMut<ViewportRegion>,
    window: Query<&Window, With<PrimaryWindow>>,
) {
    match window.get_single() {
        Ok(window) => {
            if let Some(mouse_position) = window.physical_cursor_position() {
                viewport.mouse_position = mouse_position;
            }
        }
        Err(err) => {
            error!("{}", err.to_string());
            return;
        }
    }
}

fn middle_mouse_actions_system(
    mut mouse_motion_reader: EventReader<MouseMotion>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut target_transform_query: Query<&mut TargetTransform, With<Camera>>,
    viewport: Res<ViewportRegion>,
) {
    if !viewport.is_mouse_over() {
        return;
    }

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
        (true, true, false, true) => {
            // Dolly
            target_transform_query
                .get_single_mut()
                .expect("There must be one and only one camera.")
                .dolly(mouse_motion.y)
        }
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

/// Placeholder code to set up a basic 3D viewport.
fn startup_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    const CAMERA_INITIAL_TRANSLATION: Vec3 = Vec3::new(-50.0, 300.0, 200.0);
    const CAMERA_INITIAL_TARGET: Vec3 = Vec3::ZERO;

    // Create camera
    let transform = Transform::from_translation(CAMERA_INITIAL_TRANSLATION)
        .looking_at(CAMERA_INITIAL_TARGET, Vec3::Y);
    commands.spawn((
        Camera3d::default(),
        transform,
        TargetTransform {
            translation: transform.translation,
            rotation: transform.rotation,
        },
        Name::new("Camera"),
    ));

    // A pivot point so we can work in Z-up coords.
    commands
        .spawn((
            Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
            Visibility::default(),
            Name::new("Pivot Z-Up"),
        ))
        .with_children(|parent| {
            // Create ground quad
            parent.spawn((
                Name::new("Preview Mesh"),
                PreviewMesh,
                Mesh3d(meshes.add(Rectangle::new(1.0, 1.0))),
                MeshMaterial3d(
                    materials.add(
                        Color::Srgba(bevy::color::palettes::tailwind::AMBER_400).with_alpha(0.5),
                    ),
                ),
                // This is a quick and dirty way of rendering the wireframe,
                // but it is not capable of quad rendering.  To do proper quad
                // wireframes we either need to mark the diagonal edges
                // somehow (how?) and use a custom shader, or we need to use a
                // second edges-only mesh.
                Wireframe,
                WireframeColor {
                    color: bevy::color::palettes::tailwind::AMBER_400.into(),
                },
            ));
        });

    // Add light.
    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(-3.0, 5.0, -4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn update_camera_system(
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
            (1.0f32 - INTERPOLATION_FACTOR).powf(time.delta_secs() * 60.0),
        );

        transform.rotation = Quat::interpolate(
            &target_transform.rotation,
            &transform.rotation,
            (1.0f32 - INTERPOLATION_FACTOR).powf(time.delta_secs() * 60.0),
        );
    }
}
