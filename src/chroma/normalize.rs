use crate::pipeline::Inplace;

pub struct Normalize {
    threshold: f64,
}

impl Normalize {
    pub fn new(threshold: f64) -> Self {
        Self {
            threshold,
        }
    }
}

impl Inplace<f64> for Normalize {
    fn process(&mut self, in_out: &mut [f64]) {
        let norm = euclidian_norm(in_out);
        if norm < self.threshold {
            for v in in_out.iter_mut() {
                *v = 0.0;
            }
        } else {
            for v in in_out.iter_mut() {
                *v /= norm;
            }
        }
    }
}

fn euclidian_norm(buf: &[f64]) -> f64 {
    let squares: f64 = buf.iter().map(|&v| v * v).sum();
    if squares > 0.0 {
        squares.sqrt()
    } else {
        0.0
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use approx::assert_abs_diff_eq;

    #[test]
    fn euclidian_norm_fn() {
        assert_eq!(euclidian_norm(&[0.1, 0.2, 0.4, 1.0]), 1.1)
    }

    #[test]
    fn test() {
        let buf = &mut [0.1, 0.2, 0.4, 1.0];
        let expected = &[0.090909, 0.181818, 0.363636, 0.909091];
        Normalize::new(0.01).process(buf);
        for (a, e) in buf.iter().zip(expected) {
            assert_abs_diff_eq!(a, e, epsilon = 1e-5);
        }
    }

    #[test]
    fn near_zero() {
        let buf = &mut [0.0, 0.001, 0.002, 0.003];
        Normalize::new(0.01).process(buf);
        assert_eq!(buf, &[0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn zero() {
        let buf = &mut [0.0, 0.0, 0.0, 0.0];
        Normalize::new(0.01).process(buf);
        assert_eq!(buf, &[0.0, 0.0, 0.0, 0.0]);
    }
}