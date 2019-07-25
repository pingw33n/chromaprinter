use samplerate::{Samplerate, ConverterType};

use crate::pipeline::Step;

pub struct Resample {
    buf: Vec<f32>,
    out: Vec<i16>,
    imp: Option<Samplerate>,
}

impl Resample {
    pub fn new(src_rate: u32, dst_rate: u32) -> Self {
        Self {
            buf: Vec::new(),
            out: Vec::new(),
            imp: if src_rate != dst_rate {
                Some(Samplerate::new(ConverterType::SincFastest, src_rate, dst_rate, 1).unwrap())
            } else {
                None
            }
        }
    }
}

impl Step<i16, i16> for Resample {
    fn process(&mut self, inp: &[i16], _finish: bool) -> usize {
        if let Some(imp) = &self.imp {
            // FIXME this is quite ineffective
            self.buf.clear();
            self.buf.reserve(inp.len());
            for &v in inp.iter() {
                self.buf.push((v as f64 / 32768.0) as f32);
            }

            let res = imp.process(&self.buf).unwrap();

            self.out.clear();
            self.out.reserve(res.len());
            for &v in res.iter() {
                self.out.push((v as f64 * 32768.0) as i16);
            }
        }

        inp.len()
    }

    fn output<'a>(&'a self, inp: &'a [i16], _finish: bool) -> &'a [i16] {
        if !self.out.is_empty() {
            &self.out
        } else {
            inp
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::read_audio_raw;

    #[test]
    fn test() {
        let inp = &read_audio_raw(include_bytes!("../../tests/data/test_mono_44100.raw"));
        let exp = &read_audio_raw(include_bytes!("../../tests/data/test_mono_11025.raw"))[..];

        let mut r = Resample::new(44100, 11025);
        assert_eq!(r.process(inp, false), inp.len());
        for (&a, &e) in r.output(inp, false)[..1000].iter().zip(exp[..1000].iter()) {
            assert!((a as i32 - e as i32).abs() <= 20, "{} {}", a, e);
        }
    }
}