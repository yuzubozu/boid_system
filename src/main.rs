use bevy::{
    prelude::*,
    render::prelude::Mesh,
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
    window::{PresentMode,PrimaryWindow}
};
use rand::Rng;
use std::{collections::HashMap, f32::consts::PI};
//const for window
const WINDOW_WIDTH: f32 = 700.;
const WINDOW_HEIGHT: f32 = 500.;
const WINDOW_MARGIN: f32 = 20.;

//const for display
const FISH_WIDTH: f32 = 1.;
const FISH_HEIGHT: f32 = 3.;
const FISH_LIGHTNESS: f32 = 0.7;

//const for initial conditions
const SPAWN_RANGE_HEIGHT: f32 = 500.;
const SPAWN_RANGE_WIDTH: f32 = 500.;

//const for simulation
const SEPARATION_COEFFICIENT: f32 = 500.;
const SEPARATION_SIGHT_RAD: f32 = 120.;
const SEPARATION_SIGHT_DEGREE: f32 = PI * 360. / 180.;
const SEPARATION_MIN_RANGE:f32 = 0.00001;

const ALIGNMENT_COEFFICIENT: f32 = 1.;
const ALIGNMENT_SIGHT_RAD: f32 = 30.;
const ALIGNMENT_SIGHT_DEGREE: f32 = PI * 120. / 180.;

const COHESION_COEFFICIENT: f32 = 10.;
const COHESION_SIGHT_RAD: f32 = 80.;
const COHESION_SIGHT_DEGREE: f32 = PI * 300. / 180.;

const MOUSE_COEFFICIENT: f32 = 30000.0;
const MOUSE_SIGHT_RAD: f32 = 240.;
const MOUSE_SIGHT_DEGREE: f32 = PI * 120. / 180.;

const FISH_NUM: usize = 200;
const MAX_VEL: f32 = 100.;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "boid system".into(),
                resolution: (WINDOW_WIDTH, WINDOW_HEIGHT).into(),
                present_mode: PresentMode::AutoVsync,
                // Tells wasm to resize the window according to the available canvas
                canvas: Some("#mygame-canvas".into()),
                // Tells wasm not to override default event handling, like F5, Ctrl+R etc.
                prevent_default_event_handling: false,
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, (setup_camera, setup_fish))
        .add_systems(Update, (boid_system, apply_force_system, update_fish))
        .run();
}

/**
 * structs,components
 */
#[derive(Default, Bundle)]
struct FishBundle {
    id: ID,
    position: Position,
    velocity: Velocity,
    force: Force,
    _marker: FishMarker,
}

#[derive(Default, Component, PartialEq, Eq, Hash, Deref, Clone, Copy)]
struct ID(usize);

#[derive(Default, Component)]
struct FishMarker;

#[derive(Default, Component, Deref, Clone, Copy)]
struct Position(Vec2);

trait VecCalc
where
    Self: Sized,
{
    fn new(v: Vec2) -> Self;
    fn origin() -> Self {
        Self::new(Vec2::new(0., 0.))
    }
    fn get_vec2(&self) -> Vec2;

    fn distance(&self, target: impl VecCalc) -> f32 {
        self.get_vec2().distance(target.get_vec2())
    }

    fn add(&self, target: impl VecCalc) -> Self {
        Self::new(self.get_vec2() + target.get_vec2())
    }

    fn diff(&self, target: impl VecCalc) -> Self {
        Self::new(self.get_vec2() - target.get_vec2())
    }

    fn div(&self, num: f32) -> Self {
        Self::new(self.get_vec2() / num)
    }

    fn multiply(&self, num: f32) -> Self {
        Self::new(self.get_vec2() * num)
    }
}

impl VecCalc for Position {
    fn get_vec2(&self) -> Vec2 {
        self.0
    }
    fn new(v: Vec2) -> Self {
        Self(v)
    }
}

impl Position {
    fn is_in_range(self, target: Position, rad: f32, self_v: Velocity, sight_degree: f32) -> bool {
        let angle = (target.get_vec2() - self.get_vec2()).to_angle() - self_v.get_vec2().to_angle();
        let angle = angle.abs().min(2. * PI - angle.abs());
        angle.abs() <= sight_degree / 2. && self.distance(target) <= rad
    }
}

#[derive(Default, Component, Deref, Clone, Copy)]
struct Velocity(Vec2);

impl VecCalc for Velocity {
    fn get_vec2(&self) -> Vec2 {
        self.0
    }
    fn new(v: Vec2) -> Self {
        Self(v)
    }
}

#[derive(Default, Component, Deref, Clone, Copy)]
struct Force(Vec2);

impl VecCalc for Force {
    fn get_vec2(&self) -> Vec2 {
        self.0
    }
    fn new(v: Vec2) -> Self {
        Self(v)
    }
}

/**
 * functions to calculate boid system
 */
fn separation(self_pos: Position, tg_pos: Position) -> Force {
    let dist = self_pos.distance(tg_pos);
    if dist > 0. {
        Force(self_pos.diff(tg_pos).div(dist * dist).get_vec2())
    } else {
        Force::origin()
    }
}

fn alignment(self_v: Velocity, v_avg: Velocity) -> Force {
    Force(v_avg.diff(self_v).get_vec2())
}

fn cohesion(self_pos: Position, cent_pos: Position) -> Force {
    Force(cent_pos.diff(self_pos).get_vec2())
}

/**
 * functions to calculate boundary condition
 */

fn is_in_window_contents_left(pos_x: f32) -> bool {
    pos_x >= -1. * WINDOW_WIDTH / 2. + WINDOW_MARGIN
}

fn is_in_window_contents_right(pos_x: f32) -> bool {
    pos_x <= WINDOW_WIDTH / 2. - WINDOW_MARGIN
}

fn is_in_window_contents_top(pos_y: f32) -> bool {
    pos_y <= WINDOW_HEIGHT / 2. - WINDOW_MARGIN
}

fn is_in_window_contents_bottom(pos_y: f32) -> bool {
    pos_y >= -1. * WINDOW_HEIGHT / 2. + WINDOW_MARGIN
}

fn bound(pos: Position, vel: Velocity) -> (Position, Velocity) {
    let mut pos_vec = pos.get_vec2();
    let mut vel_vec = vel.get_vec2();

    if !is_in_window_contents_left(pos.x) || !is_in_window_contents_right(pos.x) {
        let x_border = if !is_in_window_contents_left(pos.x) {
            -1. * WINDOW_WIDTH / 2. + WINDOW_MARGIN
        } else {
            WINDOW_WIDTH / 2. - WINDOW_MARGIN
        };
        vel_vec.x = -1. * vel_vec.x;
        pos_vec.x = 2. * x_border - pos_vec.x;
    }

    if !is_in_window_contents_top(pos.y) || !is_in_window_contents_bottom(pos.y) {
        let y_border = if !is_in_window_contents_top(pos.y) {
            WINDOW_HEIGHT / 2. - WINDOW_MARGIN
        } else {
            -1. * WINDOW_HEIGHT / 2. + WINDOW_MARGIN
        };
        vel_vec.y = -1. * vel_vec.y;
        pos_vec.y = 2. * y_border - pos_vec.y;
    }

    if vel_vec.length() >= MAX_VEL {
        vel_vec *= MAX_VEL / vel_vec.length();
    }

    (Position(pos_vec), Velocity(vel_vec))
}

/**
 * functions to calculate rotate and color
 */
//radian angle
fn quat_to_radian(quat: Quat) -> f32 {
    let (vec_dir, angle) = quat.to_axis_angle();
    angle * vec_dir.z
}

fn quat_to_hue(quat: Quat) -> f32 {
    (quat_to_radian(quat) / PI * 180.) + 180.
}

fn create_fish_color(fish_quat: Quat, fish_vel: Vec2) -> Color {
    Color::hsl(
        quat_to_hue(fish_quat),
        fish_vel.length() / MAX_VEL,
        FISH_LIGHTNESS,
    )
}

fn create_fish_quat(fish_vel: Vec2) -> Quat {
    Quat::from_rotation_arc_2d(Vec2::X, fish_vel.normalize())
}

/**
 * functions to setup
 */
fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle {
        transform: Transform::from_xyz(0., 0., 5.),
        ..Default::default()
    });
}

fn setup_fish(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let mut rng = rand::thread_rng();
    //create fish
    for i in 0..FISH_NUM {
        let fish_shape = Mesh2dHandle(meshes.add(Ellipse::new(FISH_HEIGHT, FISH_WIDTH)));
        let position_vec = Vec2::from((
            rng.gen::<f32>() * SPAWN_RANGE_WIDTH - SPAWN_RANGE_WIDTH / 2.,
            rng.gen::<f32>() * SPAWN_RANGE_HEIGHT - SPAWN_RANGE_HEIGHT / 2.,
        ));
        let velocity_vec = Vec2::from((
            (rng.gen::<f32>() - 0.5) * MAX_VEL,
            (rng.gen::<f32>() - 0.5) * MAX_VEL,
        ));
        let fish_quat = create_fish_quat(velocity_vec);

        commands.spawn((
            MaterialMesh2dBundle {
                mesh: fish_shape,
                material: materials.add(create_fish_color(fish_quat, velocity_vec)),
                transform: Transform::from_xyz(position_vec.x, position_vec.y, 0.0).with_rotation(
                    Quat::from_rotation_arc_2d(Vec2::X, velocity_vec.normalize()),
                ),
                ..default()
            },
            FishBundle {
                id: ID(i),
                position: Position(position_vec),
                velocity: Velocity(velocity_vec),
                force: Force::origin(),
                _marker: FishMarker {},
            },
        ));
    }
}

/**
 * functions to update
 */
fn update_fish(
    mut query: Query<(&mut Transform, &Handle<ColorMaterial>, &Position, &Velocity)>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    query
        .iter_mut()
        .for_each(|(mut transform, color, pos, vel)| {
            let pos_vec = pos.get_vec2();
            let vel_vec = vel.get_vec2();
            let fish_quat = create_fish_quat(vel_vec);
            let color_mat = materials.get_mut(color).unwrap();
            color_mat.color = create_fish_color(fish_quat, vel_vec);
            *transform = Transform::from_xyz(pos_vec.x, pos_vec.y, 0.).with_rotation(fish_quat);
        });
}

/**
 * calculation of force for each fish feat boid system
 */
fn boid_system(
    query: Query<(&ID, &Position, &Velocity), With<FishMarker>>,
    mut update_query: Query<(&ID, &mut Force), With<FishMarker>>,
    q_windows: Query<&Window, With<PrimaryWindow>>,
    buttons: Res<ButtonInput<MouseButton>>,
) {
    let mut force_map = HashMap::new();

    //calc force
    query.iter().for_each(|(id_base, pos_base, vel_base)| {
        let (
            in_range_alignment_fish,
            in_range_cohesion_fish,
            in_range_sum_separation_force,
            in_range_sum_alignment_vel,
            in_range_sum_cohesion_pos,
        ) = query.iter().fold(
            (
                0.,
                0.,
                Force::origin(),
                Velocity::origin(),
                Position::origin(),
            ),
            |(
                mut sum_alignment_fish,
                mut sum_cohesion_fish,
                mut sum_separation_force,
                mut sum_alignment_vel,
                mut sum_cohesion_pos,
            ),
             (id_target, pos_target, vel_target)| {
                if pos_base.is_in_range(
                    *pos_target,
                    SEPARATION_SIGHT_RAD,
                    *vel_base,
                    SEPARATION_SIGHT_DEGREE,
                ) && id_base != id_target
                  && !pos_base.is_in_range(
                    *pos_target,
                    SEPARATION_MIN_RANGE,
                    *vel_base,
                    SEPARATION_SIGHT_DEGREE,
                )
                {
                    sum_separation_force =
                        sum_separation_force.add(separation(*pos_base, *pos_target));
                }

                if pos_base.is_in_range(
                    *pos_target,
                    ALIGNMENT_SIGHT_RAD,
                    *vel_base,
                    ALIGNMENT_SIGHT_DEGREE,
                ) && id_base != id_target
                {
                    sum_alignment_vel = sum_alignment_vel.add(*vel_target);
                    sum_alignment_fish += 1.;
                }
                if pos_base.is_in_range(
                    *pos_target,
                    COHESION_SIGHT_RAD,
                    *vel_base,
                    COHESION_SIGHT_DEGREE,
                ) && id_base != id_target
                {
                    sum_cohesion_pos = sum_cohesion_pos.add(*pos_target);
                    sum_cohesion_fish += 1.;
                }

                (
                    sum_alignment_fish,
                    sum_cohesion_fish,
                    sum_separation_force,
                    sum_alignment_vel,
                    sum_cohesion_pos,
                )
            },
        );

        let mut force = Force::origin();
        force = force
            .add(in_range_sum_separation_force)
            .multiply(SEPARATION_COEFFICIENT);
        if in_range_alignment_fish > 0. {
            force = force.add(
                alignment(
                    *vel_base,
                    in_range_sum_alignment_vel.div(in_range_alignment_fish),
                )
                .multiply(ALIGNMENT_COEFFICIENT),
            );
        }
        if in_range_cohesion_fish > 0. {
            force = force.add(
                cohesion(
                    *pos_base,
                    in_range_sum_cohesion_pos.div(in_range_cohesion_fish),
                )
                .multiply(COHESION_COEFFICIENT),
            );
        }

        let mut mouse_force = Force::origin();
        if let Some(mouse_position_vec) = q_windows.single().cursor_position() {
            let mut mouse_position_vec_fixed = mouse_position_vec-Vec2::new(WINDOW_WIDTH/2.,WINDOW_HEIGHT/2.);
            mouse_position_vec_fixed.y = -1. * mouse_position_vec_fixed.y;
            let mouse_position = Position::new(mouse_position_vec_fixed);
            if pos_base.is_in_range(
                mouse_position,
                MOUSE_SIGHT_RAD,
                *vel_base,
                MOUSE_SIGHT_DEGREE,
            )
            {
                if buttons.pressed(MouseButton::Right) {
                    mouse_force = mouse_force.add(separation(*pos_base, mouse_position));
                }
                if buttons.pressed(MouseButton::Left) {
                    mouse_force = mouse_force.diff(separation(*pos_base, mouse_position));
                }
            }
        }

        force = force.add(mouse_force.multiply(MOUSE_COEFFICIENT));

        force_map.insert(id_base, force);
    });

    update_query.iter_mut().for_each(|(id, mut force)| {
        *force = *force_map.get(id).unwrap();
    });
}

/**
 * function to apply force
 */
fn apply_force_system(
    time: Res<Time>,
    mut query: Query<(&mut Position, &mut Velocity, &Force), With<FishMarker>>,
) {
    query.iter_mut().for_each(|(mut pos, mut vel, force)| {
        let vel_next = vel.as_ref().add(force.multiply(time.delta_seconds()));
        let next_pos = pos.as_ref().add(vel.multiply(time.delta_seconds()));
        (*pos, *vel) = bound(next_pos, vel_next);
    });
}