use std::cmp;

pub struct RollingImage {
    data: Vec<f64>,
    width: usize,
    max_height: usize,

    /// Total number of rows pushed.
    /// `row_count % max_height * width` points to where the next row will be written in `data`.
    row_count: usize,
}

impl RollingImage {
    pub fn new(width: usize, max_height: usize) -> Self {
        Self {
            data: vec![0.0; width * max_height],
            width,
            row_count: 0,
            max_height,
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        cmp::min(self.row_count, self.max_height)
    }

    pub fn max_height(&self) -> usize {
        self.max_height
    }

    pub fn area(&self, r1: usize, c1: usize, r2: usize, c2: usize) -> f64 {
        assert!(r1 <= self.height());
        assert!(r2 <= self.height());
        assert!(c1 <= self.width());
        assert!(c2 <= self.width());

        if r1 == r2 || c1 == c2 {
            return 0.0;
        }

        assert!(r2 > r1);
        assert!(c2 > c1);

        if r1 == 0 {
            let row = self.row(r2 - 1);
            if c1 == 0 {
                row[c2 - 1]
            } else {
                row[c2 - 1] - row[c1 - 1]
            }
        } else {
            let row1 = self.row(r1 - 1);
            let row2 = self.row(r2 - 1);
            if c1 == 0 {
                row2[c2 - 1] - row1[c2 - 1]
            } else {
                row2[c2 - 1] - row1[c2 - 1] - row2[c1 - 1] + row1[c1 - 1]
            }
        }
    }

    pub fn push(&mut self, row: &[f64]) {
        assert_eq!(row.len(), self.width);

        let i = self.row_offset(self.row_count);
        let (last_row, next_row) = if i > 0 {
            let (l, r) = self.data.split_at_mut(i);
            (&mut l[i - self.width..], &mut r[..self.width])
        } else {
            let (l, r) = self.data.split_at_mut(self.width);
            let rlen = r.len();
            (&mut r[rlen - self.width..], l)
        };

        partial_sum(row, next_row);

        if self.row_count > 0 {
            for (l, n) in last_row.iter().zip(next_row.iter_mut()) {
                *n += *l;
            }
        }

        self.row_count = self.row_count.checked_add(1).unwrap();
    }

    fn row_offset(&self, abs_idx: usize) -> usize {
        (abs_idx % self.max_height) * self.width
    }

    fn row(&self, rel_idx: usize) -> &[f64] {
        let i = (self.row_count - self.height() + rel_idx) % self.max_height * self.width;
        &self.data[i..i + self.width]
    }
}

fn partial_sum(inp: &[f64], out: &mut [f64]) {
    let mut sum = 0.0;
    for (inp, out) in inp.iter().zip(out.iter_mut()) {
        sum += *inp;
        *out = sum;
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test() {
        let data = &[
            // row, [(area input, area expected)]
            (&[1.0, 2.0, 3.0][..],
            &[
                ((0, 0, 1, 3), 1.0 + 2.0 + 3.0),
            ][..]),

            (&[4.0, 5.0, 6.0][..],
            &[
                ((1, 0, 2, 1), 4.0),
                ((1, 1, 2, 2), 5.0),
                ((1, 2, 2, 3), 6.0),
                ((0, 0, 2, 3), 1.0 + 2.0 + 3.0 + 4.0 + 5.0 + 6.0),
            ][..]),

            (&[7.0, 8.0, 9.0][..],
            &[][..]),

            (&[10.0, 11.0, 12.0][..],
            &[
                ((0, 0, 4, 3), (1.0 + 2.0 + 3.0) + (4.0 + 5.0 + 6.0) + (7.0 + 8.0 + 9.0) + (10.0 + 11.0 + 12.0)),
                ((1, 1, 2, 2), 5.0),
                ((1, 2, 2, 3), 6.0),
                ((0, 0, 2, 3), 1.0 + 2.0 + 3.0 + 4.0 + 5.0 + 6.0),
            ][..]),

            (&[13.0, 14.0, 15.0][..],
            &[
                ((1, 0, 2, 1), 4.0),
                ((1, 1, 2, 2), 5.0),
                ((1, 2, 2, 3), 6.0),
                ((4, 0, 5, 1), 13.0),
                ((4, 1, 5, 2), 14.0),
                ((4, 2, 5, 3), 15.0),
                ((1, 0, 5, 3), (4.0 + 5.0 + 6.0) + (7.0 + 8.0 + 9.0) + (10.0 + 11.0 + 12.0) + (13.0 + 14.0 + 15.0)),
            ][..]),

            (&[16.0, 17.0, 18.0][..],
            &[
                ((1, 0, 2, 1), 7.0),
                ((1, 1, 2, 2), 8.0),
                ((1, 2, 2, 3), 9.0),
                ((4, 0, 5, 1), 16.0),
                ((4, 1, 5, 2), 17.0),
                ((4, 2, 5, 3), 18.0),
                ((1, 0, 5, 3), (7.0 + 8.0 + 9.0) + (10.0 + 11.0 + 12.0) + (13.0 + 14.0 + 15.0) + (16.0 + 17.0 + 18.0)),
            ][..]),
        ];

        const MAX_HEIGHT: usize = 5;

        let mut im = RollingImage::new(3, MAX_HEIGHT);
        assert_eq!(im.width(), 3);
        assert_eq!(im.max_height(), MAX_HEIGHT);
        assert_eq!(im.height(), 0);

        for (i, &(row, areas)) in data.iter().enumerate() {
            im.push(row);

            assert_eq!(im.width(), 3);
            assert_eq!(im.max_height(), MAX_HEIGHT);
            assert_eq!(im.height(), cmp::min(i + 1, im.max_height()));

            for &((r1, c1, r2, c2), exp) in areas {
                assert_eq!(im.area(r1, c1, r2, c2), exp);
            }
        }
    }
}