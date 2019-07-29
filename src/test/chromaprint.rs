use crate::audio::*;
use crate::chroma::*;
use crate::pipeline::Step;
use crate::test::*;
use crate::chroma::Normalize;
use crate::pipeline::test_utils::collect;

#[test]
fn test() {
    const FRAME_LEN: usize = 4096;
    const OVERLAP: usize = FRAME_LEN - FRAME_LEN / 3;
    const SAMPLE_RATE: u32 = 11025;

    let actual = &mut Vec::new();

    let mut pipeline = Downmix::new(2)
        .then(Resample::new(44100, SAMPLE_RATE))
        .then(FFT::new(FRAME_LEN, OVERLAP))
        .then(Chroma::new(28, 3520, 4096, SAMPLE_RATE, false))
        .then_inplace(Normalize::new(0.01))
        ;

    let inp = &read_audio_raw(include_bytes!("../../tests/data/test_stereo_44100.raw"))[..];

    pipeline.process(inp, collect(actual));
    pipeline.finish(collect(actual));

    dbg!(actual);

    // FIXME
}