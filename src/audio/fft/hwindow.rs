use num_traits::{cast, NumCast};
use num_traits::float::{Float, FloatConst};
use std::cmp;

pub struct HWindow<T: Float> {
    window: Vec<T>,
}

impl<T: Float + FloatConst> HWindow<T> {
    pub fn new(len: usize, scale: T) -> Self {
        let mut window = vec![T::zero(); len];
        Self::init(&mut window, scale);
        Self {
            window,
        }
    }

    pub fn apply(&self, inp: &[i16], out: &mut [T]) {
        assert!(out.len() >= inp.len());
        let e = cmp::min(inp.len(), self.window.len());
        for i in 0..e {
            out[i] = Self::c(inp[i]) * self.window[i];
        }
    }

    fn init(buf: &mut [T], scale: T) {
        assert!(!buf.is_empty());
        let x = Self::c(buf.len() - 1);
        for (i, v) in buf.iter_mut().enumerate() {
            *v = scale * (Self::c(0.54) - Self::c(0.46) *
                (Self::c(i) * Self::c(2.0) * FloatConst::PI() / x).cos())
        }
    }

    fn c<U: NumCast>(v: U) -> T {
        cast::<_, T>(v).unwrap()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use approx::assert_abs_diff_eq;

    #[test]
    fn init() {
        let expected = &[0.08, 0.187619556165, 0.460121838273, 0.77, 0.972258605562,
            0.972258605562, 0.77, 0.460121838273, 0.187619556165, 0.08];

        let actual = HWindow::<f32>::new(10, 1.0);

        for (a, e) in actual.window.iter().zip(expected) {
            assert_abs_diff_eq!(a, e, epsilon = 1e-5);
        }
    }

    #[test]
    fn apply() {
        let expected = &[0.08, 0.187619556165, 0.460121838273, 0.77, 0.972258605562,
            0.972258605562, 0.77, 0.460121838273, 0.187619556165, 0.08];

        let win = HWindow::<f64>::new(10, 1.0 / i16::max_value() as f64);

        let input = &[i16::max_value(); 10];
        let actual = &mut [0.0; 10];
        win.apply(input, actual);

        for (a, e) in actual.iter().zip(expected) {
            assert_abs_diff_eq!(a, e, epsilon = 1e-8);
        }
    }
}