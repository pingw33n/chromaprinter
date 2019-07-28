pub mod buf;
pub mod windows;

use std::marker::PhantomData;
use std::cmp;

pub use buf::Buf;
pub use windows::Windows;

// empty input sequence is special and shouldn't be treated as a value
// finish() called once after previously calling process() until idle
pub trait Step<I, O> {
    fn process(&mut self, inp: &[I]) -> usize;

    fn finish(&mut self);

    fn output<'a>(&'a self, inp: &'a [I]) -> &'a [O];

    fn then<S, U>(self, step: S) -> Then<I, O, U, Self, S>
        where S: Step<O, U>,
              Self: Sized,
    {
        Then::new(self, step)
    }

    fn then_inplace<S>(self, step: S) -> ThenInplace<I, O, Self, S>
        where S: Inplace<O>,
              O: Clone,
              Self: Sized,
    {
        ThenInplace::new(self, step)
    }
}

pub struct Then<I, O, U, S1, S2> {
    first: S1,
    second: S2,
    /// Amount consumed from input by the `first` step.
    consumed: usize,
    /// Position in the `first` step's output.
    pos: usize,
    /// Next position in the `first` step's output.
    next_pos: usize,
    finished: bool,
    _ty: PhantomData<(I, O, U)>,
}

impl<I, O, U, S1, S2> Then<I, O, U, S1, S2>
    where S1: Step<I, O>,
          S2: Step<O, U>,
{
    fn new(first: S1, second: S2) -> Self {
        Self {
            first,
            second,
            consumed: 0,
            pos: 0,
            next_pos: 0,
            finished: false,
            _ty: PhantomData,
        }
    }

    fn process_second(&mut self, inp: &[I]) -> usize {
        let mut second_finished = false;
        let first_out = self.first.output(inp);
        loop {
            let second_inp = &first_out[self.pos..];
            let second_cons = self.second.process(second_inp);

            if self.finished && !second_finished {
                second_finished = true;
                continue;
            }

            self.next_pos = self.pos + second_cons;
            assert!(self.next_pos <= first_out.len());

            // If the second step consumed all we're done.
            if self.next_pos == first_out.len() {
                self.next_pos = 0;
                break self.consumed;
            }

            // If the second step consumed nothing we're done.
            if second_cons == 0 {
                assert!(self.second.output(second_inp).len() > 0);
                break 0;
            }
            // If the second step produced some output we're done.
            if self.second.output(second_inp).len() > 0 {
                break 0;
            }

            // Otherwise continue pushing data to the second step.
            self.pos = self.next_pos;
        }
    }
}

impl<I, O, U, S1, S2> Step<I, U> for Then<I, O, U, S1, S2>
    where S1: Step<I, O>,
          S2: Step<O, U>,
{
    fn process(&mut self, inp: &[I]) -> usize {
        self.pos = self.next_pos;
        if self.pos == 0 || self.pos == self.consumed {
            self.consumed = self.first.process(inp);
        }
        self.process_second(inp)
    }

    fn finish(&mut self) {
        self.first.finish();
        self.process_second(&[]);
    }

    fn output<'a>(&'a self, inp: &'a [I]) -> &'a [U] {
        let buf = &self.first.output(inp)[self.pos..];
        self.second.output(buf)
    }
}

pub trait Inplace<T> {
    fn process(&mut self, in_out: &mut [T]);

    fn then<S>(self, inplace: S) -> InplaceThenInplace<T, Self, S>
        where S: Inplace<T>,
              Self: Sized,
    {
        InplaceThenInplace::new(self, inplace)
    }
}

pub struct ThenInplace<I, O, S1, S2> {
    step: S1,
    inplace: S2,
    buf: Vec<O>,
    _ty: PhantomData<(I, O)>,
}

impl<I, O: Clone, S1, S2> ThenInplace<I, O, S1, S2>
    where S1: Step<I, O>,
          S2: Inplace<O>,
{
    fn new(step: S1, inplace: S2) -> Self {
        Self {
            step,
            inplace,
            buf: Vec::new(),
            _ty: PhantomData,
        }
    }

    fn process_inplace(&mut self, inp: &[I]) {
        self.buf.clear();
        self.buf.extend_from_slice(self.step.output(inp));
        self.inplace.process(&mut self.buf);
    }
}

impl<I, O: Clone, S1, S2> Step<I, O> for ThenInplace<I, O, S1, S2>
    where S1: Step<I, O>,
          S2: Inplace<O>,
{
    fn process(&mut self, inp: &[I]) -> usize {
        let consumed = self.step.process(inp);
        self.process_inplace(inp);
        consumed
    }

    fn finish(&mut self) {
        self.step.finish();
        self.process_inplace(&[]);
    }

    fn output<'a>(&'a self, _inp: &'a [I]) -> &'a [O] {
        &self.buf
    }
}

pub struct InplaceThenInplace<T, S1, S2> {
    first: S1,
    second: S2,
    _ty: PhantomData<T>,
}

impl<T, S1, S2> InplaceThenInplace<T, S1, S2> {
    fn new(first: S1, second: S2) -> Self {
        Self {
            first,
            second,
            _ty: PhantomData,
        }
    }
}

impl<T, S1, S2> Inplace<T> for InplaceThenInplace<T, S1, S2>
    where S1: Inplace<T>,
          S2: Inplace<T>,
{
    fn process(&mut self, in_out: &mut [T]) {
        self.first.process(in_out);
        self.second.process(in_out);
    }
}

#[cfg(test)]
mod test {
    use super::*;


    struct TestStep<T> {
        id: &'static str,
        buf: Vec<T>,
        max_chunk: usize,
        last_process_call: Option<Vec<T>>,
        finished: bool,
    }

    impl<T> TestStep<T> {
        pub fn new(id: &'static str, capacity: usize, max_chunk: usize) -> Self {
            Self {
                id,
                buf: Vec::with_capacity(capacity),
                max_chunk,
                last_process_call: None,
                finished: false,
            }
        }
    }

    impl<T: Clone + std::fmt::Debug + Eq> Step<T, T> for TestStep<T> {
        fn process(&mut self, inp: &[T]) -> usize {
            assert!(!self.finished);

            self.last_process_call = Some(inp.into());

            dbg!((self.id, &self.buf));
            let capacity = self.buf.capacity();

            if self.buf.len() == capacity {
                self.buf.clear();
            }

            let max_chunk = cmp::min(self.max_chunk, inp.len());
            let consumed = cmp::min(max_chunk, capacity - self.buf.len());
            self.buf.extend_from_slice(&inp[..consumed]);

            dbg!((self.id, &self.buf));
            consumed
        }

        fn finish(&mut self) {
            dbg!("finish");
            assert!(!self.finished);
            self.finished = true;
        }

        fn output<'a>(&'a self, inp: &'a [T]) -> &'a [T] {
            if !self.finished {
                assert_eq!(Some(inp.to_vec()), self.last_process_call);
            } else {
                assert!(inp.is_empty());
            }
            dbg!((self.id, &self.buf, self.finished));
            if self.buf.len() == self.buf.capacity() || self.finished {
                &self.buf
            } else {
                &[]
            }
        }
    }

    struct TestInplace;

    impl Inplace<u8> for TestInplace {
        fn process(&mut self, in_out: &mut [u8]) {
            for v in in_out.iter_mut() {
                *v += 1;
            }
        }
    }

    #[test]
    fn then() {
        let mut pl = TestStep::new("first", 3, 100)
            .then(TestStep::new("second", 2, 1));

        dbg!(1);
        assert_eq!(pl.process(&[1, 2, 3, 4]), 0);
        assert_eq!(pl.output(&[1, 2, 3, 4]), &[1, 2]);

        dbg!(2);
        assert_eq!(pl.process(&[1, 2, 3, 4]), 3);
        assert_eq!(pl.output(&[1, 2, 3, 4]), &[]);

        dbg!(3);
        assert_eq!(pl.process(&[4]), 1);
        assert_eq!(pl.output(&[4]), &[]);

        dbg!(4);
        pl.finish();
        assert_eq!(pl.output(&[]), &[3, 4]);
    }

    #[test]
    fn then_inplace() {
        let mut pl = TestStep::new("first", 3, 100)
            .then_inplace(TestInplace);

        assert_eq!(pl.process(&[1, 2, 3, 4]), 3);
        assert_eq!(pl.output(&[1, 2, 3, 4]), &[2, 3, 4]);

        assert_eq!(pl.process(&[4]), 1);
        assert_eq!(pl.output(&[4]), &[]);

        pl.finish();
        assert_eq!(pl.output(&[]), &[5]);
    }

    #[test]
    fn inplace_then_inplace() {
        let mut pl = TestStep::new("first", 3, 100)
            .then_inplace(TestInplace.then(TestInplace))
            .then_inplace(TestInplace);

        assert_eq!(pl.process(&[1, 2, 3, 4]), 3);
        assert_eq!(pl.output(&[1, 2, 3, 4]), &[4, 5, 6]);

        assert_eq!(pl.process(&[4]), 1);
        assert_eq!(pl.output(&[4]), &[]);

        pl.finish();
        assert_eq!(pl.output(&[]), &[7]);
    }
}