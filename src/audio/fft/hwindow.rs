use std::cmp;
use std::f64::consts::PI;

pub struct HWindow {
    window: Vec<f64>,
}

impl HWindow {
    pub fn new(len: usize, scale: f64) -> Self {
        let mut window = vec![0.0; len];
        Self::init(&mut window, scale);
        Self {
            window,
        }
    }

    pub fn apply(&self, inp: &[i16], out: &mut [f64]) {
        assert!(out.len() >= inp.len());
        let e = cmp::min(inp.len(), self.window.len());
        for i in 0..e {
            out[i] = inp[i] as f64 * self.window[i];
        }
    }

    fn init(buf: &mut [f64], scale: f64) {
        assert!(!buf.is_empty());
        let x = (buf.len() - 1) as f64;
        for (i, v) in buf.iter_mut().enumerate() {
            *v = scale * (0.54 - 0.46 * (i as f64 * 2.0 * PI / x).cos())
        }
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

        let actual = HWindow::new(10, 1.0);

        for (a, e) in actual.window.iter().zip(expected) {
            assert_abs_diff_eq!(a, e, epsilon = 1e-8);
        }
    }

    #[test]
    fn apply() {
        let expected = &[0.08, 0.187619556165, 0.460121838273, 0.77, 0.972258605562,
            0.972258605562, 0.77, 0.460121838273, 0.187619556165, 0.08];

        let win = HWindow::new(10, 1.0 / i16::max_value() as f64);

        let input = &[i16::max_value(); 10];
        let actual = &mut [0.0; 10];
        win.apply(input, actual);

        for (a, e) in actual.iter().zip(expected) {
            assert_abs_diff_eq!(a, e, epsilon = 1e-8);
        }
    }
}