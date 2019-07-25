use std::mem;

use super::Elem;

pub struct GaussianFilter {
    inp: Vec<Elem>,
    buf: Vec<Elem>,
    sigma: f64,
    n: usize,
}

impl GaussianFilter {
    pub fn new(sigma: f64, n: usize) -> Self {
        Self {
            inp: Vec::new(),
            buf: Vec::new(),
            sigma,
            n,
        }
    }

    pub fn apply(&mut self, inp: &[Elem]) -> &[Elem] {
        self.inp.clear();
        self.inp.extend_from_slice(inp);
        self.buf.resize(inp.len(), 0.0);

        gaussian_filter(&mut self.inp, &mut self.buf, self.sigma, self.n)
    }
}

fn gaussian_filter<'a>(inp: &'a mut [Elem], buf: &'a mut [Elem], sigma: f64, n: usize) -> &'a [Elem] {
    let w = ((12.0 * sigma * sigma / n as f64 + 1.0).sqrt()).floor() as usize;
    let wl = w - (w % 2 == 0) as usize;
    let wu = wl + 2;
    let m = (-(12.0 * sigma * sigma - (n * wl * wl + 4 * n * wl + 3 * n) as f64)
        / (4 * wl + 4) as f64).round() as usize;

    let mut i = 0;
    {
        let mut inp = &mut *inp;
        let mut out = &mut *buf;
        while i < m {
            box_filter(inp, out, wl);
            mem::swap(&mut inp, &mut out);
            i += 1;
        }
        while i < n {
            box_filter(inp, out, wu);
            mem::swap(&mut inp, &mut out);
            i += 1;
        }
    }

    if i % 2 == 0 {
        inp
    } else {
        buf
    }
}

struct PingPongIter<'a, T> {
    slice: &'a [T],
    pos: usize,
    reverse: bool,
}

impl<'a, T: Copy> PingPongIter<'a, T> {
    pub fn new(slice: &'a [T]) -> Self {
        Self {
            slice,
            pos: 0,
            reverse: false,
        }
    }

    pub fn get(&self) -> T {
        self.slice[self.pos]
    }

    pub fn next(&mut self) {
        self.go(self.reverse)
    }

    pub fn prev(&mut self) {
        self.go(!self.reverse)
    }

    fn go(&mut self, reverse: bool) {
        if !reverse {
            let next_pos = self.pos + 1;
            if next_pos == self.slice.len() {
                self.reverse = !self.reverse;
            } else {
                self.pos = next_pos
            }
        } else {
            if self.pos == 0 {
                self.reverse = !self.reverse;
            } else {
                self.pos -= 1;
            }
        }
    }
}

fn box_filter(inp: &[Elem], out: &mut [Elem], width: usize) {
    if inp.is_empty() || width == 0 {
        return;
    }

    let wl = width / 2;
    let wr = width - wl;

    let mut out_it = out.iter_mut();
    let mut out = move |sum| *out_it.next().unwrap() = sum / width as Elem;

    let mut it1 = PingPongIter::new(inp);
    let mut it2 = PingPongIter::new(inp);

    for _ in 0..wl {
        it1.prev();
        it2.prev();
    }

    let mut sum = 0.0;
    for _ in 0..width {
        sum += it2.get();
        it2.next();
    }

    if inp.len() > width {
        for _ in 0..wl {
            out(sum);
            sum += it2.get() - it1.get();
            it1.next();
            it2.next();
        }
        for _ in 0..inp.len() - width - 1 {
            out(sum);
            sum += it2.get() - it1.get();
            it1.pos += 1;
            it2.pos += 1;
        }
        for _ in 0..wr + 1 {
            out(sum);
            sum += it2.get() - it1.get();
            it1.next();
            it2.next();
        }
    } else {
        for _ in 0..inp.len() {
            out(sum);
            sum += it2.get() - it1.get();
            it1.next();
            it2.next();
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use approx::assert_abs_diff_eq;

    #[test]
    fn iter() {
        let mut it = PingPongIter::new(&[1, 2, 3, 4, 5, 6, 7, 8, 9]);

        it.prev();
        it.prev();
        it.prev();

        assert_eq!(it.get(), 3);
        assert!(it.reverse);

        it.next();
        assert_eq!(it.get(), 2);
        assert!(it.reverse);

        it.next();
        assert_eq!(it.get(), 1);
        assert!(it.reverse);

        it.next();
        assert_eq!(it.get(), 1);
        assert!(!it.reverse);

        it.next();
        assert_eq!(it.get(), 2);
        assert!(!it.reverse);
    }

    #[test]
    fn box_filter_fn() {
        let data = &[
            // width=1
            (&[1.0, 2.0, 4.0][..], 1,
                &[1.0, 2.0, 4.0][..]),

            // width=2
            (&[1.0, 2.0, 4.0][..], 2,
                &[1.0, 1.5, 3.0][..]),

            // width=3
            (&[1.0, 2.0, 4.0][..], 3,
                &[1.333333333, 2.333333333, 3.333333333][..]),

            // width=4
            (&[1.0, 2.0, 4.0][..], 4,
                &[1.5, 2.0, 2.75][..]),

            // width=5
            (&[1.0, 2.0, 4.0][..], 5,
                &[2.0, 2.4, 2.6][..]),
        ];
        for (inp, w, exp) in data {
            let act = &mut vec![0.0; inp.len()];
            box_filter(inp, act, *w);

            for (a, e) in act.iter().zip(exp.iter()) {
                assert_abs_diff_eq!(a, e, epsilon = 1e-5);
            }
        }
    }

    #[test]
    fn gaussian_filter_fn() {
        let data = &[
            (&[1.0, 2.0, 4.0][..], 1.6, 3,
                &[1.88888889, 2.33333333, 2.77777778]),
            (&[1.0, 2.0, 4.0][..], 3.6, 4,
                &[2.3322449, 2.33306122, 2.33469388]),
        ];
        for (inp, sigma, n, exp) in data {
            let inp = &mut inp.to_vec();
            let buf = &mut vec![0.0; inp.len()];
            let act = gaussian_filter(inp, buf, *sigma, *n);

            for (a, e) in act.iter().zip(exp.iter()) {
                assert_abs_diff_eq!(a, e, epsilon = 1e-5);
            }
        }
    }
}