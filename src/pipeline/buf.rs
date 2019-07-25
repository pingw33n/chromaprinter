use super::*;

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
    fn process(&mut self, inp: &[T], finish: bool) -> usize {
        let capacity = self.buf.capacity();

        if self.buf.len() == capacity {
            self.buf.clear();
        }

        if self.buf.is_empty() && (inp.len() >= capacity || finish) {
            return inp.len();
        }

        let consumed = cmp::min(inp.len(), capacity - self.buf.len());
        self.buf.extend_from_slice(&inp[..consumed]);
        consumed
    }

    fn output<'a>(&'a self, inp: &'a [T], finish: bool) -> &'a [T] {
        if self.buf.is_empty() {
            &inp[..inp.len()]
        } else if self.buf.len() == self.buf.capacity() || finish {
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

        assert_eq!(b.process(&[1, 2], false), 2);
        assert_eq!(&b.buf, &[1, 2]);
        assert_eq!(b.output(&[1, 2], false), &[]);

        assert_eq!(b.process(&[3, 4, 5], false), 1);
        assert_eq!(&b.buf, &[1, 2, 3]);
        assert_eq!(b.output(&[3, 4, 5], false), &[1, 2, 3]);

        assert_eq!(b.process(&[4, 5], true), 2);
        assert_eq!(&b.buf, &[]);
        assert_eq!(b.output(&[4, 5], true), &[4, 5]);
    }

    #[test]
    fn direct() {
        let mut b = Buf::with_capacity(3);
        assert!(b.buf.is_empty());

        assert_eq!(b.process(&[1, 2, 3], false), 3);
        assert!(b.buf.is_empty());
        assert_eq!(b.output(&[1, 2, 3], false), &[1, 2, 3]);

        assert_eq!(b.process(&[4, 5, 6, 7], false), 4);
        assert!(b.buf.is_empty());
        assert_eq!(b.output(&[4, 5, 6, 7], false), &[4, 5, 6, 7]);

        assert_eq!(b.process(&[8, 9, 10, 11], true), 4);
        assert!(b.buf.is_empty());
        assert_eq!(b.output(&[8, 9, 10, 11], true), &[8, 9, 10, 11]);
    }
}