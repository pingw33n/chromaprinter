pub mod filter;
pub mod normalize;

use std::cmp;

use crate::pipeline::Step;
use crate::util::*;

pub use filter::Filter;
pub use normalize::Normalize;

pub const BAND_COUNT: usize = 12;

pub struct Chroma {
    interpolate: bool,
    notes: Vec<u8>,
    notes_frac: Vec<f64>,
    min_index: u32,
    max_index: u32,
    out: Vec<f64>,
}

impl Chroma {
    pub fn new(
        min_freq: u32,
        max_freq: u32,
        frame_len: u32,
        sample_rate: u32,
        interpolate: bool) -> Self
    {
        let mut notes = vec![0; frame_len as usize];
        let mut notes_frac = vec![0.0; frame_len as usize];

        let min_index = cmp::max(1, freq_to_index(min_freq as f64, frame_len, sample_rate));
        let max_index = cmp::min(frame_len / 2, freq_to_index(max_freq as f64, frame_len, sample_rate));
        for i in min_index..max_index {
            let freq = index_to_freq(i, frame_len, sample_rate);
            let octave = freq_to_octave(freq);
            let note = BAND_COUNT as f64 * (octave - octave.floor());

            let i = i as usize;
            notes[i] = note as u8;
            notes_frac[i] = note.fract();
        }

        Self {
            interpolate,
            notes,
            notes_frac,
            min_index,
            max_index,
            out: vec![0.0; BAND_COUNT],
        }
    }
}

impl Step<f64, f64> for Chroma {
    fn process(&mut self, inp: &[f64], _finish: bool) -> usize {
        dbg!(inp.len());
        if inp.is_empty() {
            return 0;
        }
        debug_assert!(inp.len() >= self.max_index as usize);
        for v in self.out.iter_mut() {
            *v = 0.0;
        }
        for i in self.min_index..self.max_index {
            let i = i as usize;
            let note = self.notes[i];
            let energy = inp[i];
            if self.interpolate {
                let note_frac = self.notes_frac[i];
                let (note2, a) = if note_frac < 0.5 {
                    ((note + BAND_COUNT as u8 - 1) % BAND_COUNT as u8,
                        0.5 + note_frac)
                } else if note_frac > 0.5 {
                    ((note + 1) % BAND_COUNT as u8,
                        1.5 - note_frac)
                } else {
                    (note, 1.0)
                };
                self.out[note as usize] += energy * a;
                self.out[note2 as usize] += energy * (1.0 - a);
            }
            else {
                self.out[note as usize] += energy;
            }
        }

        inp.len()
    }

    fn output<'a>(&'a self, inp: &'a [f64], _finish: bool) -> &'a [f64] {
        if inp.len() > 0 {
            &self.out
        } else {
            &[]
        }
    }
}

fn freq_to_octave(freq: f64) -> f64 {
    const BASE: f64 = 440.0 / 16.0;
    (freq / BASE).ln() / 2f64.ln()
}

#[cfg(test)]
mod test {
    use super::*;
    use approx::assert_abs_diff_eq;

    #[test]
    fn chroma() {
        // chroma params, frame, expected
        let data = &[
            // G
            ((10, 510, 256, 1000, false), &[(113, 1.0)],
            [
                1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
                0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            ]),
            // G#
            ((10, 510, 256, 1000, false), &[(112, 1.0)],
            [
                0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
                0.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ]),
            // interpolated B
            ((10, 510, 256, 1000, true), &[(64, 1.0)],
            [
                0.0, 0.286905, 0.713095, 0.0, 0.0, 0.0,
                0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            ]),
            // interpolated A
            ((10, 510, 256, 1000, true), &[(113, 1.0)],
            [
                0.555242, 0.0, 0.0, 0.0, 0.0, 0.0,
                0.0, 0.0, 0.0, 0.0, 0.0, 0.444758,
            ]),
            // interpolated G#
            ((10, 510, 256, 1000, true), &[(112, 1.0)],
            [
                0.401354, 0.0, 0.0, 0.0, 0.0, 0.0,
                0.0, 0.0, 0.0, 0.0, 0.0, 0.598646,
            ]),
        ];

        for ((min_freq, max_freq, frame_len, sample_rate, interpolate), frame_def, expected)
            in data
        {
            let mut chroma = Chroma::new(
                *min_freq, *max_freq, *frame_len, *sample_rate, *interpolate);

            let mut frame = vec![0.0; 128];
            for &(i, v) in *frame_def {
                frame[i] = v;
            }

            assert_eq!(chroma.process(&frame, false), frame.len());

            for (a, e) in chroma.output(&frame, false).iter().zip(expected) {
                assert_abs_diff_eq!(a, e, epsilon = 0.0001);
            }
        }
    }
}