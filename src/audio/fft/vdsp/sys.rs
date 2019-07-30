#![allow(non_camel_case_types)]

use std::os::raw::*;

pub const FFT_RADIX2: FFTRadix = 0;
pub const FFT_RADIX3: FFTRadix = 1;
pub const FFT_RADIX5: FFTRadix = 2;
pub const FFT_FORWARD: FFTDirection = 1;
pub const FFT_INVERSE: FFTDirection = -1;

pub type FFTSetup = *mut c_void;
pub type FFTRadix = c_int;
pub type FFTDirection = c_int;
pub type Length = c_ulong;
pub type Stride = c_int;

#[repr(C)]
pub struct Complex {
    pub real: f32,
    pub imag: f32,
}

#[repr(C)]
pub struct SplitComplex {
    pub realp: *mut f32,
    pub imagp: *mut f32,
}

extern {
    pub fn vDSP_create_fftsetup(log2n: Length, radix: FFTRadix) -> FFTSetup;
    pub fn vDSP_destroy_fftsetup(setup: FFTSetup);
    pub fn vDSP_ctoz(c: *const Complex,
                     ic: Stride,
                     z: *const SplitComplex,
                     iz: Stride,
                     n: Length);
    pub fn vDSP_fft_zrip(setup: FFTSetup,
                         c: *const SplitComplex,
                         ic: Stride,
                         log2n: Length,
                         direction: FFTDirection);
}



