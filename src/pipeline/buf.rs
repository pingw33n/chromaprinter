use super::*;
use std::cmp;

pub struct Buf<T> {
    buf: Vec<T>,
}

impl<T> Buf<T> {
    pub fn with_capacity(capacity: usize) -> Self {
        assert!(capacity > 0);
        Self {
            buf: Vec::with_capacity(capacity),
        }
    }
}

impl<T: Clone> Step<T, T> for Buf<T> {
    fn process<F>(&mut self, mut input: &[T], mut output: F)
        where F: FnMut(&[T])
    {
        let cap = self.buf.capacity();

        while input.len() > 0 {
            if self.buf.is_empty() {
                while input.len() >= cap {
                    output(&input[..cap]);
                    input = &input[cap..];
                }
            }

            let can_buf = cmp::min(input.len(), cap - self.buf.len());
            self.buf.extend_from_slice(&input[..can_buf]);

            if self.buf.len() == cap {
                output(&self.buf);
                self.buf.clear();
            }

            input = &input[can_buf..];
        }
    }

    fn finish<F>(&mut self, mut output: F)
        where F: FnMut(&[T])
    {
        if self.buf.len() > 0 {
            output(&self.buf);
            self.buf.clear();
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use super::test_utils::*;

    #[test]
    fn indirect() {
        let b = &mut Buf::with_capacity(3);
        assert!(b.buf.is_empty());

        assert!(process(b, &[1, 2]).is_empty());
        assert_eq!(&b.buf, &[1, 2]);

        assert_eq!(&process(b, &[3, 4, 5]), &[vec![1, 2, 3]]);
        assert_eq!(&b.buf, &[4, 5]);

        assert_eq!(&process(b, &[6, 7, 8, 9]), &[vec![4, 5, 6], vec![7, 8, 9]]);
        assert_eq!(&b.buf, &[]);

        assert!(process(b, &[10]).is_empty());
        assert_eq!(&b.buf, &[10]);

        assert_eq!(&finish(b), &[vec![10]]);
        assert_eq!(&b.buf, &[]);
    }

    #[test]
    fn direct() {
        let b = &mut Buf::with_capacity(3);
        assert!(b.buf.is_empty());

        assert_eq!(&process(b, &[1, 2, 3]), &[vec![1, 2, 3]]);
        assert!(b.buf.is_empty());

        assert_eq!(&process(b, &[4, 5, 6, 7]), &[vec![4, 5, 6]]);
        assert_eq!(&b.buf, &[7]);

        assert_eq!(&process(b, &[8, 9]), &[vec![7, 8, 9]]);
        assert!(b.buf.is_empty());

        assert_eq!(&process(b, &[9, 10, 11, 12, 13, 14]), &[vec![9, 10, 11], vec![12, 13, 14]]);
        assert!(b.buf.is_empty());

        assert!(finish(b).is_empty());
        assert_eq!(&b.buf, &[]);
    }
}