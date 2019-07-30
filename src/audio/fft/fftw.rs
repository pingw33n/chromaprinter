use ::fftw::array::AlignedVec;
use ::fftw::plan::{R2RPlan, R2RPlan64};
use ::fftw::types::{Flag, R2RKind};

use super::*;

pub struct FFTW {
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