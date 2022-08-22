use nannou::{
    image::{self, RgbImage},
    prelude::*,
};

struct Model {
    buffer: image::RgbImage,
}

fn main() {
    nannou::app(model).update(update).run();
}

fn model(app: &App) -> Model {
    // Create a new window!
    app.new_window().size(512, 512).view(view).build().unwrap();
    // Load the image from disk and upload it to a GPU texture.
    let assets = app.assets_path().unwrap();
    let img_path = assets.join("input.png");

    let image = image::open(img_path).unwrap();

    Model {
        buffer: image.into_rgb8(),
    }
}

struct RowOp<T> {
    key: Option<fn(&[T; 3]) -> usize>,
    slices: Vec<[usize; 2]>,
}

impl<T> RowOp<T> {
    fn add_slice(&mut self, start: usize, end: usize) -> &mut Self {
        self.slices.push([start, end]);
        self
    }

    fn set_key(&mut self, key: fn(&[T; 3]) -> usize) -> &mut Self {
        self.key = Some(key);
        self
    }
}

struct SortableImage<T> {
    rows: Vec<Vec<[T; 3]>>,
    row_ops: Vec<RowOp<T>>,
    width: u32,
    height: u32,
}

impl SortableImage<u8> {
    fn from_buffer(buf: &RgbImage) -> Self {
        let (width, height) = buf.dimensions();

        SortableImage {
            rows: buf
                .rows()
                .map(|row| row.map(|pixel| pixel.0).collect())
                .collect(),
            row_ops: (0..height)
                .into_iter()
                .map(|_| RowOp {
                    key: None,
                    slices: Vec::new(),
                })
                .collect(),
            width,
            height,
        }
    }

    fn sort_unstable(&mut self) {
        for (i, row_op) in self.row_ops.iter().enumerate() {
            for range in &row_op.slices {
                self.rows[i][range[0]..range[1]]
                    .sort_unstable_by_key(row_op.key.expect("Key was not set prior to sorting."));
            }
        }
    }
    fn consume_to_buffer(&mut self) -> image::RgbImage {
        self.sort_unstable();
        let data: Vec<u8> = self.rows.iter().flatten().flat_map(|a| *a).collect();
        image::ImageBuffer::from_raw(self.width, self.height, data).unwrap()
    }
}

fn update(_app: &App, model: &mut Model, _update: Update) {
    let mut img = SortableImage::from_buffer(&model.buffer);

    for row in &mut img.row_ops {
        row.set_key(|pixel| pixel[0] as usize).add_slice(100, 160);
    }

    model.buffer = img.consume_to_buffer();

    // for y in 0..h {
    //     for x in 0..w {
    //         if x + 1 < w {
    //             let pixel = model.buffer[(x, y)];
    //             let forward = model.buffer[(x + 1, y)];
    //             if forward.0[0] > pixel.0[0] {
    //                 model.buffer[(x, y)] = forward;
    //                 model.buffer[(x + 1, y)] = pixel;
    //             }
    //         }
    //     }
    // }
}

fn view(app: &App, model: &Model, frame: Frame) {
    frame.clear(WHITE);
    let win = app.window_rect();
    let draw = app.draw();

    let texture =
        wgpu::Texture::from_image(app, &image::DynamicImage::ImageRgb8(model.buffer.clone()));

    draw.texture(&texture).xy(win.xy()).wh(win.wh());

    draw.to_frame(app, &frame).unwrap();
}
