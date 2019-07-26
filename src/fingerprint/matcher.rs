mod gaussian_filter;
mod gradient;

use if_chain::if_chain;
use rand::prelude::*;
use std::cmp;

use crate::util::hamming_distance;
use gradient::gradient;
use gaussian_filter::GaussianFilter;

const ALIGN_BITS: u32 = 12;
const ALIGN_MASK: u32 = (1 << ALIGN_BITS) - 1;
const HASH_SHIFT: u32 = 32 - ALIGN_BITS;
const HASH_MASK: u32 = ((1 << ALIGN_BITS) - 1) << HASH_SHIFT;
const OFFSET_MASK: u32 = (1 << (32 - ALIGN_BITS - 1)) - 1;
const SOURCE_MASK: u32 = 1 << (32 - ALIGN_BITS - 1);

type Elem = f32;

pub struct Matcher {
    threshold: f64,
    offsets: Vec<u32>,
    histogram: Vec<u32>,
    segments: Vec<Segment>,
}

impl Matcher {
    pub fn with_threshold(threshold: f64) -> Self {
        Self {
            threshold,
            offsets: Vec::new(),
            histogram: Vec::new(),
            segments: Vec::new(),
        }
    }

    pub fn new() -> Self {
        Self::with_threshold(10.0)
    }

    pub fn matches(&mut self, fp1: &[u32], fp2: &[u32]) -> bool {
        if fp1.len() + 1 >= OFFSET_MASK as usize {
            return false;
        }
        if fp2.len() + 1 >= OFFSET_MASK as usize {
            return false;
        }

        self.build_offsets(fp1, fp2);
        self.build_histogram(fp1, fp2);
        let item = self.find_best_alignment();

        // TODO Move to fields so it doesn't reallocate.
        let mut bit_counts = Vec::new();
        let mut bit_counts_gradient = Vec::new();
        let mut gradient_peaks = Vec::new();

        let mut gaussian_filter = GaussianFilter::new(8.0, 3);

        self.segments.clear();
        let offset_diff = item.1 as isize - fp2.len() as isize;
        let offset1 = cmp::max(offset_diff, 0) as usize;
        let offset2 = -cmp::min(offset_diff, 0) as usize;

        let len = cmp::min(fp1.len() - offset1, fp2.len() - offset2);
        bit_counts.clear();
        bit_counts.reserve(len);

        for (&v1, &v2) in fp1[..offset1].iter().zip(fp2[..offset2].iter()) {
            bit_counts.push(hamming_distance(v1, v2) as Elem +
                rand::thread_rng().gen_range(0.0 as Elem, 0.001));
        }

        let smoothed_bit_counts = gaussian_filter.apply(&bit_counts);

        bit_counts_gradient.clear();
        bit_counts_gradient.reserve(smoothed_bit_counts.len());
        gradient(&smoothed_bit_counts, |v| bit_counts_gradient.push(v));
        for v in bit_counts_gradient.iter_mut() {
            *v = v.abs();
        }

        for i in 0..bit_counts_gradient.len() {
            let gi = bit_counts_gradient[i];
            if i > 0 && i < len - 1 && gi > 0.15 &&
                gi >= bit_counts_gradient[i - 1] && gi >= bit_counts_gradient[i + 1]
            {
                if gradient_peaks.last().map(|v| v + 1 < i).unwrap_or(true) {
                    gradient_peaks.push(i);
                }
            }
        }
        gradient_peaks.push(len);

        let mut begin = 0;
        for &end in &gradient_peaks {
            let duration = end - begin;
            let score = bit_counts[begin..end].iter().sum::<Elem>() / duration as Elem;
            if (score as f64) < self.threshold {
                let new_seg = Segment {
                    pos1: offset1 + begin,
                    pos2: offset2 + begin,
                    duration,
                    score,
                };
                if_chain! {
                    if let Some(seg) = self.segments.last_mut();
                    if (seg.score - score).abs() < 0.7;
                    then {
                        *seg = seg.merge(&new_seg);
                    } else {
                        self.segments.push(new_seg);
                    }
                }
            }
            begin = end;
        }

        return true;
    }

    fn build_offsets(&mut self, fp1: &[u32], fp2: &[u32]) {
        self.offsets.clear();
        self.offsets.reserve(fp1.len() + fp2.len());

        for (i, &v) in fp1.iter().enumerate()  {
            self.offsets.push((align_strip(v) << HASH_SHIFT) | (i as u32 & OFFSET_MASK));
        }
        for (i, &v) in fp2.iter().enumerate()  {
            self.offsets.push((align_strip(v) << HASH_SHIFT) | (i as u32 & OFFSET_MASK) | SOURCE_MASK);
        }
        self.offsets.sort();
    }

    fn build_histogram(&mut self, fp1: &[u32], fp2: &[u32]) {
        self.histogram.clear();
        self.histogram.resize(fp1.len() + fp2.len(), 0);
        for i in 0..self.offsets.len() {
            let v = self.offsets[i];
            let hash = v & HASH_MASK;
            let offset1 = v & OFFSET_MASK;
            let source1 = v & SOURCE_MASK;
            if source1 != 0 {
                // if we got hash from fp2, it means there is no hash from fp1,
                // because if there was, it would be first
                continue;
            }

            for j in i + 1..self.offsets.len() {
                let v = self.offsets[j];
                let hash2 = v & HASH_MASK;
                if hash != hash2 {
                    break;
                }
                let offset2 = v & OFFSET_MASK;
                let source2 = v & SOURCE_MASK;
                if source2 != 0 {
                    let offset_diff = offset1 as usize + fp2.len() - offset2 as usize;
                    self.histogram[offset_diff] += 1;
                }
            }
        }
    }

    fn find_best_alignment(&mut self) -> (u32, usize) {
        assert!(self.histogram.len() > 0);
        let mut min = (u32::max_value(), usize::max_value());
        for i in 0..self.histogram.len() {
            let count = self.histogram[i];
            if count > 1 {
                let is_peak_left = i > 0 && self.histogram[i - 1] <= count || i == 0;
                let is_peak_right = i < self.histogram.len() - 1 && self.histogram[i + 1] <= count ||
                    i < self.histogram.len() - 1;
                if is_peak_left && is_peak_right && (count, i) < min {
                    min = (count, i);
                }
            }
        }
        min
    }
}

struct Segment {
    pos1: usize,
    pos2: usize,
    duration: usize,
    score: Elem,
}

impl Segment {
    pub fn public_score(&self) -> u32 {
        (self.score * 100.0 + 0.5) as u32
    }

    pub fn merge(&self, o: &Self) -> Self {
        assert_eq!(self.pos1 + self.duration, o.pos1);
        assert_eq!(self.pos2 + self.duration, o.pos2);
        let duration = self.duration + o.duration;
        let score = (self.score * self.duration as Elem + o.score * o.duration as Elem)
            / duration as Elem;
        Self {
            pos1: o.pos1,
            pos2: o.pos2,
            duration,
            score,
        }
    }
}

fn align_strip(v: u32) -> u32 {
    v >> (32 - ALIGN_BITS)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test() {
        let fp1: &[u32] = &[
            0x70a6be7c, 0xf0a6ba3c, 0xf1a6ae3c, 0xf1e68e3c, 0xb3fe962f, 0xb37e962e, 0xb32e862e,
            0xb32dc62a, 0xf22cce3a, 0xf26c7f1a, 0xf26f291a, 0xe66e381a, 0x6665183a, 0x6e241c2a,
            0x6e27343a, 0x6e36300a, 0x3a56200a, 0x3b46200a, 0x3bc66018, 0x39c56138, 0x39c4e378,
            0x78c4f278, 0x78c596f8, 0x78c79ed8, 0x788faad8, 0xe88daad8, 0xe88cbaf8, 0xe88cbee8,
            0xa8ccaea9, 0xa84896a9, 0xa80886a9, 0xa809cea9, 0xac0bde9b, 0xac4f6e9b, 0xac4f26ba,
            0xaccd17aa, 0xb4dc03aa, 0xf4f401ae, 0xf46110be, 0xf06220b6, 0xf06230f6, 0xf062c476,
            0xf0e3cc06, 0xf1f1ac06, 0xf1f3a805, 0xf372ad04, 0xf3528e44, 0xf3128644, 0xf3128634,
            0xb331b224, 0xb334f22c, 0xb364c22d, 0xb1ecc32d, 0xb1ecd52f, 0xa1efd53e, 0xb3eef51e,
            0xb36fed5e, 0xb36d7fee, 0xb36d47ef, 0xb36d47fd, 0xb36f575d, 0xb367510c, 0xb375511c,
            0xb774712d, 0xb5f4612d, 0xb5b7613f, 0xb5b6611e, 0xbdb7631e, 0xbd55773e, 0xbd5d452a,
            0xbf1d442a, 0xbf59447a, 0xbe5bf4db, 0xfed954cb, 0xfed8548b, 0xfe586c8b, 0xff192d9a,
            0xfd1a2dba, 0xfc1a2faa, 0xdc5e07ba, 0xdcd6039a, 0xd8d6019e, 0xd89531de, 0xd890205e,
            0xd990201e, 0xcb91e01e, 0xdb95a41e, 0xdb95ac3e, 0xdbd4bc3f, 0x9b5dbd1c, 0xb95fbf0c,
            0xb9dfbe0c, 0xbdd9865c, 0xad58867c, 0xad1996fc, 0xa71992e8, 0xa71982e8, 0xa639c2e8,
            0xa32802e8, 0xa12c03ef, 0xa02c016e, 0xa06c007e, 0xe0ed143e, 0xe0af3c3e, 0xf0af286e,
            0xf0ae38de, 0xf0ae38ce, 0x51ae6c8e, 0x53aefc8e, 0x52efc58e, 0x56ed819e, 0x566cd1fe,
            0x5e6cf12e, 0x5e7f712e, 0x5e3f223f, 0x5f7f2239, 0x5d7f2239, 0x5c7f3238, 0x5cdf1228,
            0x58dc3169, 0x589c21f9, 0x588c64cb, 0x588cb4cb, 0x18ccac5b, 0x18cdac1b, 0x18ceac0b,
            0x1bdeac0f, 0x1a5e8c0f, 0x1a3e840f, 0x1e3f801d, 0x1e3a803f, 0x1e3ac07f, 0x1e1ae06f,
            0x1e1b60ee, 0x3e5b60ee, 0x3e5a60ff, 0x3f5a00cf, 0x3c7a01cd, 0x3c7b03dc, 0x1c7932fc,
            0x14d926fc, 0x34d92668, 0x34d93678, 0x14d81e59, 0x10981e5b, 0x10d81eda, 0x11dc17ea,
            0x11fc35aa, 0x10fc34aa, 0x107c509a, 0x107c408a, 0x107cc08a, 0x14ffd0ba, 0x14feb4fa,
            0x14feb4de, 0x55fea54e, 0x57fea54f, 0x46ffb51f, 0x46fdb13d, 0x56fdb33c, 0x5e7fb33c,
            0x5e3fa13c, 0x5e1fe12c, 0x5e1de56c, 0x5e1de4ec, 0x5e4ff4fc, 0x5fced0dd, 0x5ddac1cf,
            0x5cdac1de, 0x5cfbc1fe, 0x5c7bd1fe, 0x5c79747e, 0x5c3d6476, 0x5c3d64f6, 0x5c3565d6,
            0x5c5766d6, 0x5cd3f2f6, 0xdc93d2f6, 0xdd8092d6, 0xdd80b676, 0xd780a766, 0x97c0af67,
            0xb7c7afdc, 0xb5d69edc, 0xb15786bc, 0xb15d86ac, 0xb33d82ac, 0xb33dc2bc, 0xb33dd2bc,
            0xb33cf3ec, 0xb33df5ed, 0xb36d546f, 0xa16f747f, 0xa1efe41f, 0xa0efe51f, 0xa1eec70f,
            0xa1eec60f, 0xa16fc61f, 0xa36df737, 0xa36ce536, 0xa368e016, 0xa32cd037, 0xa12f4037,
            0xa52e0156, 0xe57e325e, 0xe5fe223e, 0xe5fd222e, 0xe7fc332e, 0xe7fc111e, 0xe6fc300a,
            0xe7fd741b, 0xe7fdd429, 0xe5fffc19, 0xe4feec09, 0xe47ffd09, 0x647cc659, 0x643cc2eb,
            0x643c02fb, 0x643f125b, 0xf47e127b, 0xf4fe316b][..];
        let fp2: &[u32]  = &[
            0xb32e966e, 0xb32ec62a, 0xb32dc62a, 0xf22c4f3a, 0xf27d691a, 0xe26f281a, 0x666f381a,
            0x66241c3a, 0x6e25343a, 0x6e26301a, 0x3e76200a, 0x3a56200a, 0x3bc6200b, 0x39c76038,
            0x39c4e138, 0x78c4f278, 0x78c49678, 0x78c79ef8, 0x788fbed8, 0xf88faad8, 0xe88daad8,
            0xe88cbef8, 0xa88cbea8, 0xa84cbea9, 0xa80886a9, 0xa80886a9, 0xa809ceb9, 0xac4f6e9b,
            0xac4f6e9b, 0xaccd26ba, 0xa4dc13aa, 0xb4d401aa, 0xf47401be, 0xf463309e, 0xf06220f6,
            0xf0625076, 0xf0e2c456, 0xf0f3cc06, 0xf1f1a807, 0xf1f2a805, 0xf372bf04, 0xf3528e44,
            0xf3128644, 0xf3139634, 0xb330a224, 0xb374f22c, 0xb1e4c32d, 0xb1ecc12f, 0xb1edd52e,
            0xb1efd51e, 0xb36ffd1e, 0xb36fed7e, 0xb36d5fef, 0xb36d47ef, 0xb36f47dd, 0xb36f534d,
            0xb377510c, 0xb374513d, 0xb5f4712d, 0xb5b5613f, 0xb5b7611e, 0xb5b6611e, 0xbdf7631e,
            0xbd5d573e, 0xbd5d452a, 0xbf59446a, 0xbe59d4fa, 0xfed974db, 0xfed8548b, 0xfe58748b,
            0xfe582c8b, 0xff1b2d9a, 0xfd1a2fba, 0xfc5e3faa, 0xdc5e07ba, 0xd8d6039e, 0xd89711de,
            0xd89131de, 0xd990205e, 0xd991601e, 0xdb91e01e, 0xdb95a416, 0xdbd5a43f, 0xdbd4bc3d,
            0xdb5db51c, 0xb95fb60c, 0xbddf960c, 0xbcd98e5c, 0xac589efc, 0xad19b6fc, 0xa71996ac,
            0xa63bc2a8, 0xa63b42ac, 0xa22802af, 0xa12803ee, 0xa02c002e, 0xe06c002e, 0xe0ec342e,
            0xf0ad282e, 0xf0af287e, 0x70ae38ce, 0x73ae788e, 0x52aedc8e, 0x52ee858e, 0x526d818e,
            0x566c90be, 0x566df0ee, 0x5e2df02e, 0x5e2f212f, 0x5f2e222d, 0x5d6f2239, 0x5c7f3239,
            0x5cff1229, 0x5cfd122b, 0x589c232b, 0x589c255a, 0x588ce44a, 0x588cac4a, 0x188cac5a,
            0x188eac4e, 0x19cebc0e, 0x1bde9c0f, 0x1a5e840d, 0x1a3f841d, 0x1e3f8034, 0x1e3ac034,
            0x1a3ac065, 0x1a1a60e5, 0x1a5b60e7, 0x1a5b60e7, 0x1ada30c7, 0x1ffa00c5, 0x1c7b01d5,
            0x1c7912f4, 0x1cf9227c, 0x1cd9266c, 0x1cd9266c, 0x1c993e78, 0x14983e49, 0x14982ecb,
            0x15983ffa, 0x11d815aa, 0x11dc15aa, 0x105c00ba, 0x7c008a, 0x7c408a, 0xfcd09a,
            0xfef0ba, 0xbeb4fa, 0x4bea41e, 0x44fea50e, 0x45ffa51f, 0x47fda53d, 0x46fdb11c,
            0x46ff911c, 0x467e911c, 0x561eb13c, 0x5e1ea56c, 0x5e1fe5ec, 0x5e1de4ec, 0x5e5fe4bc,
            0x5f5ed59d, 0x1fdac18f, 0x1fdbc39f, 0x5d7bc1de, 0x5d79557e, 0x5d7d742e, 0x5c3d642e,
            0x5c356426, 0x5c776156, 0x5c766256, 0x5c97d256, 0xdd95c256, 0xdd809256, 0xdf80a677,
            0x9784af75, 0x97c5afd4, 0x95c7bed4, 0x95df8eb4, 0xb0dd86ac, 0xb15d82ac, 0xb17c82ac,
            0xb33cc2ac, 0xb33cf2ad, 0xb32df3ed, 0xb32df1ef, 0xb12dd5e7, 0xa06df477, 0xa0efec57,
            0xa0eefd17, 0xa0eecd07, 0xa0eecf07, 0xa16fd737, 0xa36de537, 0xa229e117, 0xa229f007,
            0xa22b4037, 0xa32b0167, 0xa76a1377, 0xe5fe2276, 0xe5fd226e, 0xe5bc322f, 0xe5fc331e,
            0xe7fc310e, 0xe7fd340b, 0xe7fd743b, 0xe7fd7c39, 0xe5fffc09, 0xe5ffbc09, 0xf5fc9f4b,
            0x757c825b, 0x753cc2fb, 0x753d525a, 0x753e125a, 0xf57e116a, 0xf4fe306a, 0xf4bd346b,
            0xf4bc34f9, 0xf4bf7cc9, 0xf4bedcc9, 0xfdfeddc9, ][..];

        let mut m = Matcher::new();
        panic!("{}", m.matches(fp1, fp2));
    }
}