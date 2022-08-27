use crate::Settings;
use itertools::Itertools;
use std::{iter::Copied, slice::ArrayChunks};

#[derive(strum_macros::Display, PartialEq, Clone, Debug)]
pub(crate) enum Threshold {
    Luminance(f32),
    ColorSimilarity(i16, [u8; 3]),
}

impl Default for Threshold {
    fn default() -> Self {
        Threshold::Luminance(150.)
    }
}

#[derive(Default, strum_macros::Display, PartialEq, Clone)]
pub(crate) enum PixelOrdering {
    #[default]
    Luminance,
    ColorSimilarity([u8; 3]),
}

fn pixel_to_luminance(pixel: &[u8; 4]) -> f32 {
    (pixel[0] as usize * 2 + pixel[1] as usize * 3 + pixel[2] as usize) as f32 / 6.
}

// source: https://www.compuphase.com/cmetric.htm
// double ColourDistance(RGB e1, RGB e2)
// {
//     long rmean = ( (long)e1.r + (long)e2.r ) / 2;
//     long r = (long)e1.r - (long)e2.r;
//     long g = (long)e1.g - (long)e2.g;
//     long b = (long)e1.b - (long)e2.b;
//     return sqrt((((512+rmean)*r*r)>>8) + 4*g*g + (((767-rmean)*b*b)>>8));
// }

// likely max: 2294
fn distance_between(pixel: &[u8; 4], color: &[u8; 3]) -> i16 {
    let rmean: i16 = (pixel[0] as i16 + color[0] as i16) / 2;
    let r: i16 = pixel[0] as i16 - color[0] as i16;
    let g: i16 = pixel[1] as i16 - color[1] as i16;
    let b: i16 = pixel[2] as i16 - color[2] as i16;

    ((2 + (rmean / 256)) * r + 4 * g + (2 + (255 - rmean) / 256) * b).abs()
}

impl PixelOrdering {
    pub(crate) fn order(&self, iter: Copied<ArrayChunks<u8, 4>>, reverse: bool) -> Vec<u8> {
        let iter = match self {
            PixelOrdering::Luminance => iter
                .sorted_unstable_by(|a, b| pixel_to_luminance(a).total_cmp(&pixel_to_luminance(b))),
            PixelOrdering::ColorSimilarity(color) => iter.sorted_unstable_by(|a, b| {
                distance_between(a, color).cmp(&distance_between(b, color))
            }),
        };
        if reverse {
            iter.rev().flatten().collect()
        } else {
            iter.flatten().collect()
        }
    }
}

#[derive(Default)]
pub(crate) struct RowOp {
    pub(crate) slices: Vec<(usize, usize)>,
}

impl RowOp {
    fn add_slice(&mut self, (start, end): (usize, usize)) {
        self.slices.push((start, end));
    }

    fn extend_slices(&mut self, settings: &Settings, row_length: usize) {
        let slices = self.slices.clone();
        self.slices = slices
            .iter()
            .enumerate()
            .map(|(i, slice)| {
                let end = match slices.get(i + 1) {
                    Some(next) => (slice.1 + settings.extend_threshold_right).min(next.1),
                    None => (slice.1 + settings.extend_threshold_right).min(row_length),
                };

                let start = if i > 0 {
                    match slices.get(i - 1) {
                        Some(prev) => {
                            (slice.0.saturating_sub(settings.extend_threshold_left)).max(prev.0)
                        }
                        None => slice.0.saturating_sub(settings.extend_threshold_left),
                    }
                } else {
                    slice.0.saturating_sub(settings.extend_threshold_left)
                };
                (start.max(0), end)
            })
            .collect();
    }

    pub(crate) fn apply_threshold(&mut self, row: &[u8], width: usize, settings: &Settings) {
        let threshold = &settings.threshold;
        let reverse = settings.threshold_reverse;

        let bools: Vec<bool> = {
            match threshold {
                Threshold::Luminance(value) => row
                    .array_chunks::<4>()
                    .map(|x| pixel_to_luminance(x) < *value)
                    .collect(),
                Threshold::ColorSimilarity(value, color) => row
                    .array_chunks::<4>()
                    .map(|x| distance_between(x, color) < *value)
                    .collect(),
            }
        };

        for (key, mut group) in &bools.iter().enumerate().group_by(|(_, b)| *b) {
            if *key != reverse {
                let first = group.next().unwrap();
                if let Some(last) = group.last() {
                    self.add_slice((first.0, last.0 + 1));
                } else {
                    self.add_slice((first.0, first.0 + 1));
                }
            }
        }
        self.extend_slices(&settings, width);
    }
}
