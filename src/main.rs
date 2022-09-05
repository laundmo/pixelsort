#![feature(array_chunks)]

use bevy::{
    ecs::system::{Command, Insert},
    prelude::*,
    render::{render_resource::Extent3d, texture::ImageSampler},
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
    // Enable ansi on windows, if possible
    let _ = enable_ansi_support::enable_ansi_support();

    App::new()
        // Setup resources (global state) for this app.
        .insert_resource(ClearColor(Color::rgb(0., 0., 0.)))
        .insert_resource(Canvas(None))
        .init_resource::<Settings>()
        .add_event::<RotateEvent>()
        // Setup states
        .add_loopless_state(ImageStates::Before)
        // Ass all plugins - WebAssetPlugin currently unused
        .add_plugin(WebAssetPlugin)
        .add_plugins(DefaultPlugins)
        // Never seen the loading take long, this is likely only useful once WebAssetPlugin is used.
        .add_plugin(ProgressPlugin::new(ImageStates::Loading))
        .add_plugin(PanCamPlugin::default())
        .add_plugin(EguiPlugin)
        // State in which asset loading happenes.
        .add_loading_state(
            LoadingState::new(ImageStates::Loading)
                .continue_to_state(ImageStates::Loaded)
                .with_collection::<ImageAssets>()
                .init_resource::<PixelsortImage>(),
        )
        // Systems which run in the main loop
        .add_startup_system(setup)
        .add_system(ui::ui)
        .add_system(file_drop)
        // These only run once a image was loaded.
        .add_system_set(
            ConditionSet::new()
                .run_in_state(ImageStates::Loaded)
                .with_system(update_img)
                .with_system(rotate_img_90)
                .into(),
        )
        .run();
}

// Only needed for asset_loader, wont be used outside of it.
#[derive(AssetCollection)]
struct ImageAssets {
    #[asset(key = "image")]
    image: Handle<Image>,
}

// Store the actual images used: source being the unedited one and dest the sorted one.
struct PixelsortImage {
    source: Handle<Image>,
    dest: Handle<Image>,
}

// All of the settings which can be set in the UI
#[derive(Default, PartialEq, Clone)]
struct Settings {
    threshold: Threshold,
    threshold_reverse: bool,
    ordering: PixelOrdering,
    ordering_reverse: bool,
    extend_threshold_left: usize,
    extend_threshold_right: usize,
    merge_limit: usize,
}

impl FromWorld for PixelsortImage {
    fn from_world(world: &mut World) -> Self {
        let (source_clone, source_handle_clone, canvas_entity) = {
            let cell = world.cell();

            // Load resources we want to use/edit from the world.
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
        // Overwrite the sprite to force it into recalculating the size.
        let i = Insert {
            entity: canvas_entity,
            component: Sprite::default(),
        };
        i.write(world);

        // Return self with the new handles.
        Self {
            source: source_handle_clone,
            dest: clone_handle,
        }
    }
}

// Resource used to track the Canvas entity
#[derive(Deref, DerefMut)]
struct Canvas(Option<Entity>);

fn setup(mut commands: Commands, mut canvas: ResMut<Canvas>) {
    // Setup empty sprite
    canvas.0 = Some(
        commands
            .spawn_bundle(SpriteBundle {
                sprite: Sprite {
                    color: Color::ANTIQUE_WHITE,
                    custom_size: Some(Vec2::new(1000., 500.)),
                    ..default()
                },
                ..default()
            })
            .id(),
    );

    // Add a PanCam, allows for pan and zoom
    commands
        .spawn_bundle(Camera2dBundle::default())
        .insert(PanCam {
            grab_buttons: vec![MouseButton::Left, MouseButton::Middle],
            enabled: true,
            zoom_to_cursor: true,
            min_scale: 0.,
            max_scale: Some(40.),
        });
}

// Event dispatched when the image is rotated.
#[derive(Default)]
struct RotateEvent;

// System which rotates the image
fn rotate_img_90(
    mut evt: EventReader<RotateEvent>,
    pixelsimage: Option<Res<PixelsortImage>>,
    mut images: ResMut<Assets<Image>>,
    // needed to recreate the Sprite, forces it to re-size itself to the rotated size.
    canvas: Res<Canvas>,
    mut commands: Commands,
) {
    if let Some(pixelsimg) = pixelsimage {
        for _ in evt.iter() {
            let new_extent = {
                let source = images.get_mut(&pixelsimg.source).expect("unreachable");
                let (w, h) = source.size().into();
                let w = w.round() as usize;
                let h = h.round() as usize;
                let src_data = source.data.clone();
                let mut i_dest = 0;
                for x in 0..w {
                    for y in (0..h).rev() {
                        // Rotate the image, assuming 4 bytes are one pixel (rgba)
                        let index = (x + y * w) * 4;
                        source.data[i_dest] = src_data[index];
                        source.data[i_dest + 1] = src_data[index + 1];
                        source.data[i_dest + 2] = src_data[index + 2];
                        source.data[i_dest + 3] = src_data[index + 3];
                        i_dest += 4;
                    }
                }
                let extent = Extent3d {
                    width: h as u32,
                    height: w as u32,
                    depth_or_array_layers: 1,
                };
                // Resize the image to the dimensions fitting its new rotated data
                source.reinterpret_size(extent);
                extent
            };
            let src_clone = images.get(&pixelsimg.source).unwrap().data.clone();
            let dest = images.get_mut(&pixelsimg.dest).unwrap();
            // Resize the other, destination, image the same way.
            dest.reinterpret_size(new_extent);
            dest.data = src_clone;
            // Force sprite reset
            commands
                .entity(canvas.0.expect("unreachable"))
                .insert(Sprite::default());
        }
    }
}

fn update_img(
    pixelsimage: Option<ResMut<PixelsortImage>>,
    mut images: ResMut<Assets<Image>>,
    settings: Res<Settings>,
    mut last_settings: Local<Settings>,
) {
    // Check if settings have changed
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

                // Overwrite the source completely, otherwise there will be artifacts from previous sorts.
                dest.data = src_data;

                // Paralell loop over the rows of pixels
                dest.data.par_chunks_exact_mut(width).for_each(|row| {
                    let mut row_op = RowOp::default();
                    // Apply the threshold settings to this row
                    row_op.apply_threshold(row, w, &settings);

                    // loop over all parts of the row matched by the threshold
                    for range in row_op.slices.iter() {
                        // and sort them
                        let sorted = &settings.ordering.order(
                            row[range.0 * 4..range.1 * 4].array_chunks::<4>().copied(),
                            settings.ordering_reverse,
                        );
                        // and copy them back into the row
                        row[range.0 * 4..range.1 * 4].copy_from_slice(&sorted[..]);
                    }
                });
            }
        }
    }
}

fn file_drop(mut dnd_evr: EventReader<FileDragAndDrop>, mut commands: Commands) {
    // Loop over all drop events
    for ev in dnd_evr.iter() {
        if let FileDragAndDrop::DroppedFile { id: _, path_buf } = ev {
            if let Some(extension) = path_buf.extension() {
                // Find the correct file extensions
                match extension.to_str() {
                    Some("png") | Some("jpg") | Some("jpeg") => {
                        // And dynamically load the image at runtime
                        commands.add(RegisterStandardDynamicAsset {
                            key: "image",
                            asset: StandardDynamicAsset::File {
                                path: path_buf.as_os_str().to_str().expect("").to_owned(),
                            },
                        });
                        // Transition the state to Loading, to trigger asset loading.
                        commands.insert_resource(NextState(ImageStates::Loading));
                    }
                    // Any other file type is unsupported
                    Some(_) => println!("Unsupported file type dropped."),
                    None => println!("Cant deal with non utf-8 paths."),
                }
            }
        }
    }
}
