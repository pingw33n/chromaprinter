use std::cell::RefCell;
use std::rc::Rc;

use crate::audio::*;
use crate::chroma::*;
use crate::pipeline::Step;
use crate::test::*;
use crate::chroma::Normalize;

#[derive(Clone)]
struct Collector {
    feats: Rc<RefCell<Vec<Vec<f64>>>>,
}

impl Collector {
    pub fn new() -> Self {
        Self {
            feats: Rc::new(RefCell::new(Vec::new())),
        }
    }
}

impl Step<f64, f64> for Collector {
    fn process(&mut self, inp: &[f64]) -> usize {
        if inp.len() > 0 {
            self.feats.borrow_mut().push(inp.into());
        }
        inp.len()
    }

    fn finish(&mut self) {}

    fn output<'a>(&'a self, inp: &'a [f64]) -> &'a [f64] {
        inp
    }
}

#[test]
fn test() {
    const FRAME_LEN: usize = 4096;
    const OVERLAP: usize = FRAME_LEN - FRAME_LEN / 3;
    const SAMPLE_RATE: u32 = 11025;

    let coll = Collector::new();

    let mut pipeline = Downmix::new(2)
        .then(Resample::new(44100, SAMPLE_RATE))
        .then(FFT::new(FRAME_LEN, OVERLAP))
        .then(Chroma::new(28, 3520, 4096, SAMPLE_RATE, false))
        .then_inplace(Normalize::new(0.01))
        .then(coll.clone())
        ;

    let inp = &read_audio_raw(include_bytes!("../../tests/data/test_stereo_44100.raw"))[..];

    let mut inp = inp;
    for i in 0..100 {
        let consumed = pipeline.process(inp);
        let output = pipeline.output(inp);
        dbg!((i, consumed, output.len()));
        if consumed == 0 && output.is_empty() {
            pipeline.finish();
            let output = pipeline.output(&[]);
            dbg!((i, consumed, output.len()));
            break;
        }
        inp = &inp[consumed..];
    }
    assert!(inp.is_empty());
    dbg!(coll.feats.borrow());

    // FIXME
}