#![feature(array_chunks)]

use bevy::{
    ecs::system::{Command, Insert},
    prelude::*,
    render::texture::ImageSampler,
};
use bevy_asset_loader::prelude::*;
use bevy_egui::EguiPlugin;
use bevy_pancam::{PanCam, PanCamPlugin};
use bevy_web_asset::WebAssetPlugin;
use iyes_loopless::prelude::*;
use iyes_progress::prelude::*;
use rayon::prelude::*;

mod sorting;
mod ui;
use sorting::{PixelOrdering, RowOp, Threshold};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ImageStates {
    Before,
    Loading,
    Loaded,
}

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::rgb(0., 0., 0.)))
        .insert_resource(Canvas(None))
        .init_resource::<Settings>()
        .add_loopless_state(ImageStates::Before)
        .add_plugin(WebAssetPlugin)
        .add_plugins(DefaultPlugins)
        .add_loading_state(
            LoadingState::new(ImageStates::Loading)
                .continue_to_state(ImageStates::Loaded)
                .with_collection::<ImageAssets>()
                .init_resource::<PixelsortImage>(),
        )
        .add_plugin(ProgressPlugin::new(ImageStates::Loading))
        .add_plugin(PanCamPlugin::default())
        .add_plugin(EguiPlugin)
        .add_startup_system(setup)
        .add_system(ui::ui)
        .add_system(file_drop)
        .add_system_set(
            ConditionSet::new()
                .run_in_state(ImageStates::Loaded)
                .with_system(update_img)
                .into(),
        )
        .run();
}

#[derive(AssetCollection)]
struct ImageAssets {
    #[asset(key = "image")]
    image: Handle<Image>,
}

struct PixelsortImage {
    source: Handle<Image>,
    dest: Handle<Image>,
}

#[derive(Default, PartialEq, Clone)]
struct Settings {
    threshold: Threshold,
    threshold_reverse: bool,
    ordering: PixelOrdering,
    ordering_reverse: bool,
    extend_threshold_left: usize,
    extend_threshold_right: usize,
}

impl FromWorld for PixelsortImage {
    fn from_world(world: &mut World) -> Self {
        let (source_clone, source_handle_clone, canvas_entity) = {
            let cell = world.cell();
            //
            let image_assets = cell
                .get_resource::<ImageAssets>()
                .expect("Failed to get loaded image Asset");

            let mut images = cell
                .get_resource_mut::<Assets<Image>>()
                .expect("Faild to get image asset handler.");

            // Get and clone source image (and set nearest sampler)
            let mut source_image = images.get_mut(&image_assets.image).unwrap();
            source_image.sampler_descriptor = ImageSampler::nearest();

            // get canvas entity
            let canvas = cell
                .get_resource_mut::<Canvas>()
                .expect("Faild to get image asset handler.");

            let canvas_entity = canvas.0.expect("Canvas not yet initialised.");

            (
                source_image.clone(),
                image_assets.image.clone(),
                canvas_entity,
            )
        };

        // add image to Asset<Image> and get handle
        let clone_handle = {
            world
                .get_resource_mut::<Assets<Image>>()
                .unwrap()
                .add(source_clone)
        };

        // Overwrite canvas entity handle.
        let i = Insert {
            entity: canvas_entity,
            component: clone_handle.as_weak::<Image>(),
        };
        i.write(world);

        Self {
            source: source_handle_clone,
            dest: clone_handle,
        }
    }
}

#[derive(Deref, DerefMut)]
struct Canvas(Option<Entity>);

fn setup(mut commands: Commands, mut canvas: ResMut<Canvas>) {
    // Store sprite in canvas
    canvas.0 = Some(
        commands
            .spawn_bundle(SpriteBundle {
                sprite: Sprite {
                    color: Color::WHITE,
                    custom_size: Some(Vec2::new(1000., 500.)),
                    ..Default::default()
                },
                ..default()
            })
            .id(),
    );

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
    pixelsimage: Option<ResMut<PixelsortImage>>,
    mut images: ResMut<Assets<Image>>,
    settings: Res<Settings>,
    mut last_settings: Local<Settings>,
) {
    if *settings == *last_settings {
        return;
    }
    *last_settings = settings.clone();

    if let Some(pixelsimg) = pixelsimage {
        if let Some(source) = images.get(&pixelsimg.source) {
            let (w, _) = source.size().into();
            let w = w.round() as usize;
            let src_data = source.data.clone();
            if let Some(dest) = images.get_mut(&pixelsimg.dest) {
                let width = w * 4;

                dest.data = src_data;

                dest.data.par_chunks_exact_mut(width).for_each(|row| {
                    let mut row_op = RowOp::default();
                    row_op.apply_threshold(row, w, &settings);

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
}

fn file_drop(mut dnd_evr: EventReader<FileDragAndDrop>, mut commands: Commands) {
    for ev in dnd_evr.iter() {
        if let FileDragAndDrop::DroppedFile { id: _, path_buf } = ev {
            if let Some(extension) = path_buf.extension() {
                match extension.to_str() {
                    Some("png") | Some("jpg") => {
                        commands.add(RegisterStandardDynamicAsset {
                            key: "image",
                            asset: StandardDynamicAsset::File {
                                path: path_buf.as_os_str().to_str().expect("").to_owned(),
                            },
                        });
                        commands.insert_resource(NextState(ImageStates::Loading));
                    }
                    Some(_) => println!("Unsupported file type dropped."),
                    None => println!("Cant deal with non utf-8 paths."),
                }
            }
        }
    }
}
