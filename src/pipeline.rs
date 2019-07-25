pub mod buf;
pub mod windows;

use std::marker::PhantomData;
use std::cmp;

pub use buf::Buf;
pub use windows::Windows;

// empty input sequence is special and shouldn't be treated as a value
// finish=true called once after previously calling process(finish=false) until idle
pub trait Step<I, O> {
    fn process(&mut self, inp: &[I], finish: bool) -> usize;

    fn output<'a>(&'a self, inp: &'a [I], finish: bool) -> &'a [O];

    fn then<S, U>(self, step: S) -> Then<I, O, U, Self, S>
        where S: Step<O, U>,
              Self: Sized,
    {
        Then::new(self, step)
    }

    fn then_inplace<S>(self, step: S) -> ThenInplace<I, O, Self, S>
        where S: Inplace<O>,
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
    _ty: PhantomData<(I, O, U)>,
}

impl<I, O, U, S1, S2> Then<I, O, U, S1, S2> {
    fn new(first: S1, second: S2) -> Self {
        Self {
            first,
            second,
            consumed: 0,
            pos: 0,
            next_pos: 0,
            _ty: PhantomData,
        }
    }
}

impl<I, O, U, S1, S2> Step<I, U> for Then<I, O, U, S1, S2>
    where S1: Step<I, O>,
          S2: Step<O, U>,
{
    fn process(&mut self, inp: &[I], finish: bool) -> usize {
        self.pos = self.next_pos;
        if self.pos == 0 || self.pos == self.consumed {
            self.consumed = self.first.process(inp, finish);
        }
        let first_out = self.first.output(inp, finish);
        loop {
            let second_inp = &first_out[self.pos..];
            let second_cons = self.second.process(second_inp, finish);

            self.next_pos = self.pos + second_cons;
            assert!(self.next_pos <= first_out.len());

            // If the second step consumed all we're done.
            if self.next_pos == first_out.len() {
                self.next_pos = 0;
                break self.consumed;
            }

            // If the second step consumed nothing we're done.
            if second_cons == 0 {
                assert!(self.second.output(second_inp, finish).len() > 0);
                break 0;
            }
            // If the second step produced some output we're done.
            if self.second.output(second_inp, finish).len() > 0 {
                break 0;
            }

            // Otherwise continue pushing data to the second step.
            self.pos = self.next_pos;
        }
    }

    fn output<'a>(&'a self, inp: &'a [I], finish: bool) -> &'a [U] {
        let buf = &self.first.output(inp, finish)[self.pos..];
        self.second.output(buf, finish)
    }
}

pub struct PassThrough<T>(PhantomData<T>);

impl<T> Step<T, T> for PassThrough<T> {
    fn process(&mut self, inp: &[T], _finish: bool) -> usize {
        inp.len()
    }

    fn output<'a>(&'a self, inp: &'a [T], _finish: bool) -> &'a [T] {
        inp
    }
}

pub fn pass_through<T>() -> PassThrough<T> {
    PassThrough(PhantomData)
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

impl<I, O, S1, S2> ThenInplace<I, O, S1, S2> {
    fn new(step: S1, inplace: S2) -> Self {
        Self {
            step,
            inplace,
            buf: Vec::new(),
            _ty: PhantomData,
        }
    }
}

impl<I, O: Clone, S1, S2> Step<I, O> for ThenInplace<I, O, S1, S2>
    where S1: Step<I, O>,
          S2: Inplace<O>,
{
    fn process(&mut self, inp: &[I], finish: bool) -> usize {
        let consumed = self.step.process(inp, finish);
        self.buf.clear();
        self.buf.extend_from_slice(self.step.output(inp, finish));
        self.inplace.process(&mut self.buf);
        consumed
    }

    fn output<'a>(&'a self, _inp: &'a [I], _finish: bool) -> &'a [O] {
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
        last_process_call: Option<(Vec<T>, bool)>,
    }

    impl<T> TestStep<T> {
        pub fn new(id: &'static str, capacity: usize, max_chunk: usize) -> Self {
            Self {
                id,
                buf: Vec::with_capacity(capacity),
                max_chunk,
                last_process_call: None,
            }
        }
    }

    impl<T: Clone + std::fmt::Debug + Eq> Step<T, T> for TestStep<T> {
        fn process(&mut self, inp: &[T], finish: bool) -> usize {
            self.last_process_call = Some((inp.into(), finish));

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

        fn output<'a>(&'a self, inp: &'a [T], finish: bool) -> &'a [T] {
            assert_eq!(Some((inp.to_vec(), finish)), self.last_process_call);
            dbg!((self.id, &self.buf));
            if self.buf.len() == self.buf.capacity() || finish {
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
        assert_eq!(pl.process(&[1, 2, 3, 4], false), 0);
        assert_eq!(pl.output(&[1, 2, 3, 4], false), &[1, 2]);

        dbg!(2);
        assert_eq!(pl.process(&[1, 2, 3, 4], false), 3);
        assert_eq!(pl.output(&[1, 2, 3, 4], false), &[]);

        dbg!(3);
        assert_eq!(pl.process(&[4], false), 1);
        assert_eq!(pl.output(&[4], false), &[]);

        dbg!(4);
        assert_eq!(pl.process(&[], true), 0);
        assert_eq!(pl.output(&[], true), &[3, 4]);
    }

    #[test]
    fn then_inplace() {
        let mut pl = TestStep::new("first", 3, 100)
            .then_inplace(TestInplace);

        assert_eq!(pl.process(&[1, 2, 3, 4], false), 3);
        assert_eq!(pl.output(&[1, 2, 3, 4], false), &[2, 3, 4]);

        assert_eq!(pl.process(&[4], false), 1);
        assert_eq!(pl.output(&[4], false), &[]);

        assert_eq!(pl.process(&[], true), 0);
        assert_eq!(pl.output(&[], true), &[5]);
    }

    #[test]
    fn inplace_then_inplace() {
        let mut pl = TestStep::new("first", 3, 100)
            .then_inplace(TestInplace.then(TestInplace))
            .then_inplace(TestInplace);

        assert_eq!(pl.process(&[1, 2, 3, 4], false), 3);
        assert_eq!(pl.output(&[1, 2, 3, 4], false), &[4, 5, 6]);

        assert_eq!(pl.process(&[4], false), 1);
        assert_eq!(pl.output(&[4], false), &[]);

        assert_eq!(pl.process(&[], true), 0);
        assert_eq!(pl.output(&[], true), &[7]);
    }
}