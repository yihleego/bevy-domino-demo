use avian3d::math::AdjustPrecision;
use avian3d::prelude::*;
use bevy::color::palettes::css;
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::{Backends, RenderCreation, WgpuSettings};
use bevy::scene::SceneInstanceReady;
use bevy_tnua::builtins::*;
use bevy_tnua::math::AsF32;
use bevy_tnua::{
    TnuaAnimatingState, TnuaAnimatingStateDirective, TnuaConfigModifier,
    builtins::TnuaBuiltinJumpMemory, prelude::*,
};
use bevy_tnua_avian3d::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI};

const GLTF_PATH: &str = "models/characters/Knight.glb";
const ANIMATIONS: [&str; 76] = [
    "1H_Melee_Attack_Chop",
    "1H_Melee_Attack_Slice_Diagonal",
    "1H_Melee_Attack_Slice_Horizontal",
    "1H_Melee_Attack_Stab",
    "1H_Ranged_Aiming",
    "1H_Ranged_Reload",
    "1H_Ranged_Shoot",
    "1H_Ranged_Shooting",
    "2H_Melee_Attack_Chop",
    "2H_Melee_Attack_Slice",
    "2H_Melee_Attack_Spin",
    "2H_Melee_Attack_Spinning",
    "2H_Melee_Attack_Stab",
    "2H_Melee_Idle",
    "2H_Ranged_Aiming",
    "2H_Ranged_Reload",
    "2H_Ranged_Shoot",
    "2H_Ranged_Shooting",
    "Block",
    "Block_Attack",
    "Block_Hit",
    "Blocking",
    "Cheer",
    "Death_A",
    "Death_A_Pose",
    "Death_B",
    "Death_B_Pose",
    "Dodge_Backward",
    "Dodge_Forward",
    "Dodge_Left",
    "Dodge_Right",
    "Dualwield_Melee_Attack_Chop",
    "Dualwield_Melee_Attack_Slice",
    "Dualwield_Melee_Attack_Stab",
    "Hit_A",
    "Hit_B",
    "Idle",
    "Interact",
    "Jump_Full_Long",
    "Jump_Full_Short",
    "Jump_Idle",
    "Jump_Land",
    "Jump_Start",
    "Lie_Down",
    "Lie_Idle",
    "Lie_Pose",
    "Lie_StandUp",
    "PickUp",
    "Running_A",
    "Running_B",
    "Running_Strafe_Left",
    "Running_Strafe_Right",
    "Sit_Chair_Down",
    "Sit_Chair_Idle",
    "Sit_Chair_Pose",
    "Sit_Chair_StandUp",
    "Sit_Floor_Down",
    "Sit_Floor_Idle",
    "Sit_Floor_Pose",
    "Sit_Floor_StandUp",
    "Spellcast_Long",
    "Spellcast_Raise",
    "Spellcast_Shoot",
    "Spellcasting",
    "T-Pose",
    "Throw",
    "Unarmed_Idle",
    "Unarmed_Melee_Attack_Kick",
    "Unarmed_Melee_Attack_Punch_A",
    "Unarmed_Melee_Attack_Punch_B",
    "Unarmed_Pose",
    "Use_Item",
    "Walking_A",
    "Walking_B",
    "Walking_Backwards",
    "Walking_C",
];

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    fit_canvas_to_parent: true,
                    prevent_default_event_handling: false,
                    ..default()
                }),
                ..default()
            }),
            PhysicsPlugins::default(),
            PhysicsDebugPlugin::default(),
            TnuaControllerPlugin::<ControlScheme>::new(FixedUpdate),
            TnuaAvian3dPlugin::new(FixedUpdate),
        ))
        .add_systems(Startup, (setup_level, setup_player, setup_domino_scene))
        .add_systems(
            FixedUpdate,
            (apply_controls).in_set(TnuaUserControlsSystems),
        )
        .add_systems(Update, (handle_animating, orbit_camera))
        .run();
}

#[derive(Clone, Copy)]
enum DominoType {
    Domino,
    Ball,
}

struct DominoElement {
    pos: Vec3,
    rot: Quat,
    kind: DominoType,
}

fn setup_domino_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let platform_mat = materials.add(Color::from(css::DARK_SLATE_GRAY));

    // High platform
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(4.0, 1.0, 6.0))),
        MeshMaterial3d(platform_mat.clone()),
        Transform::from_xyz(5.0, 2.0, -6.0),
        RigidBody::Static,
        Collider::cuboid(4.0, 1.0, 6.0),
    ));

    // Steps
    for i in 0..5 {
        let h = 2.0 - (i as f32) * 0.4;
        let z = -2.5 + (i as f32) * 1.0;
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(2.0, h, 1.0))),
            MeshMaterial3d(platform_mat.clone()),
            Transform::from_xyz(5.0, h / 2.0, z), // Bottom at 0
            RigidBody::Static,
            Collider::cuboid(2.0, h, 1.0),
        ));
    }

    // --- Generate Domino Vector ---
    let mut elements: Vec<DominoElement> = Vec::new();
    let domino_rot = Quat::from_rotation_y(FRAC_PI_2);

    // 1. Line on High Platform
    for i in 0..6 {
        let z = -6.5 + (i as f32) * 0.6; // -6.5, -5.9, -5.3, -4.7, -4.1, -3.5
        elements.push(DominoElement {
            pos: Vec3::new(5.0, 3.0, z),
            rot: domino_rot,
            kind: DominoType::Domino,
        });
    }

    // 2. Down the stairs
    for i in 0..5 {
        let h = 2.0 - (i as f32) * 0.4;
        let z = -2.5 + (i as f32) * 1.0;
        elements.push(DominoElement {
            pos: Vec3::new(5.0, h + 0.5, z),
            rot: domino_rot,
            kind: DominoType::Domino,
        });
    }

    // 3. Continue on ground
    for i in 0..8 {
        elements.push(DominoElement {
            pos: Vec3::new(5.0, 0.5, 2.5 + (i as f32) * 0.6),
            rot: domino_rot,
            kind: DominoType::Domino,
        });
    }

    // 4. End with a Ball
    let ball_z = 2.5 + 8.0 * 0.6 + 0.6; // approx 7.9
    elements.push(DominoElement {
        pos: Vec3::new(5.0, 0.4, ball_z), // Radius 0.4
        rot: Quat::IDENTITY,
        kind: DominoType::Ball,
    });

    // 5. Post-Ball Straight Line
    let line_start_z = ball_z + 0.9;
    let line_count = 6;
    for i in 0..line_count {
        elements.push(DominoElement {
            pos: Vec3::new(5.0, 0.5, line_start_z + (i as f32) * 0.6),
            rot: domino_rot,
            kind: DominoType::Domino,
        });
    }

    // 6. Circular Split
    let circle_center_z = 15.4;
    let circle_radius = 3.0;

    let circle_steps = 16;
    let angle_step = PI / (circle_steps as f32);

    for i in 0..circle_steps {
        let angle = -FRAC_PI_2 + (i as f32 + 0.5) * angle_step;

        let sin_a = angle.sin();
        let cos_a = angle.cos();

        let pos_r = Vec3::new(
            5.0 + circle_radius * cos_a,
            0.5,
            circle_center_z + circle_radius * sin_a,
        );
        let tangent_r = Vec3::new(-sin_a, 0.0, cos_a).normalize();
        let rot_r = Quat::from_rotation_arc(Vec3::X, tangent_r);

        elements.push(DominoElement {
            pos: pos_r,
            rot: rot_r,
            kind: DominoType::Domino,
        });

        let pos_l = Vec3::new(
            5.0 - circle_radius * cos_a,
            0.5,
            circle_center_z + circle_radius * sin_a,
        );
        let tangent_l = Vec3::new(sin_a, 0.0, cos_a).normalize();
        let rot_l = Quat::from_rotation_arc(Vec3::X, tangent_l);

        elements.push(DominoElement {
            pos: pos_l,
            rot: rot_l,
            kind: DominoType::Domino,
        });
    }

    // 7. Final Straight Line (Merge)
    let final_line_z = circle_center_z + circle_radius + 0.6;
    for i in 0..10 {
        elements.push(DominoElement {
            pos: Vec3::new(5.0, 0.5, final_line_z + (i as f32) * 0.6),
            rot: domino_rot,
            kind: DominoType::Domino,
        });
    }

    // --- Spawn Elements ---
    let domino_mesh = meshes.add(Cuboid::new(0.1, 1.0, 0.5));
    let domino_mat = materials.add(Color::from(css::ORANGE_RED));

    let ball_mesh = meshes.add(Sphere::new(0.4));
    let ball_mat = materials.add(Color::from(css::DODGER_BLUE));

    for elem in elements {
        match elem.kind {
            DominoType::Domino => {
                commands.spawn((
                    Mesh3d(domino_mesh.clone()),
                    MeshMaterial3d(domino_mat.clone()),
                    Transform::from_translation(elem.pos).with_rotation(elem.rot),
                    RigidBody::Dynamic,
                    Collider::cuboid(0.1, 1.0, 0.5),
                    Mass(1.0),
                    Friction::new(0.5),
                ));
            }
            DominoType::Ball => {
                commands.spawn((
                    Mesh3d(ball_mesh.clone()),
                    MeshMaterial3d(ball_mat.clone()),
                    Transform::from_translation(elem.pos).with_rotation(elem.rot),
                    RigidBody::Dynamic,
                    Collider::sphere(0.4),
                    Mass(2.0),
                    Restitution::new(0.7), // Bouncy
                ));
            }
        }
    }
}

#[derive(TnuaScheme)]
#[scheme(basis = TnuaBuiltinWalk)]
enum ControlScheme {
    Jump(TnuaBuiltinJump),
    Dash(TnuaBuiltinDash),
}

impl Default for ControlSchemeConfig {
    fn default() -> Self {
        ControlSchemeConfig {
            basis: TnuaBuiltinWalkConfig {
                speed: 10.0,
                turning_angvel: 15.0,
                float_height: 0.01,
                ..default()
            },
            jump: TnuaBuiltinJumpConfig {
                height: 5.0,
                ..default()
            },
            dash: TnuaBuiltinDashConfig {
                horizontal_distance: 5.0,
                vertical_distance: 0.0,
                ..default()
            },
        }
    }
}

pub struct SlowDownWhileCrouching(pub bool);

impl TnuaConfigModifier<TnuaBuiltinWalkConfig> for SlowDownWhileCrouching {
    fn modify_config(&self, config: &mut TnuaBuiltinWalkConfig) {
        if self.0 {
            config.speed *= 0.5;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AnimationState {
    Standing,
    Running(f32),
    Jumping,
    Falling,
    Landing,
    Dashing,
}

#[derive(Resource)]
struct Animations {
    animations: HashMap<String, AnimationNodeIndex>,
    graph_handle: Handle<AnimationGraph>,
}

fn setup_level(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((Camera3d::default(), ThirdPersonCamera::default()));

    commands.spawn((PointLight::default(), Transform::from_xyz(5.0, 5.0, 5.0)));

    commands.spawn((
        DirectionalLight {
            illuminance: 4000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::default().looking_at(-Vec3::Y, Vec3::Z),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(512.0, 512.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        RigidBody::Static,
        Collider::half_space(Vec3::Y),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(10.0, 1.0, 10.0))),
        MeshMaterial3d(materials.add(Color::from(css::GRAY))),
        Transform::from_xyz(-6.0, 2.0, 0.0),
        RigidBody::Static,
        Collider::cuboid(10.0, 1.0, 10.0),
    ));
}

#[derive(Component)]
struct Player;

fn setup_player(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut control_scheme_configs: ResMut<Assets<ControlSchemeConfig>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    let clips = (0..ANIMATIONS.len())
        .into_iter()
        .map(|i| asset_server.load(GltfAssetLabel::Animation(i).from_asset(GLTF_PATH)))
        .collect::<Vec<_>>();

    let (graph, node_indices) = AnimationGraph::from_clips(clips);

    let mut animations = HashMap::<String, AnimationNodeIndex>::new();
    for i in 0..ANIMATIONS.len() {
        let node_index = node_indices[i];
        let name = ANIMATIONS[i].to_string();
        animations.insert(name, node_index);
    }

    let graph_handle = graphs.add(graph);
    commands.insert_resource(Animations {
        animations,
        graph_handle,
    });

    commands
        .spawn((
            Player,
            Transform::from_xyz(0.0, 2.0, 0.0),
            TnuaAnimatingState::<AnimationState>::default(),
            RigidBody::Dynamic,
            Collider::capsule_endpoints(0.5, Vec3::Y * (1.0 + 0.5), Vec3::Y * 0.5),
            TnuaController::<ControlScheme>::default(),
            TnuaConfig::<ControlScheme>(control_scheme_configs.add(ControlSchemeConfig::default())),
            TnuaAvian3dSensorShape(Collider::cylinder(0.49, 0.0)),
            LockedAxes::ROTATION_LOCKED.unlock_rotation_y(),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(GLTF_PATH))),
                    Transform::from_rotation(Quat::from_rotation_y(PI)),
                ))
                .observe(play_animation_when_ready);
        });
}

fn play_animation_when_ready(
    scene_ready: On<SceneInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    mut players: Query<(Entity, &mut AnimationPlayer)>,
    animations: Res<Animations>,
) {
    for child in children.iter_descendants(scene_ready.entity) {
        if let Ok(_) = players.get_mut(child) {
            commands
                .entity(child)
                .insert(AnimationGraphHandle(animations.graph_handle.clone()));
        }
    }
}

fn apply_controls(
    keyboard: Res<ButtonInput<KeyCode>>,
    camera: Single<&Transform, With<Camera3d>>,
    mut query: Query<(&mut TnuaController<ControlScheme>,)>,
) {
    let Ok((mut controller,)) = query.single_mut() else {
        return;
    };
    controller.initiate_action_feeding();

    let mut direction = Vec3::ZERO;
    if keyboard.pressed(KeyCode::KeyW) {
        direction.z += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        direction.z -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        direction.x -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        direction.x += 1.0;
    }

    if direction != Vec3::ZERO {
        let forward = *camera.forward();
        let forward = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
        let right = *camera.right();
        let right = Vec3::new(right.x, 0.0, right.z).normalize_or_zero();
        direction = (forward * direction.z + right * direction.x).normalize_or_zero();
    }

    let jump = keyboard.pressed(KeyCode::Space);
    if jump {
        controller.action(ControlScheme::Jump(TnuaBuiltinJump {
            allow_in_air: false,
            ..default()
        }));
    }

    let dash = keyboard.pressed(KeyCode::ShiftLeft);
    if dash {
        let up_direction = controller.up_direction().unwrap_or(Dir3::Y);
        controller.action(ControlScheme::Dash(TnuaBuiltinDash {
            displacement: direction.normalize() + up_direction.adjust_precision(),
            desired_forward: Dir3::new(direction.f32()).ok(),
            allow_in_air: true,
        }));
    }

    let turn_in_place = keyboard.pressed(KeyCode::AltLeft);

    controller.basis = TnuaBuiltinWalk {
        desired_motion: if turn_in_place { Vec3::ZERO } else { direction },
        desired_forward: Dir3::new(direction).ok(),
    };
}

fn handle_animating(
    mut player_query: Query<(
        &TnuaController<ControlScheme>,
        &mut TnuaAnimatingState<AnimationState>,
    )>,
    mut animation_player_query: Query<&mut AnimationPlayer>,
    animation_nodes: Option<Res<Animations>>,
) {
    let Ok((controller, mut animating_state)) = player_query.single_mut() else {
        return;
    };
    let Ok(mut animation_player) = animation_player_query.single_mut() else {
        return;
    };
    let Some(animation_nodes) = animation_nodes else {
        return;
    };

    let current_status_for_animating = match controller.current_action.as_ref() {
        Some(ControlSchemeActionState::Jump(state)) => match state.memory {
            TnuaBuiltinJumpMemory::NoJump => return,
            TnuaBuiltinJumpMemory::StartingJump { .. } => AnimationState::Jumping,
            TnuaBuiltinJumpMemory::SlowDownTooFastSlopeJump { .. } => AnimationState::Jumping,
            TnuaBuiltinJumpMemory::MaintainingJump { .. } => AnimationState::Jumping,
            TnuaBuiltinJumpMemory::StoppedMaintainingJump => AnimationState::Jumping,
            TnuaBuiltinJumpMemory::FallSection => AnimationState::Falling,
        },
        Some(ControlSchemeActionState::Dash(_)) => AnimationState::Dashing,
        None => {
            if controller.basis_memory.standing_on_entity().is_none() {
                AnimationState::Falling
            } else {
                let speed = controller.basis_memory.running_velocity.length();
                if 0.01 < speed {
                    AnimationState::Running(0.1 * speed)
                } else {
                    AnimationState::Standing
                }
            }
        }
    };

    let animating_directive = animating_state.update_by_discriminant(current_status_for_animating);

    match animating_directive {
        TnuaAnimatingStateDirective::Maintain { state } => {
            if let AnimationState::Running(speed) = state
                && let Some(animation) =
                    animation_player.animation_mut(animation_nodes.animations["Running_A"])
            {
                animation.set_speed(*speed);
            }
            if let AnimationState::Standing = state {
                let landing_finished = if let Some(landing_anim) =
                    animation_player.animation(animation_nodes.animations["Jump_Land"])
                {
                    landing_anim.is_finished()
                } else {
                    false
                };

                if landing_finished {
                    animation_player
                        .start(animation_nodes.animations["Idle"])
                        .set_speed(1.0)
                        .repeat();
                }
            }
        }
        TnuaAnimatingStateDirective::Alter { old_state, state } => {
            animation_player.stop_all();

            match state {
                AnimationState::Standing => {
                    if let Some(AnimationState::Falling) = old_state {
                        animation_player
                            .start(animation_nodes.animations["Jump_Land"])
                            .set_speed(1.0);
                    } else {
                        animation_player
                            .start(animation_nodes.animations["Idle"])
                            .set_speed(1.0)
                            .repeat();
                    }
                }
                AnimationState::Running(speed) => {
                    animation_player
                        .start(animation_nodes.animations["Running_A"])
                        .set_speed(*speed)
                        .repeat();
                }
                AnimationState::Jumping => {
                    animation_player
                        .start(animation_nodes.animations["Jump_Start"])
                        .set_speed(2.0);
                }
                AnimationState::Falling => {
                    animation_player
                        .start(animation_nodes.animations["Jump_Idle"])
                        .set_speed(1.0);
                }
                AnimationState::Dashing => {
                    animation_player
                        .start(animation_nodes.animations["Dodge_Forward"])
                        .set_speed(1.0);
                }
                _ => {}
            }
        }
    }
}

#[derive(Component)]
struct ThirdPersonCamera {
    distance: f32,
    pitch: f32,
    yaw: f32,
    sensitivity: f32,
}

impl Default for ThirdPersonCamera {
    fn default() -> Self {
        Self {
            distance: 15.0,
            pitch: -0.5,
            yaw: 0.0,
            sensitivity: 0.003,
        }
    }
}

fn orbit_camera(
    mut camera_query: Query<(&mut ThirdPersonCamera, &mut Transform)>,
    player_query: Query<&Transform, (With<Player>, Without<ThirdPersonCamera>)>,
    mut mouse_motion: MessageReader<MouseMotion>,
    mut mouse_wheel: MessageReader<MouseWheel>,
) {
    let Ok((mut camera, mut camera_transform)) = camera_query.single_mut() else {
        return;
    };
    let Ok(player_transform) = player_query.single() else {
        return;
    };

    let mut rotation_delta = Vec2::ZERO;
    for event in mouse_motion.read() {
        rotation_delta += event.delta;
    }

    let mut zoom_delta = 0.0;
    for event in mouse_wheel.read() {
        zoom_delta -= event.y;
    }

    camera.yaw -= rotation_delta.x * camera.sensitivity;
    camera.pitch -= rotation_delta.y * camera.sensitivity;
    camera.distance += zoom_delta * 0.5;
    camera.distance = camera.distance.clamp(2.0, 20.0);

    camera.pitch = camera.pitch.clamp(0.01 - FRAC_PI_2, 0.0);

    let rot = Quat::from_euler(EulerRot::YXZ, camera.yaw, camera.pitch, 0.0);
    let offset = rot * Vec3::new(0.0, 0.0, camera.distance);

    let target = player_transform.translation + Vec3::new(0.0, 1.5, 0.0);

    camera_transform.translation = target + offset;
    camera_transform.look_at(target, Vec3::Y);
}
