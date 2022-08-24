use std::ops::RangeInclusive;

use itertools::Itertools;

pub(crate) enum Threshold {
    Luminance(f32),
    Red(u8),
    Green(u8),
    Blue(u8),
    // HueDegrees(u16),
}

enum Ordering {
    Luminance,
    Red,
    Green,
    Blue,
    // HueDegrees,
}

#[derive(Default)]
pub(crate) struct RowOp {
    pub(crate) slices: Vec<RangeInclusive<usize>>,
}

impl RowOp {
    fn add_slice(&mut self, range: RangeInclusive<usize>) {
        self.slices.push(range);
    }

    pub(crate) fn apply_threshold(&mut self, row: &[u8], threshold: Threshold, reverse: bool) {
        let mut bools: Vec<bool> = {
            match threshold {
                Threshold::Luminance(value) => row
                    .chunks_exact(3)
                    .map(|x| (x[0] + x[1] * 3 + x[2] * 2) as f32 / 6. < value)
                    .collect(),
                Threshold::Red(value) => row.chunks_exact(3).map(|x| x[0] < value).collect(),
                Threshold::Green(value) => row.chunks_exact(3).map(|x| x[1] < value).collect(),
                Threshold::Blue(value) => row.chunks_exact(3).map(|x| x[2] < value).collect(),
            }
        };

        if reverse {
            bools.reverse();
        }

        for (key, mut group) in &bools.iter().enumerate().group_by(|(_, b)| *b) {
            if *key {
                let first = group.next().unwrap();
                if let Some(last) = group.last() {
                    self.add_slice(first.0..=last.0);
                } else {
                    self.add_slice(first.0..=first.0);
                }
            }
        }
    }
}
