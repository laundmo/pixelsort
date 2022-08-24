use nannou::{
    color::IntoLinSrgba,
    image::{self, RgbImage},
    prelude::*,
};
use nannou_egui::{self, egui, Egui};
use num_traits::PrimInt;
use rayon::prelude::*;
use std::{
    fmt,
    sync::{Arc, Mutex},
};

// TODO: allow storing functions to be selected from the UI.
struct Settings {
    thresh: usize,
}

// TODO: Pre-cache luminance calculations once per image
struct Model {
    buffer: image::RgbImage,
    settings: Settings,
    egui: Egui,
}

fn main() {
    nannou::app(model).update(update).run();
}

fn model(app: &App) -> Model {
    // Create a new window!
    let winid = app
        .new_window()
        .size(512, 512)
        .raw_event(raw_window_event)
        .view(view)
        .build()
        .unwrap();
    // Load the image from disk and upload it to a GPU texture.
    let assets = app.assets_path().unwrap();
    let img_path = assets.join("input1.jpg");

    let image = image::open(img_path).unwrap();

    let window = app.window(winid).unwrap();
    let egui = Egui::from_window(&window);

    Model {
        buffer: image.into_rgb8(),
        egui,
        settings: Settings { thresh: 60 },
    }
}

// TODO: Does not actually need to store function? rather store enum pointing to predefined funcs.
struct RowOp<T> {
    key: Option<fn(&[T; 3], &[T; 3]) -> std::cmp::Ordering>,
    threshold: Option<fn(&[T; 3], &Settings) -> bool>,
    slices: Vec<[usize; 2]>,
}

impl<T> RowOp<T> {
    fn add_slice(&mut self, start: usize, end: usize) -> &mut Self {
        self.slices.push([start, end]);
        self
    }

    fn set_key(&mut self, key: fn(&[T; 3], &[T; 3]) -> std::cmp::Ordering) -> &mut Self {
        self.key = Some(key);
        self
    }

    fn add_threshold(&mut self, threshold: fn(&[T; 3], &Settings) -> bool) -> &mut Self {
        self.threshold = Some(threshold);
        self
    }

    fn apply_threshold(&mut self, row: &[[T; 3]], settings: &Settings) -> &mut Self {
        let thresh = self
            .threshold
            .expect("threshold needs to be set before applying");
        // f, f, f, t, t, t, t, f, f , t, t
        let mut start = (0, false);

        for (i, val) in row.iter().map(|x| thresh(x, settings)).enumerate() {
            if val {
                // start of a run
                if !start.1 {
                    start = (i, true);
                }
            } else if start.1 {
                // end of a run
                self.add_slice(start.0, i - 1);
                start = (0, false);
            }
        }
        if start.1 {
            self.add_slice(start.0, row.len() - 1);
        }

        self
    }
}

impl<T> fmt::Debug for RowOp<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("RowOp")
            .field("slices", &self.slices)
            .finish()
    }
}

// TODO: do i actually need to store the entire image, maybe could use the buffer directly
struct PixelsortImage<T> {
    rows: Vec<Vec<[T; 3]>>,
    row_ops: Vec<RowOp<T>>,
    width: u32,
    height: u32,
}

impl PixelsortImage<u8> {
    fn from_buffer(buf: &RgbImage) -> Self {
        let (width, height) = buf.dimensions();

        PixelsortImage {
            rows: buf
                .rows()
                .map(|row| row.map(|pixel| pixel.0).collect())
                .collect(),
            row_ops: (0..height)
                .into_iter()
                .map(|_| RowOp {
                    key: None,
                    threshold: None,
                    slices: Vec::new(),
                })
                .collect(),
            width,
            height,
        }
    }

    fn sort_unstable(&mut self, settings: &Settings) {
        self.row_ops
            .par_iter_mut()
            .zip(&self.rows)
            .for_each(|(row_op, row)| {
                row_op.apply_threshold(&row[..], settings);
            });

        self.rows
            .par_iter_mut()
            .zip(&self.row_ops)
            .for_each(|(row, row_op)| {
                for range in row_op.slices.iter() {
                    row[range[0]..range[1]]
                        .sort_unstable_by(row_op.key.expect("Key was not set prior to sorting."));
                }
            });
    }

    fn consume_to_buffer(&mut self, settings: &Settings) -> image::RgbImage {
        self.sort_unstable(settings);
        let data: Vec<u8> = self.rows.iter().flatten().flat_map(|a| *a).collect();
        image::ImageBuffer::from_raw(self.width, self.height, data).unwrap()
    }
}

struct PixelUtil<'a, T: PrimInt>(&'a [T; 3]);

impl<'a> PixelUtil<'a, u8> {
    #[inline]
    fn rgb(&self) -> [u8; 3] {
        [self.r(), self.g(), self.b()]
    }

    #[inline]
    fn r(&self) -> u8 {
        self.0[0]
    }

    #[inline]
    fn rf(&self) -> f32 {
        self.r() as f32 / 255.
    }

    #[inline]
    fn g(&self) -> u8 {
        self.0[1]
    }

    #[inline]
    fn gf(&self) -> f32 {
        self.g() as f32 / 255.
    }

    #[inline]
    fn b(&self) -> u8 {
        self.0[2]
    }

    #[inline]
    fn bf(&self) -> f32 {
        self.b() as f32 / 255.
    }

    #[inline]
    fn sum(&self) -> usize {
        self.rgb().iter().map(|x| usize::from_u8(*x).unwrap()).sum()
    }

    #[inline]
    fn avg(&self) -> usize {
        self.sum() / 3
    }

    #[inline]
    fn luminance(&self) -> usize {
        let redf = 0.299 * self.rf();
        let greenf = 0.587 * self.gf();
        let bluef = 0.114 * self.bf();
        ((redf + greenf + bluef) * 255.).round() as usize
    }

    fn hsv(&self) -> [usize; 3] {
        let hsv: Hsv = rgb8(self.0[0], self.0[1], self.0[2])
            .into_lin_srgba()
            .into();
        [
            hsv.hue.to_positive_degrees().round() as usize,
            (hsv.saturation * 255.).round() as usize,
            (hsv.value * 255.).round() as usize,
        ]
    }
}

fn update(_app: &App, model: &mut Model, update: Update) {
    println!("{} ms", update.since_last.as_millis());

    let egui = &mut model.egui;
    let settings = &mut model.settings;

    egui.set_elapsed_time(update.since_start);
    let ctx = egui.begin_frame();
    egui::Window::new("Settings").show(&ctx, |ui| {
        ui.label("Threshold:");
        ui.add(egui::Slider::new(&mut settings.thresh, 1..=255));
    });
}

fn raw_window_event(_app: &App, model: &mut Model, event: &nannou::winit::event::WindowEvent) {
    // Let egui handle things like keyboard and mouse input.
    model.egui.handle_raw_event(event);
}

fn view(app: &App, model: &Model, frame: Frame) {
    frame.clear(WHITE);
    let win = app.window_rect();
    let draw = app.draw();
    let settings = &model.settings;

    let mut img = PixelsortImage::from_buffer(&model.buffer);
    for op in &mut img.row_ops {
        op.set_key(|pixela, pixelb| pixelb[0].cmp(&pixela[0]))
            .add_threshold(|pixel, settings| PixelUtil(pixel).luminance() < settings.thresh);
    }

    let buf = img.consume_to_buffer(settings);
    let texture = wgpu::Texture::from_image(app, &image::DynamicImage::ImageRgb8(buf));

    draw.texture(&texture).xy(win.xy()).wh(win.wh());

    draw.to_frame(app, &frame).unwrap();
    model.egui.draw_to_frame(&frame).unwrap();
}
