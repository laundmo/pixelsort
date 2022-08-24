use bevy::prelude::*;
use bevy_pancam::{PanCam, PanCamPlugin};

mod sorting;
use sorting::{RowOp, Threshold};

#[derive(Component)]
struct PanCamera;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(PanCamPlugin::default())
        .init_resource::<PixelsortImage>()
        .add_startup_system(setup)
        .add_system(update_img)
        .run();
}

struct PixelsortImage {
    image: Handle<Image>,
}

impl FromWorld for PixelsortImage {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.get_resource_mut::<AssetServer>().unwrap();
        Self {
            image: asset_server.load("input1.jpg"),
        }
    }
}

fn setup(mut commands: Commands, pixelsimg: Res<PixelsortImage>) {
    // camera
    commands.spawn_bundle(SpriteBundle {
        texture: pixelsimg.image.clone(),
        transform: Transform::from_scale(Vec2::splat(3.).extend(0.)),
        ..default()
    });
    commands
        .spawn_bundle(Camera2dBundle { ..default() })
        .insert(PanCam {
            grab_buttons: vec![MouseButton::Left, MouseButton::Middle],
            enabled: true,
            zoom_to_cursor: true,
            min_scale: 0.,
            max_scale: Some(40.),
        });
}

fn update_img(pixelsimg: ResMut<PixelsortImage>, mut images: ResMut<Assets<Image>>) {
    if let Some(image) = images.get_mut(&pixelsimg.image) {
        let (w, h) = image.size().into();
        for row in image.data.chunks_exact(w.round() as usize) {
            let row_op = RowOp::default();
            row_op.apply_threshold(row, Threshold::Red(150), false);

            for range in row_op.slices.iter() {
                row[range].sort_unstable_by();
            }
        }
    }
}
