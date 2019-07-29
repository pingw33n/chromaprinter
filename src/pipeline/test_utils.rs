use super::*;

pub fn collect<'a, T: Clone>(out: &'a mut Vec<Vec<T>>) -> impl 'a + FnMut(&[T]) {
    move |v: &[T]| out.push(v.to_vec())
}

pub fn collect_flat<'a, T: Clone>(out: &'a mut Vec<T>) -> impl 'a + FnMut(&[T]) {
    move |v: &[T]| out.extend_from_slice(v)
}

pub fn process<I, O: Clone, S: Step<I, O>>(step: &mut S, input: &[I]) -> Vec<Vec<O>> {
    let mut out = Vec::new();
    step.process(input, collect(&mut out));
    out
}

pub fn process_flat<I, O: Clone, S: Step<I, O>>(step: &mut S, input: &[I]) -> Vec<O> {
    let mut out = Vec::new();
    step.process(input, collect_flat(&mut out));
    out
}

pub fn finish<I, O: Clone, S: Step<I, O>>(step: &mut S) -> Vec<Vec<O>> {
    let mut out = Vec::new();
    step.finish(collect(&mut out));
    out
}

pub fn finish_flat<I, O: Clone, S: Step<I, O>>(step: &mut S) -> Vec<O> {
    let mut out = Vec::new();
    step.finish(collect_flat(&mut out));
    out
}

pub fn process_all_flat<I, O: Clone, S: Step<I, O>>(step: &mut S, input: &[I]) -> Vec<O> {
    let mut out = Vec::new();
    step.process(input, collect_flat(&mut out));
    step.finish(collect_flat(&mut out));
    out
}