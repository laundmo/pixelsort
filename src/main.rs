#![feature(array_chunks)]

use bevy::{prelude::*, render::texture::ImageSampler};
use bevy_egui::EguiPlugin;
use bevy_pancam::{PanCam, PanCamPlugin};
use rayon::prelude::*;

mod sorting;
mod ui;
use sorting::{PixelOrdering, RowOp, Threshold};

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::rgb(0., 0., 0.)))
        .init_resource::<Settings>()
        .add_plugins(DefaultPlugins)
        .add_plugin(PanCamPlugin::default())
        .add_plugin(EguiPlugin)
        .init_resource::<PixelsortImage>()
        .add_startup_system(setup)
        .add_system(ensure_nn)
        .add_system(update_img)
        .add_system(ui::ui)
        .run();
}
struct PixelsortImage {
    source: Handle<Image>,
    image: Handle<Image>,
}

#[derive(Default, PartialEq, Clone)]
struct Settings {
    threshold: Threshold,
    threshold_reverse: bool,
    ordering: PixelOrdering,
    ordering_reverse: bool,
}

impl FromWorld for PixelsortImage {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.get_resource_mut::<AssetServer>().unwrap();
        let source = asset_server.load("input1.jpg");
        Self {
            source: source.clone(),
            image: source,
        }
    }
}

fn ensure_nn(
    mut images: ResMut<Assets<Image>>,
    mut pixelsimg: ResMut<PixelsortImage>,
    canvas_texture: Query<Entity, With<Canvas>>,
    mut commands: Commands,
    mut ran: Local<bool>,
) {
    // early return if already run
    if *ran {
        return;
    }

    // get source image
    let image_src = images.get(&pixelsimg.source);
    if let Some(img) = image_src {
        // clone source image
        let image = img.clone();
        // add handle to resource
        pixelsimg.image = images.add(image);
        // add handle to canvas
        let canvas_entity = canvas_texture.single();
        commands
            .entity(canvas_entity)
            .insert(pixelsimg.image.as_weak::<Image>());
        *ran = true;
    }
}

#[derive(Component)]
struct Canvas;

fn setup(mut commands: Commands, pixelsimg: Res<PixelsortImage>) {
    // camera
    commands
        .spawn_bundle(SpriteBundle {
            texture: pixelsimg.image.clone(),
            transform: Transform::from_scale(Vec2::splat(3.).extend(0.)),
            ..default()
        })
        .insert(Canvas);
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

fn update_img(
    pixelsimg: ResMut<PixelsortImage>,
    mut images: ResMut<Assets<Image>>,
    settings: Res<Settings>,
    mut last_settings: Local<Settings>,
) {
    if *settings == *last_settings {
        return;
    }
    *last_settings = settings.clone();

    if let Some(source) = images.get(&pixelsimg.source) {
        let (w, h) = source.size().into();
        let src_data = source.data.clone(); // TODO: this should not be neccessary
        if let Some(dest) = images.get_mut(&pixelsimg.image) {
            let width = w.round() as usize * 4;

            dest.data = src_data;

            dest.data.par_chunks_exact_mut(width).for_each(|row| {
                let mut row_op = RowOp::default();
                row_op.apply_threshold(row, &settings.threshold, settings.threshold_reverse);

                for range in row_op.slices.iter() {
                    let sorted = &settings.ordering.order(
                        row[range.0 * 4..range.1 * 4].array_chunks::<4>().copied(),
                        settings.ordering_reverse,
                    );
                    row[range.0 * 4..range.1 * 4].copy_from_slice(&sorted[..]);
                }
            });
        }
    }
}
