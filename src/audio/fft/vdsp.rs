mod sys;

use super::hwindow::HWindow;

pub struct VDSP {
    window: HWindow<f32>,
    input: Box<[f32]>,
    buf_real: Box<[f32]>,
    buf_imag: Box<[f32]>,
    log2n: sys::Length,
    setup: sys::FFTSetup,
}

impl VDSP {
    pub fn new(len: usize) -> Self {
        assert_eq!(len.count_ones(), 1);
        let log2n = len.trailing_zeros() as sys::Length;
        let setup = unsafe { sys::vDSP_create_fftsetup(log2n, sys::FFT_RADIX2) };
        assert!(!setup.is_null());

        Self {
            window: HWindow::new(len, 0.5 / i16::max_value() as f32),
            input: vec![0.0; len].into(),
            buf_real: vec![0.0; len / 2].into(),
            buf_imag: vec![0.0; len / 2].into(),
            log2n,
            setup,
        }
    }

    pub fn process(&mut self, inp: &[i16], out: &mut [f64]) {
        fn sqr(v: f32) -> f64 {
            (v * v) as f64
        }

        let inp_buf = &mut self.input[..inp.len()];

        self.window.apply(inp, inp_buf);

        unsafe {
            let z = sys::SplitComplex {
                realp: self.buf_real.as_mut_ptr(),
                imagp: self.buf_imag.as_mut_ptr(),
            };
            sys::vDSP_ctoz(inp_buf.as_ptr() as *const sys::Complex, 2,
                           &z, 1,
                           self.buf_real.len() as sys::Length);
            sys::vDSP_fft_zrip(self.setup, &z, 1, self.log2n, sys::FFT_FORWARD);
        }

        out[0] = sqr(self.buf_real[0]);
        out[self.buf_real.len()] = sqr(self.buf_imag[0]);
        for i in 1..self.buf_real.len() {
            out[i] = sqr(self.buf_real[i]) + sqr(self.buf_imag[i]);
        }
    }
}

impl Drop for VDSP {
    fn drop(&mut self) {
        unsafe { sys::vDSP_destroy_fftsetup(self.setup) }
    }
}