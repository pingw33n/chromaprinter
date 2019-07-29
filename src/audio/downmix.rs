use crate::pipeline::Step;

pub struct Downmix {
    buf: Vec<i16>,
    input_channel_count: u32,
}

impl Downmix {
    pub fn new(input_channel_count: u32) -> Self {
        Self {
            buf: Vec::new(),
            input_channel_count,
        }
    }
}

impl Step<i16, i16> for Downmix {
    fn process<F>(&mut self, input: &[i16], mut output: F)
        where F: FnMut(&[i16])
    {
        assert_eq!(input.len() % self.input_channel_count as usize, 0);

        if self.input_channel_count == 1 {
            output(input);
            return;
        }

        self.buf.clear();

        match self.input_channel_count {
            2 => {
                self.buf.reserve(input.len() / 2);
                for chunk in input.chunks(2) {
                    self.buf.push(((chunk[0] as i32 + chunk[1] as i32) / 2) as i16);
                }
            }
            n => {
                let n = n as usize;
                self.buf.reserve(input.len() / n);
                for chunk in input.chunks(n) {
                    let sum = chunk.iter().fold(0i32, |sum, &v| sum + v as i32);
                    self.buf.push((sum / self.input_channel_count as i32) as i16);
                }
            },
        }
        output(&self.buf);
    }

    fn finish<F>(&mut self, _output: F)
        where F: FnMut(&[i16])
    {
    }
}