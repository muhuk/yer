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

use core::f32::consts::FRAC_PI_2;

use bevy::math::Affine3A;
use bevy::pbr::wireframe::{Wireframe, WireframeColor, WireframePlugin};
use bevy::prelude::*;

use crate::theme;

const CAMERA_INITIAL_TARGET: Vec3 = Vec3::ZERO;
const CAMERA_INITIAL_TRANSLATION: Vec3 = Vec3::new(-50.0, 300.0, 200.0);
const DEFAULT_PREVIEW_FACE_COLOR: Color = Color::hsl(0.0, 0.0, 0.5);
const DEFAULT_PREVIEW_WIREFRAME_COLOR: Color = Color::hsl(0.0, 0.0, 0.85);
const PREVIEW_FACE_ALPHA: f32 = 0.65f32;
const VIEWPORT_LIGHT_POSITION: Vec3 = Vec3::new(-3.0, 5.0, -4.0);
const VIEWPORT_LIGHT_LOOK_AT_TARGET: Vec3 = Vec3::ZERO;

// PLUGIN

pub struct ViewportPlugin;

impl Plugin for ViewportPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<WireframePlugin>() {
            app.add_plugins(WireframePlugin::default());
        }
        app.add_systems(
            Startup,
            (
                create_viewport_root_system,
                (create_viewport_camera_system, create_preview_mesh_system),
            )
                .chain(),
        );
        app.add_systems(
            Update,
            (
                create_viewport_grid_system,
                draw_focal_point_system,
                keyboard_actions_system,
                update_camera_system,
                update_viewport_colors_system,
            ),
        );
    }
}

// COMPONENTS

#[derive(Component, Reflect)]
#[reflect(Component)]
/// Marker component for preview mesh.
pub struct PreviewMesh;

#[derive(Component, Reflect)]
#[reflect(Component)]
struct PivotZUp;

#[derive(Component, Reflect)]
#[reflect(Component)]
struct TargetTransform {
    translation: Vec3,
    rotation: Quat,
}

impl TargetTransform {
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
        self.translation = self.translation.normalize() * CAMERA_INITIAL_TRANSLATION.length();
    }
}

#[derive(Component, Reflect)]
#[reflect(Component)]
/// Marker component for viewport root.
struct Viewport;

#[derive(Component, Default, Deref, DerefMut, Reflect)]
#[reflect(Component)]
/// Whether the viewport is in focus or not.
struct ViewportFocus(#[deref] bool);

// OBSERVERS

fn viewport_background_sphere_pointer_drag_observer(
    drag: On<Pointer<Drag>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut target_transform_query: Query<&mut TargetTransform, With<Camera>>,
) {
    if drag.button == PointerButton::Middle {
        if let Ok(mut target_transform) = target_transform_query.single_mut() {
            match (
                keyboard_input.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]),
                keyboard_input.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]),
            ) {
                (false, false) => {
                    target_transform.orbit_xy(-drag.delta.x, -drag.delta.y);
                }
                (true, false) => {
                    target_transform.pan_xy(-drag.delta.x, -drag.delta.y);
                }
                (false, true) => {
                    target_transform.dolly(drag.delta.y);
                }
                (true, true) => (),
            }
        } else {
            error!("Cannot access viewport target transform.");
        }
    }
}

// SYSTEMS

fn create_preview_mesh_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    pivot_z_up: Single<Entity, With<PivotZUp>>,
) {
    // A pivot point so we can work in Z-up coords.
    commands.spawn((
        Name::new("Preview Mesh"),
        PreviewMesh,
        ChildOf(*pivot_z_up),
        Mesh3d(meshes.add(Rectangle::new(1.0, 1.0))),
        MeshMaterial3d(materials.add(DEFAULT_PREVIEW_FACE_COLOR.with_alpha(PREVIEW_FACE_ALPHA))),
        Pickable::IGNORE,
        // This is a quick and dirty way of rendering the wireframe,
        // but it is not capable of quad rendering.  To do proper quad
        // wireframes we either need to mark the diagonal edges
        // somehow (how?) and use a custom shader, or we need to use a
        // second edges-only mesh.
        Wireframe,
        WireframeColor {
            color: DEFAULT_PREVIEW_WIREFRAME_COLOR,
        },
    ));
}

fn create_viewport_camera_system(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    viewport_root: Single<Entity, With<Viewport>>,
) -> Result<(), BevyError> {
    let (camera_entity, far): (Entity, f32) = {
        let transform = Transform::from_translation(CAMERA_INITIAL_TRANSLATION)
            .looking_at(CAMERA_INITIAL_TARGET, Vec3::Y);
        let projection = Projection::default();
        let far = projection.far();
        let entity = commands
            .spawn((
                Name::new("Viewport Camera"),
                ChildOf(*viewport_root),
                Camera3d::default(),
                projection,
                transform,
                TargetTransform {
                    translation: transform.translation,
                    rotation: transform.rotation,
                },
            ))
            .id();
        (entity, far)
    };

    // Add an invisible background object to capture picking events.
    commands
        .spawn((
            Name::new("Viewport Background Sphere"),
            Mesh3d(
                meshes.add(
                    Sphere::new(far * (1.0 - f32::EPSILON))
                        .mesh()
                        .uv(32, 18)
                        .with_computed_smooth_normals()
                        .with_inverted_winding()?,
                ),
            ),
            MeshMaterial3d(materials.add(StandardMaterial {
                alpha_mode: AlphaMode::Multiply,
                unlit: true,
                base_color: Color::Srgba(Srgba::NONE),
                ..default()
            })),
            ChildOf(camera_entity),
        ))
        .observe(viewport_background_sphere_pointer_drag_observer)
        .observe(
            |_over: On<Pointer<Over>>, mut viewport_focus: Single<&mut ViewportFocus>| {
                ***viewport_focus = true;
            },
        )
        .observe(
            |_out: On<Pointer<Out>>, mut viewport_focus: Single<&mut ViewportFocus>| {
                ***viewport_focus = false;
            },
        );

    // Add light.
    commands.spawn((
        ChildOf(*viewport_root),
        DirectionalLight::default(),
        Transform::from_translation(VIEWPORT_LIGHT_POSITION)
            .looking_at(VIEWPORT_LIGHT_LOOK_AT_TARGET, Vec3::Y),
    ));

    Ok(())
}

// TODO: If/when https://github.com/bevyengine/bevy/issues/16041 is implemented
//       we won't need to run this system in Update.
fn create_viewport_grid_system(
    mut commands: Commands,
    mut gizmo_assets: ResMut<Assets<GizmoAsset>>,
    mut grid_created: Local<bool>,
    pivot_z_up: Single<Entity, With<PivotZUp>>,
    theme: Res<theme::Theme>,
    theme_colors: Res<Assets<theme::ThemeColors>>,
) {
    if *grid_created {
        return;
    }

    if let Some(colors) = theme_colors.get(&theme.colors) {
        debug!("Creating viewport grid.");

        let color_minor = colors.secondary_color;
        let color_major = colors.secondary_alt_color;

        // Grid
        {
            let mut gizmo = GizmoAsset::default();

            // Rotate around X axis for -90 degrees to make it Y up.
            // This may or may not be correct.
            let isometry = Isometry3d::from_rotation(Quat::from_rotation_x(-FRAC_PI_2));

            gizmo
                .grid_3d(
                    isometry,
                    UVec3::new(10, 0, 10), // cells
                    Vec3::splat(100.0),    // spacing
                    color_major.with_alpha(0.3),
                )
                .outer_edges();

            gizmo
                .grid_3d(
                    isometry,
                    UVec3::new(100, 0, 100), // cells
                    Vec3::splat(10.0),       // spacing
                    color_minor.with_alpha(0.15),
                )
                .outer_edges();

            commands.spawn((
                ChildOf(*pivot_z_up),
                Gizmo {
                    handle: gizmo_assets.add(gizmo),
                    line_config: GizmoLineConfig::default(),
                    ..default()
                },
            ));
        }

        // Axis Lines
        {
            let mut gizmo = GizmoAsset::default();

            // X axis line
            gizmo.line(
                Vec3::NEG_X * 500.0,
                Vec3::X * 500.0,
                color_major.with_alpha(0.3),
            );

            // Y axis line
            gizmo.line(
                Vec3::NEG_Y * 500.0,
                Vec3::Y * 500.0,
                color_major.with_alpha(0.3),
            );

            commands.spawn((
                ChildOf(*pivot_z_up),
                Gizmo {
                    handle: gizmo_assets.add(gizmo),
                    line_config: GizmoLineConfig {
                        width: 5.0,
                        ..default()
                    },
                    depth_bias: -0.03,
                },
            ));
        }

        *grid_created = true;
    }
}

fn create_viewport_root_system(mut commands: Commands) {
    commands.spawn((
        Name::new("Viewport"),
        Viewport,
        ViewportFocus::default(),
        Transform::default(),
        Visibility::default(),
        children![(
            Name::new("Pivot Z-Up"),
            PivotZUp,
            Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
            Visibility::default(),
        )],
    ));
}

fn draw_focal_point_system(
    mut gizmos: Gizmos,
    target_transform_query: Query<&TargetTransform, With<Camera>>,
    theme: Res<theme::Theme>,
    theme_colors: Res<Assets<theme::ThemeColors>>,
) {
    let color = theme_colors.get(&theme.colors).unwrap().secondary_alt_color;
    if let Ok(target_transform) = target_transform_query.single() {
        gizmos.circle(
            Isometry3d::new(
                target_transform.looking_at(),
                Quat::from_rotation_x(-90.0f32.to_radians()),
            ),
            2.5f32,
            color,
        );
    }
}

fn keyboard_actions_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut target_transform_query: Query<&mut TargetTransform, With<Camera>>,
    viewport_focus: Single<&ViewportFocus>,
) {
    if !***viewport_focus {
        return;
    }

    if keyboard_input.just_pressed(KeyCode::Home) {
        target_transform_query.single_mut().unwrap().reset();
    }
}

fn update_camera_system(
    time: Res<Time>,
    mut camera_query: Single<(&TargetTransform, &mut Transform), With<Camera>>,
) {
    const INTERPOLATION_FACTOR: f32 = 1.0f32 / 3.0f32;

    let (target_transform, ref mut transform) = *camera_query;
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

/// Update preview mesh colors and clear color when the theme changes.
fn update_viewport_colors_system(
    mut clear_color: ResMut<ClearColor>,
    mut preview_mesh: Single<
        (&mut MeshMaterial3d<StandardMaterial>, &mut WireframeColor),
        With<PreviewMesh>,
    >,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    theme: Res<theme::Theme>,
    theme_colors: Res<Assets<theme::ThemeColors>>,
) {
    if !theme.is_changed() {
        return;
    }

    debug!("Updating preview mesh colors from theme.");
    match (
        theme_colors.get(&theme.colors),
        standard_materials.get_mut(&preview_mesh.0 .0),
    ) {
        (Some(colors), Some(standard_material)) => {
            standard_material.base_color = colors.primary_color.with_alpha(PREVIEW_FACE_ALPHA);
            preview_mesh.1.color = colors.primary_color;
            clear_color.0 = colors.bg_color;
        }
        _ => {
            error!("Cannot update preview mesh colors from theme.");
        }
    }
}
