use std::{iter::Copied, slice::ArrayChunks};

use itertools::Itertools;

#[derive(strum_macros::Display, PartialEq, Clone)]
pub(crate) enum Threshold {
    Luminance(f32),
    Red(u8),
    Green(u8),
    Blue(u8),
    // HueDegrees(u16),
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
    Red,
    Green,
    Blue,
    // HueDegrees,
}

fn pixel_to_luminance(pixel: &[u8; 4]) -> f32 {
    (pixel[0] as usize + pixel[1] as usize * 3 + pixel[2] as usize * 2) as f32 / 6.
}

impl PixelOrdering {
    pub(crate) fn order(&self, iter: Copied<ArrayChunks<u8, 4>>, reverse: bool) -> Vec<u8> {
        let iter = match self {
            PixelOrdering::Luminance => iter
                .sorted_unstable_by(|a, b| pixel_to_luminance(a).total_cmp(&pixel_to_luminance(b)))
                .flatten(),
            PixelOrdering::Red => iter.sorted_unstable_by(|a, b| a[0].cmp(&b[0])).flatten(),
            PixelOrdering::Green => iter.sorted_unstable_by(|a, b| a[1].cmp(&b[0])).flatten(),
            PixelOrdering::Blue => iter.sorted_unstable_by(|a, b| a[2].cmp(&b[0])).flatten(),
        };
        if reverse {
            iter.rev().collect()
        } else {
            iter.collect()
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

    pub(crate) fn apply_threshold(&mut self, row: &[u8], threshold: &Threshold, reverse: bool) {
        let bools: Vec<bool> = {
            match threshold {
                Threshold::Luminance(value) => row
                    .array_chunks::<4>()
                    .map(|x| pixel_to_luminance(x) < *value)
                    .collect(),
                Threshold::Red(value) => row.array_chunks::<4>().map(|x| x[0] < *value).collect(),
                Threshold::Green(value) => row.array_chunks::<4>().map(|x| x[1] < *value).collect(),
                Threshold::Blue(value) => row.array_chunks::<4>().map(|x| x[2] < *value).collect(),
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
    }
}
