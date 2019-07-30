#[cfg(feature = "fftw")]
mod fftw;
mod hwindow;
#[cfg(feature = "vdsp")]
mod vdsp;

use crate::pipeline::{Step, Then};
use crate::pipeline::windows::Windows;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FFTImplKind {
    #[cfg(feature = "fftw")]
    FFTW,

    #[cfg(feature = "vdsp")]
    VDSP,
}

impl Default for FFTImplKind {
    fn default() -> Self {
        use FFTImplKind::*;

        #[cfg(feature = "fftw")]
        { FFTW }

        #[cfg(feature = "vdsp")]
        { VDSP }
    }
}

enum FFTImpl {
    #[cfg(feature = "fftw")]
    FFTW(fftw::FFTW),

    #[cfg(feature = "vdsp")]
    VDSP(vdsp::VDSP),
}

impl FFTImpl {
    pub fn new(kind: FFTImplKind, len: usize) -> Self {
        use FFTImpl::*;
        match kind {
            #[cfg(feature = "fftw")]
            FFTImplKind::FFTW => FFTW(fftw::FFTW::new(len)),

            #[cfg(feature = "vdsp")]
            FFTImplKind::VDSP => VDSP(vdsp::VDSP::new(len)),
        }
    }

    pub fn process(&mut self, inp: &[i16], out: &mut [f64]) {
        use FFTImpl::*;
        match self {
            #[cfg(feature = "fftw")]
            FFTW(v) => v.process(inp, out),

            #[cfg(feature = "vdsp")]
            VDSP(v) => v.process(inp, out),
        }
    }
}

pub struct FFT(Then<i16, i16, f64, Windows<i16>, Internal>);

impl FFT {
    pub fn new(len: usize, overlap: usize) -> Self {
        assert!(len > 0);
        assert!(overlap < len);
        Self(Windows::new(len, len - overlap)
            .then(Internal::new(FFTImplKind::default(), len)))
    }
}

impl Step<i16, f64> for FFT {
    fn process<F>(&mut self, input: &[i16], output: F)
        where F: FnMut(&[f64])
    {
        self.0.process(input, output);
    }

    fn finish<F>(&mut self, output: F)
        where F: FnMut(&[f64])
    {
        self.0.finish(output);
    }
}

struct Internal {
    buf: Vec<f64>,
    imp: FFTImpl,
}

impl Internal {
    pub fn new(impl_kind: FFTImplKind, len: usize) -> Self {
        Self {
            buf: vec![0.0; 1 + len / 2],
            imp: FFTImpl::new(impl_kind, len),
        }
    }
}

impl Step<i16, f64> for Internal {
    fn process<F>(&mut self, input: &[i16], mut output: F)
        where F: FnMut(&[f64])
    {
        self.imp.process(input, &mut self.buf);
        output(&self.buf);
    }

    fn finish<F>(&mut self, _output: F)
        where F: FnMut(&[f64])
    {
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use approx::assert_abs_diff_eq;
    use std::f64::consts::PI;
    use crate::pipeline::test_util::*;

    #[test]
    fn test() {
        const FRAME_COUNT: usize = 3;
        const FRAME_LEN: usize = 32;
        const OVERLAP: usize = 8;
        const INPUT_LEN: usize = FRAME_LEN + (FRAME_COUNT - 1) * (FRAME_LEN - OVERLAP);

        fn sine() -> Vec<i16> {
            const SAMPLE_RATE: u32 = 1000;
            const FREQ: f64 = 7.0 * (SAMPLE_RATE as f64 / 2.0) / (FRAME_LEN as f64 / 2.0);
            (0..INPUT_LEN)
                .map(|i| (i16::max_value() as f64 *
                    (i as f64 * FREQ * 2.0 * PI / SAMPLE_RATE as f64).sin()) as i16)
                .collect()
        }

        fn dc() -> Vec<i16> {
            vec![i16::max_value() / 2; INPUT_LEN]
        }

        type InputFn = fn() -> Vec<i16>;

        let data = &[
            (sine as InputFn, &[
                0.000346154,
                0.000398832,
                0.000565318,
                0.000925474,
                0.001774766,
                0.004868500,
                0.219557510,
                0.494690100,
                0.219551879,
                0.004881113,
                0.001790900,
                0.000948889,
                0.000585357,
                0.000401601,
                0.000301067,
                0.000249106,
                0.000230959,
            ]),
            (dc as InputFn, &[
                0.494690793,
                0.219546939,
                0.004880792,
                0.001789917,
                0.000939219,
                0.000576100,
                0.000385798,
                0.000272900,
                0.000199925,
                0.000149575,
                0.000112941,
                0.000085044,
                0.000062831,
                0.000044396,
                0.000028391,
                0.000013841,
                0.000000056,
            ]),
        ];

        for (input_fn, expected) in data {
            let fft = &mut FFT::new(FRAME_LEN, OVERLAP);

            let input = input_fn();
            let actual = &mut Vec::new();
            for chunk in input.chunks(FRAME_LEN / 3 + 1) {
                fft.process(chunk, collect(actual));
            }

            assert_eq!(actual.len(), FRAME_COUNT);

            for frame in actual {
                for (a, e) in frame.iter().zip(expected.iter()) {
                    let magnitude = a.sqrt() / frame.len() as f64;
                    assert_abs_diff_eq!(magnitude, e, epsilon = 0.001);
                }
            }
        }
    }
}

