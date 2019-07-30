use super::*;
use std::cmp;

pub struct Windows<T> {
    len: usize,
    step: usize,
    buf: Vec<T>,
    buf_pos: usize,
}

impl<T> Windows<T> {
    pub fn new(len: usize, step: usize) -> Self {
        assert!(len > 0);
        assert!(step <= len);
        Self {
            len,
            step,
            buf: Vec::with_capacity(len * 2),
            buf_pos: 0,
        }
    }

    fn available(&self) -> usize {
        self.buf.len() - self.buf_pos
    }
}

impl<T: Clone> Step<T, T> for Windows<T> {
    fn process<F>(&mut self, mut input: &[T], mut output: F)
        where F: FnMut(&[T])
    {
        while input.len() > 0 {
            if self.buf.is_empty() {
                while input.len() >= self.len {
                    output(&input[..self.len]);
                    input = &input[self.step..];
                }
            }

            let can_buf = cmp::min(input.len(), self.buf.capacity() - self.buf.len());
            self.buf.extend_from_slice(&input[..can_buf]);
            input = &input[can_buf..];

            while self.buf.len() - self.buf_pos >= self.len {
                output(&self.buf[self.buf_pos..self.buf_pos + self.len]);
                self.buf_pos += self.step;
            }

            self.buf.drain(..self.buf_pos);
            self.buf_pos = 0;
        }
    }

    fn finish<F>(&mut self, mut output: F)
        where F: FnMut(&[T])
    {
        if !self.buf.is_empty() {
            output(&self.buf);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use super::test_util::*;

    #[test]
    fn test() {
        let data = &[
            (&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9][..],
                (4, 2),
                &[
                    &[1, 1, 1, 1, 1, 1, 1, 1, 1, 1][..],
                    &[2, 2, 2, 2, 2][..],
                    &[3, 3, 3, 1][..],
                    &[10][..],
                    &[1, 2, 3, 3, 1][..],
                ][..]),
        ];

        for &(input, (len, step), chunk_seqs) in data {
            let mut expected = Vec::new();
            for i in (0..=input.len() - len).step_by(step) {
                expected.push(&input[i..i + len])
            }

            for &chunk_seq in chunk_seqs {
                let mut w = Windows::<i16>::new(len, step);

                let mut input = input;
                let mut actual = Vec::new();

                for &chunk in chunk_seq {
                    w.process(&input[..chunk], collect(&mut actual));
                    input = &input[chunk..];
                }

                assert_eq!(actual, expected);
            }
        }
    }
}