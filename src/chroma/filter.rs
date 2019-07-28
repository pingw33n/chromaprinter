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
        // FIXME use coefficients.len() instead of BUF_CAP
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
    fn process(&mut self, inp: &[f64]) -> usize {
        self.buf[self.buf_pos].copy_from_slice(inp);
        let coefs_len = self.coefs.len();
        self.buf_pos = (self.buf_pos + 1) % coefs_len;
        if self.buf_ready >= coefs_len {
            for v in self.out.iter_mut() {
                *v = 0.0;
            }
            let buf_start = self.buf_pos % coefs_len;
            for i in 0..BAND_COUNT {
                for (j, coef) in self.coefs.iter().enumerate() {
                    self.out[i] += self.buf[(buf_start + j) % coefs_len][i] * coef;
                }
            }
        }
        self.buf_ready = self.buf_ready.saturating_add(1);
        inp.len()
    }

    fn finish(&mut self) {}

    fn output<'a>(&'a self, _inp: &'a [f64]) -> &'a [f64] {
        if self.buf_ready > self.coefs.len() {
            &self.out
        } else {
            &[]
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use approx::assert_abs_diff_eq;

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
//            (&[0.5, 0.7, 0.5], &[
//                [0.0, 5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
//                [1.0, 6.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
//                [2.0, 7.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
//                [3.0, 8.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
//            ], [1.7, 10.199999999999999, 3.399999999999999, 11.899999999999999]),
//            // diff
//            (&[1.0, -1.0], &[
//                [0.0, 5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
//                [1.0, 6.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
//                [2.0, 7.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
//            ], [-1.0, -1.0, -1.0, -1.0]),
        ];

        let mut actual = [[0.0; BAND_COUNT]; 2];

        for (coefficients, inputs, expected) in data {
            let mut filter = Filter::new(*coefficients);

            let mut a_i = 0;
            for (i, input) in inputs.iter().enumerate() {
                let exp = i >= coefficients.len() - 1;
                assert_eq!(filter.process(input), input.len());
                let out = filter.output(input);
                assert_eq!(!out.is_empty(), exp);
                if !out.is_empty() {
                    actual[a_i].copy_from_slice(out);
                    a_i += 1;
                }
            }

            let points = &[(0, 0), (0, 1), (1, 0), (1, 1)];
            for (&(i, j), e) in points.iter().zip(expected) {
                let a = actual[i][j];
                assert_abs_diff_eq!(a, e, epsilon = 1e-5);
            }
        }
    }
}