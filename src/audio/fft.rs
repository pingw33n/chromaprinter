mod fftw;
mod hwindow;

use crate::pipeline::{Step, Then};
use crate::pipeline::windows::Windows;
use hwindow::HWindow;

#[derive(Clone, Copy, Eq, PartialEq)]
enum FFTImplKind {
    FFTW,
}

impl Default for FFTImplKind {
    fn default() -> Self {
        FFTImplKind::FFTW
    }
}

enum FFTImpl {
    FFTW(fftw::FFTW),
}

impl FFTImpl {
    pub fn new(kind: FFTImplKind, len: usize) -> Self {
        use FFTImpl::*;
        match kind {
            FFTImplKind::FFTW => FFTW(fftw::FFTW::new(len)),
        }
    }

    pub fn process(&mut self, inp: &[i16], out: &mut [f64]) {
        use FFTImpl::*;
        match self {
            FFTW(v) => v.process(inp, out),
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

