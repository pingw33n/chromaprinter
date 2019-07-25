use super::*;

pub struct Encoder {
    normal_bits: Vec<u8>,
    exceptional_bits: Vec<u8>,
}

impl Encoder {
    pub fn new() -> Self {
        Self{
            normal_bits: Vec::new(),
            exceptional_bits: Vec::new(),
        }
    }

    pub fn encode(&mut self, inp: &[u32], version: u8, out: &mut Vec<u8>) {
        self.normal_bits.clear();
        self.exceptional_bits.clear();

        let len = inp.len();

        if len > 0 {
            self.normal_bits.reserve(len);
            self.exceptional_bits.reserve(len / 10);
            self.process_subfingerprint(inp[0]);
            for i in 1..len {
                self.process_subfingerprint(inp[i] ^ inp[i - 1]);
            }
        }

        out.reserve_exact(4 + packed_int3_len(self.normal_bits.len()) +
            packed_int5_len(self.exceptional_bits.len()));

        out.push(version);
        out.push((len >> 16) as u8);
        out.push((len >>  8) as u8);
        out.push((len      ) as u8);

        pack_int3(&self.normal_bits, |v| out.push(v));
        pack_int5(&self.exceptional_bits, |v| out.push(v));
    }

    fn process_subfingerprint(&mut self, mut x: u32) {
        let mut bit = 1;
        let mut last_bit = 0;
        while x != 0 {
            if x & 1 != 0 {
                let value = bit - last_bit;
                if value >= MAX_NORMAL_VALUE {
                    self.normal_bits.push(MAX_NORMAL_VALUE);
                    self.exceptional_bits.push(value - MAX_NORMAL_VALUE);
                } else {
                    self.normal_bits.push(value);
                }
                last_bit = bit;
            }
            x >>= 1;
            bit += 1;
        }
        self.normal_bits.push(0);
    }
}

fn packed_int3_len(len: usize) -> usize {
    (len * 3 + 7) / 8
}

// TODO generate with macro
fn pack_int3(mut inp: &[u8], mut out: impl FnMut(u8)) {
    while inp.len() >= 8 {
        let s = &inp[..8];
        out((s[0] & 0x07) | ((s[1] & 0x07) << 3) | ((s[2] & 0x03) << 6));
        out(((s[2] & 0x04) >> 2) | ((s[3] & 0x07) << 1) | ((s[4] & 0x07) << 4) | ((s[5] & 0x01) << 7));
        out(((s[5] & 0x06) >> 1) | ((s[6] & 0x07) << 2) | ((s[7] & 0x07) << 5));
        inp = &inp[8..];
    }
    match inp.len() {
        7 => {
            let s = &inp[..7];
            out((s[0] & 0x07) | ((s[1] & 0x07) << 3) | ((s[2] & 0x03) << 6));
            out(((s[2] & 0x04) >> 2) | ((s[3] & 0x07) << 1) | ((s[4] & 0x07) << 4) | ((s[5] & 0x01) << 7));
            out(((s[5] & 0x06) >> 1) | ((s[6] & 0x07) << 2));
        }
        6 => {
            let s = &inp[..6];
            out((s[0] & 0x07) | ((s[1] & 0x07) << 3) | ((s[2] & 0x03) << 6));
            out(((s[2] & 0x04) >> 2) | ((s[3] & 0x07) << 1) | ((s[4] & 0x07) << 4) | ((s[5] & 0x01) << 7));
            out((s[5] & 0x06) >> 1);
        }
        5 => {
            let s = &inp[..5];
            out((s[0] & 0x07) | ((s[1] & 0x07) << 3) | ((s[2] & 0x03) << 6));
            out(((s[2] & 0x04) >> 2) | ((s[3] & 0x07) << 1) | ((s[4] & 0x07) << 4));
        }
        4 => {
            let s = &inp[..4];
            out((s[0] & 0x07) | ((s[1] & 0x07) << 3) | ((s[2] & 0x03) << 6));
            out(((s[2] & 0x04) >> 2) | ((s[3] & 0x07) << 1));
        }
        3 => {
            let s = &inp[..3];
            out((s[0] & 0x07) | ((s[1] & 0x07) << 3) | ((s[2] & 0x03) << 6));
            out((s[2] & 0x04) >> 2);
        }
        2 => {
            let s = &inp[..2];
            out((s[0] & 0x07) | ((s[1] & 0x07) << 3));
        }
        1 => {
            let s = &inp[..1];
            out(s[0] & 0x07);
        }
        0 => {}
        _ => unreachable!(),
    }
}

fn packed_int5_len(len: usize) -> usize {
    (len * 5 + 7) / 8
}

// TODO generate with macro
fn pack_int5(mut inp: &[u8], mut out: impl FnMut(u8)) {
    while inp.len() >= 8 {
        let s = &inp[..8];
        out((s[0] & 0x1f) | ((s[1] & 0x07) << 5));
        out(((s[1] & 0x18) >> 3) | ((s[2] & 0x1f) << 2) | ((s[3] & 0x01) << 7));
        out(((s[3] & 0x1e) >> 1) | ((s[4] & 0x0f) << 4));
        out(((s[4] & 0x10) >> 4) | ((s[5] & 0x1f) << 1) | ((s[6] & 0x03) << 6));
        out(((s[6] & 0x1c) >> 2) | ((s[7] & 0x1f) << 3));
        inp = &inp[8..];
    }
    match inp.len() {
        7 => {
            let s = &inp[..7];
            out((s[0] & 0x1f) | ((s[1] & 0x07) << 5));
            out(((s[1] & 0x18) >> 3) | ((s[2] & 0x1f) << 2) | ((s[3] & 0x01) << 7));
            out(((s[3] & 0x1e) >> 1) | ((s[4] & 0x0f) << 4));
            out(((s[4] & 0x10) >> 4) | ((s[5] & 0x1f) << 1) | ((s[6] & 0x03) << 6));
            out((s[6] & 0x1c) >> 2);
        }
        6 => {
            let s = &inp[..6];
            out((s[0] & 0x1f) | ((s[1] & 0x07) << 5));
            out(((s[1] & 0x18) >> 3) | ((s[2] & 0x1f) << 2) | ((s[3] & 0x01) << 7));
            out(((s[3] & 0x1e) >> 1) | ((s[4] & 0x0f) << 4));
            out(((s[4] & 0x10) >> 4) | ((s[5] & 0x1f) << 1));
        }
        5 => {
            let s = &inp[..5];
            out((s[0] & 0x1f) | ((s[1] & 0x07) << 5));
            out(((s[1] & 0x18) >> 3) | ((s[2] & 0x1f) << 2) | ((s[3] & 0x01) << 7));
            out(((s[3] & 0x1e) >> 1) | ((s[4] & 0x0f) << 4));
            out((s[4] & 0x10) >> 4);
        }
        4 => {
            let s = &inp[..4];
            out((s[0] & 0x1f) | ((s[1] & 0x07) << 5));
            out(((s[1] & 0x18) >> 3) | ((s[2] & 0x1f) << 2) | ((s[3] & 0x01) << 7));
            out((s[3] & 0x1e) >> 1);
        }
        3 => {
            let s = &inp[..3];
            out((s[0] & 0x1f) | ((s[1] & 0x07) << 5));
            out(((s[1] & 0x18) >> 3) | ((s[2] & 0x1f) << 2));
        }
        2 => {
            let s = &inp[..2];
            out((s[0] & 0x1f) | ((s[1] & 0x07) << 5));
            out((s[1] & 0x18) >> 3);
        }
        1 => {
            let s = &inp[..1];
            out(s[0] & 0x1f);
        }
        0 => {}
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test() {
        // input, expected
        let data = &[
            // OneItemOneBit
            (&[1][..],
            &[0, 0, 0, 1, 1][..]),

            // OneItemThreeBits
            (&[7][..],
            &[0, 0, 0, 1, 73, 0][..]),

            // OneItemOneBitExcept
            (&[1 << 6][..],
            &[0, 0, 0, 1, 7, 0][..]),

            // OneItemOneBitExcept2
            (&[1 << 8][..],
            &[0, 0, 0, 1, 7, 2][..]),

            // TwoItems
            (&[1, 0][..],
            &[0, 0, 0, 2, 65, 0][..]),

            // TwoItemsNoChange
            (&[1, 1][..],
            &[0, 0, 0, 2, 1, 0][..]),
        ];
        let mut c = Encoder::new();
        let act = &mut Vec::new();

        for (inp, exp) in data {
            const VERSION: u8 = 123;

            act.clear();
            c.encode(inp, VERSION, act);

            let mut exp = exp.to_vec();
            exp[0] = VERSION;

            assert_eq!(act, &exp);
        }
    }
}