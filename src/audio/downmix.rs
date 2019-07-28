use crate::pipeline::Step;

pub struct Downmix {
    buf: Vec<i16>,
    src_channel_count: u32,
}

impl Downmix {
    pub fn new(src_channel_count: u32) -> Self {
        Self {
            buf: Vec::new(),
            src_channel_count,
        }
    }
}

impl Step<i16, i16> for Downmix {
    fn process(&mut self, inp: &[i16]) -> usize {
        assert_eq!(inp.len() % self.src_channel_count as usize, 0);

        self.buf.clear();

        match self.src_channel_count {
            1 => {}
            2 => {
                self.buf.reserve(inp.len() / 2);
                for chunk in inp.chunks(2) {
                    self.buf.push(((chunk[0] as i32 + chunk[1] as i32) / 2) as i16);
                }
            }
            n => {
                let n = n as usize;
                self.buf.reserve(inp.len() / n);
                for chunk in inp.chunks(n) {
                    let sum = chunk.iter().fold(0i32, |sum, &v| sum + v as i32);
                    self.buf.push((sum / self.src_channel_count as i32) as i16);
                }
            },
        }
        inp.len()
    }

    fn finish(&mut self) {}

    fn output<'a>(&'a self, inp: &'a [i16]) -> &'a [i16] {
        if self.src_channel_count == 1 {
            inp
        } else {
            &self.buf
        }
    }
}