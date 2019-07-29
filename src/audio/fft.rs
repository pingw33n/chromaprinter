mod hwindow;

use fftw::array::AlignedVec;
use fftw::plan::{R2RPlan, R2RPlan64};
use fftw::types::{Flag, R2RKind};

use crate::pipeline::{Step, Then};
use crate::pipeline::windows::Windows;
use hwindow::HWindow;

pub struct FFT(Then<i16, i16, f64, Windows<i16>, Internal>);

impl FFT {
    pub fn new(len: usize, overlap: usize) -> Self {
        assert!(len > 0);
        assert!(overlap < len);
        Self(Windows::new(len, len - overlap)
            .then(Internal::new(len)))
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
    imp: FFTW,
}

impl Internal {
    pub fn new(len: usize) -> Self {
        Self {
            buf: vec![0.0; 1 + len / 2],
            imp: FFTW::new(len),
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

struct FFTW {
    window: HWindow,
    input: AlignedVec<f64>,
    output: AlignedVec<f64>,
    plan: R2RPlan64,
}

impl FFTW {
    pub fn new(len: usize) -> Self {
        let window = HWindow::new(len, 1.0 / i16::max_value() as f64);

        let input = AlignedVec::new(len);
        let output = AlignedVec::new(len);
        let plan = R2RPlan::aligned(&[len], R2RKind::FFTW_R2HC, Flag::Estimate).unwrap();

        Self {
            window,
            input,
            output,
            plan,
        }
    }

    pub fn process(&mut self, inp: &[i16], out: &mut [f64]) {
        fn sqr(v: f64) -> f64 {
            v * v
        }

        let inp_buf = &mut self.input[..inp.len()];

        self.window.apply(inp, inp_buf);

        self.plan.r2r(inp_buf, &mut self.output).unwrap();

        out[0] = sqr(self.output[0]);
        let half = self.output.len() / 2;
        out[half] = sqr(self.output[half]);
        let mut rev_i = self.output.len() - 1;
        for i in 1..half {
            out[i] = sqr(self.output[i]) + sqr(self.output[rev_i]);
            rev_i -= 1;
        }
    }
}



