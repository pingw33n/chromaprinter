use super::*;

pub struct Buf<T> {
    buf: Vec<T>,
    finished: bool,
}

impl<T> Buf<T> {
    pub fn with_capacity(capacity: usize) -> Self {
        assert!(capacity > 0);
        Self {
            buf: Vec::with_capacity(capacity),
            finished: false,
        }
    }
}

impl<T: Clone> Step<T, T> for Buf<T> {
    fn process(&mut self, inp: &[T]) -> usize {
        let capacity = self.buf.capacity();

        if self.buf.len() == capacity {
            self.buf.clear();
        }

        if self.buf.is_empty() && inp.len() >= capacity {
            return capacity;
        }

        let consumed = cmp::min(inp.len(), capacity - self.buf.len());
        self.buf.extend_from_slice(&inp[..consumed]);
        consumed
    }

    fn finish(&mut self) {
        self.finished = true;
    }

    fn output<'a>(&'a self, inp: &'a [T]) -> &'a [T] {
        dbg!(self.finished);
        if self.buf.is_empty() {
            &inp[..self.buf.capacity()]
        } else if self.buf.len() == self.buf.capacity() || self.finished {
            &self.buf
        } else {
            &[]
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn indirect() {
        let mut b = Buf::with_capacity(3);
        assert!(b.buf.is_empty());

        assert_eq!(b.process(&[1, 2]), 2);
        assert_eq!(&b.buf, &[1, 2]);
        assert_eq!(b.output(&[1, 2]), &[]);

        assert_eq!(b.process(&[3, 4, 5]), 1);
        assert_eq!(&b.buf, &[1, 2, 3]);
        assert_eq!(b.output(&[3, 4, 5]), &[1, 2, 3]);

        assert_eq!(b.process(&[4, 5]), 2);
        assert_eq!(&b.buf, &[4, 5]);
        assert_eq!(b.output(&[4, 5]), &[]);

        assert_eq!(b.process(&[]), 0);
        assert_eq!(&b.buf, &[4, 5]);
        assert_eq!(b.output(&[]), &[]);

        b.finish();
        assert_eq!(b.output(&[]), &[4, 5]);
    }

    #[test]
    fn direct() {
        let mut b = Buf::with_capacity(3);
        assert!(b.buf.is_empty());

        assert_eq!(b.process(&[1, 2, 3]), 3);
        assert!(b.buf.is_empty());
        assert_eq!(b.output(&[1, 2, 3]), &[1, 2, 3]);

        assert_eq!(b.process(&[4, 5, 6, 7]), 3);
        assert!(b.buf.is_empty());
        assert_eq!(b.output(&[4, 5, 6, 7]), &[4, 5, 6]);

        assert_eq!(b.process(&[7]), 1);
        assert_eq!(&b.buf, &[7]);
        assert_eq!(b.output(&[]), &[]);

        assert_eq!(b.process(&[]), 0);
        assert_eq!(&b.buf, &[7]);
        assert_eq!(b.output(&[]), &[]);

        b.finish();
        assert_eq!(b.output(&[]), &[7]);
    }
}