use super::*;

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
    fn process(&mut self, inp: &[T], _finish: bool) -> usize {
        if self.buf.len() == self.buf.capacity() {
            self.buf.drain(..self.buf_pos);
            self.buf_pos = 0;
        }

        if self.available() >= self.len {
            dbg!(self.buf_pos);
            self.buf_pos += self.step;
        }

        let consumed = cmp::min(inp.len(), self.buf.capacity() - self.buf.len());
        self.buf.extend_from_slice(&inp[..consumed]);
        consumed
    }

    fn output<'a>(&'a self, _inp: &'a [T], finish: bool) -> &'a [T] {
        dbg!((self.available(), self.len, self.step));
        if self.available() >= self.len || finish {
            &self.buf[self.buf_pos..cmp::min(self.buf_pos + self.len, self.buf.len())]
        } else {
            &[]
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

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
                    let mut inp = &input[..chunk];
                    loop {
                        let consumed = w.process(inp, false);
                        dbg!(consumed);
                        let out = w.output(inp, false);
                        if consumed == 0 && out.is_empty() {
                            break;
                        }
                        if out.len() > 0 {
                            actual.push(out.to_vec());
                        }
                        inp = &inp[consumed..];
                    }

                    assert!(inp.is_empty());

                    input = &input[chunk..];
                }

                assert_eq!(actual, expected);
            }
        }
    }
}