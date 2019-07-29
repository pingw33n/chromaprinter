pub mod buf;
pub mod windows;
#[cfg(test)]
pub mod test_utils;

use std::marker::PhantomData;

pub use buf::Buf;
pub use windows::Windows;

pub trait Step<I, O> {
    fn process<F>(&mut self, input: &[I], output: F)
        where F: FnMut(&[O]);

    fn finish<F>(&mut self, output: F)
        where F: FnMut(&[O]);

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
            _ty: PhantomData,
        }
    }
}

impl<I, O, U, S1, S2> Step<I, U> for Then<I, O, U, S1, S2>
    where S1: Step<I, O>,
          S2: Step<O, U>,
{
    fn process<F>(&mut self, input: &[I], mut output: F)
        where F: FnMut(&[U])
    {
        let second = &mut self.second;
        self.first.process(input, |input| second.process(input, &mut output));
    }

    fn finish<F>(&mut self, mut output: F)
        where F: FnMut(&[U])
    {
        let second = &mut self.second;
        self.first.finish(|input| second.process(input, &mut output));
        second.finish(output);
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

    fn process0<F>(inp: &[O], buf: &mut Vec<O>, inplace: &mut S2, mut output: F)
        where F: FnMut(&[O])
    {
        buf.clear();
        buf.extend_from_slice(inp);
        inplace.process(buf);
        output(&buf);
    }
}

impl<I, O: Clone, S1, S2> Step<I, O> for ThenInplace<I, O, S1, S2>
    where S1: Step<I, O>,
          S2: Inplace<O>,
{
    fn process<F>(&mut self, input: &[I], mut output: F)
        where F: FnMut(&[O])
    {
        let buf = &mut self.buf;
        let inplace = &mut self.inplace;
        self.step.process(input, |v| Self::process0(v, buf, inplace, &mut output));
    }

    fn finish<F>(&mut self, mut output: F)
        where F: FnMut(&[O])
    {
        let buf = &mut self.buf;
        let inplace = &mut self.inplace;
        self.step.finish(|v| Self::process0(v, buf, inplace, &mut output));
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
    use std::cmp;

    use test_utils::*;

    struct TestStep<T> {
        id: &'static str,
        buf: Vec<T>,
        finished: bool,
    }

    impl<T> TestStep<T> {
        pub fn new(id: &'static str, capacity: usize) -> Self {
            Self {
                id,
                buf: Vec::with_capacity(capacity),
                finished: false,

            }
        }
    }

    impl<T: Clone + std::fmt::Debug> Step<T, T> for TestStep<T> {
        fn process<F>(&mut self, mut input: &[T], mut output: F)
            where F: FnMut(&[T])
        {
            let cap = self.buf.capacity();

            while input.len() > 0 {
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
            assert!(!self.finished);
            if self.buf.len() > 0 {
                output(&self.buf)
            }
            self.finished = true;
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
        let pl = &mut TestStep::new("step1", 3)
            .then(TestStep::new("step2", 2))
            .then(TestStep::new("step3", 3));

        assert!(process(pl, &[1, 2, 3, 4]).is_empty());
        assert_eq!(&process(pl, &[5, 6]), &[vec![1, 2, 3], vec![4, 5, 6]]);
        assert!(process(pl, &[7, 8, 9, 10]).is_empty());
        assert_eq!(&finish(pl), &[vec![7, 8, 9], vec![10]]);
    }

    #[test]
    fn then_inplace() {
        let pl = &mut TestStep::new("step1", 3)
            .then_inplace(TestInplace);

        assert_eq!(&process(pl, &[1, 2, 3, 4]), &[vec![2, 3, 4]]);
        assert_eq!(&finish(pl), &[vec![5]]);
    }

    #[test]
    fn inplace_then_inplace() {
        let pl = &mut TestStep::new("step1", 3)
            .then_inplace(TestInplace.then(TestInplace))
            .then_inplace(TestInplace);

        assert_eq!(&process(pl, &[1, 2, 3, 4]), &[vec![4, 5, 6]]);
        assert_eq!(&finish(pl), &[vec![7]]);
    }
}