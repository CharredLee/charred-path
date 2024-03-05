use bevy::prelude::*;
use bevy::sprite::MaterialMesh2dBundle;
use charred_path::{piecewise_linear::PathPlugin, prelude::*};

const PLAYER_COLOR: Color = Color::rgb(0.15, 0.6, 0.5);
const PLAYER_START: Vec3 = Vec3::new(0.0, 0.0, 0.0);

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins,
        PathPlugin,
        PathDebugPlugin,
    ));
    app.add_systems(Startup, init);
    app.add_systems(FixedUpdate, player_movement);
    app.add_systems(Update, homotopy_text_update);
    app.run();
}

#[derive(Component)]
pub struct Player;

#[derive(Component)]
struct HomotopyWordText;

fn init(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // spawn the camera
    commands.spawn(Camera2dBundle::default());

    // Define some puncture points
    let puncture_points = vec![
        PuncturePoint::new(Vec2::new(-225.0, 100.0), 'A'),
        PuncturePoint::new(Vec2::new(-75.0, 150.0), 'B'),
        PuncturePoint::new(Vec2::new(75.0, 150.0), 'C'),
        PuncturePoint::new(Vec2::new(225.0, 100.0), 'D'),
    ];

    // Render puncture points as red circles
    let radius = 5.0;
    let material = materials.add(ColorMaterial::from(Color::RED));
    for point in puncture_points.iter() {
        commands.spawn(MaterialMesh2dBundle {
            mesh: meshes.add(Circle::new(radius)).into(),
            material: material.clone(),
            transform: Transform::from_translation(point.position().extend(0.0)),
            ..Default::default()
        });
    }

    // spawn the player
    commands.spawn((
        Player,
        MaterialMesh2dBundle {
            mesh: meshes.add(Circle::new(10.0)).into(),
            material: materials.add(PLAYER_COLOR),
            transform: Transform::from_translation(PLAYER_START),
            ..Default::default()
        },
        PathType::new(PLAYER_START.truncate(), puncture_points),
    ));

    // spawn the text
    commands.spawn((
        TextBundle::from_section(
            "default",
            TextStyle {
                font_size: 60.0,
                ..Default::default()
            }
        )
        .with_text_justify(JustifyText::Center)
        .with_style(Style {
            position_type: PositionType::Absolute,
            bottom: Val::Px(5.0),
            right: Val::Px(5.0), 
            ..Default::default()
        }),
        HomotopyWordText,
    ));
}


fn player_movement(
    mut player_query: Query<&mut Transform, With<Player>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    if let Ok(mut transform) = player_query.get_single_mut() {
        let mut dir = Vec3::ZERO;
        if keyboard_input.pressed(KeyCode::ArrowUp) {
            dir.y += 1.0;
        }
        if keyboard_input.pressed(KeyCode::ArrowDown) {
            dir.y -= 1.0;
        }
        if keyboard_input.pressed(KeyCode::ArrowRight) {
            dir.x += 1.0;
        }
        if keyboard_input.pressed(KeyCode::ArrowLeft) {
            dir.x -= 1.0;
        }
        transform.translation += 200.0 * dir * time.delta_seconds();
    }
}



fn homotopy_text_update(
    mut text_query: Query<&mut Text, With<HomotopyWordText>>,
    path_query: Query<&PathType>,
) {
    if let Ok(path_type) = path_query.get_single() {
        if let Ok(mut text) = text_query.get_single_mut() {
            text.sections[0].value = path_type.word();
        }
    }
}