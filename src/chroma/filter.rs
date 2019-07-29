use super::*;
use crate::pipeline::Step;

pub struct Filter {
    coefs: &'static [f64],
    buf: Vec<Vec<f64>>,
    buf_pos: usize,
    buf_ready: usize,
    out: Vec<f64>,
}

impl Filter {
    pub fn new(coefs: &'static [f64]) -> Self {
        let mut buf = Vec::with_capacity(coefs.len());
        for _ in 0..coefs.len() {
            buf.push(vec![0.0; BAND_COUNT])
        }

        Self {
            coefs,
            buf,
            buf_pos: 0,
            buf_ready: 1,
            out: vec![0.0; BAND_COUNT],
        }
    }
}

impl Step<f64, f64> for Filter {
    fn process<F>(&mut self, input: &[f64], mut output: F)
        where F: FnMut(&[f64])
    {
        let len = self.buf.len();
        self.buf[self.buf_pos].copy_from_slice(input);
        self.buf_pos = (self.buf_pos + 1) % len;
        if self.buf_ready == len {
            for v in self.out.iter_mut() {
                *v = 0.0;
            }
            for i in 0..BAND_COUNT {
                for (j, coef) in self.coefs.iter().enumerate() {
                    self.out[i] += self.buf[(self.buf_pos + j) % len][i] * coef;
                }
            }
            output(&self.out)
        } else {
            self.buf_ready += 1;
        }
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
    use crate::pipeline::test_utils::*;

    #[test]
    fn test() {
        // coefficients, inputs, expected
        let data = &[
            // blur2
            (&[0.5, 0.5][..], &[
                [0.0, 5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                [1.0, 6.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                [2.0, 7.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            ][..], [0.5, 5.5, 1.5, 6.5]),
            // blur3
            (&[0.5, 0.7, 0.5][..], &[
                [0.0, 5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                [1.0, 6.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                [2.0, 7.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                [3.0, 8.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            ], [1.7, 10.199999999999999, 3.399999999999999, 11.899999999999999]),
            // diff
            (&[1.0, -1.0], &[
                [0.0, 5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                [1.0, 6.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                [2.0, 7.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            ], [-1.0, -1.0, -1.0, -1.0]),
        ];

        for (coefficients, inputs, expected) in data {
            let mut filter = Filter::new(coefficients);

            let actual = &mut Vec::new();
            for input in *inputs {
                filter.process(input, collect(actual));
            }

            let points = &[(0, 0), (0, 1), (1, 0), (1, 1)];
            for (&(i, j), e) in points.iter().zip(expected) {
                let a = actual[i][j];
                assert_abs_diff_eq!(a, e, epsilon = 1e-5);
            }
        }
    }
}