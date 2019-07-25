mod gaussian_filter;
mod gradient;

use rand::prelude::*;
use std::cmp;

use crate::util::hamming_distance;
use crate::fingerprint::matcher::gaussian_filter::GaussianFilter;

const ALIGN_BITS: u32 = 12;
const ALIGN_MASK: u32 = (1 << ALIGN_BITS) - 1;
const HASH_SHIFT: u32 = 32 - ALIGN_BITS;
const HASH_MASK: u32 = ((1 << ALIGN_BITS) - 1) << HASH_SHIFT;
const OFFSET_MASK: u32 = (1 << (32 - ALIGN_BITS - 1)) - 1;
const SOURCE_MASK: u32 = 1 << (32 - ALIGN_BITS - 1);

type Elem = f32;

pub struct Matcher {
    item_duration_secs: f64,
    delay_secs: f64,
    match_threshold: f64,
    offsets: Vec<u32>,
    histogram: Vec<u32>,
    best_alignments: Vec<(u32, usize)>,
    segments: Vec<Segment>,
}

impl Matcher {
    pub fn new(item_duration_secs: f64, delay_secs: f64, match_threshold: f64) -> Self {
        Self {
            item_duration_secs,
            delay_secs,
            match_threshold,
            offsets: Vec::new(),
            histogram: Vec::new(),
            best_alignments: Vec::new(),
            segments: Vec::new(),
        }
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
        self.find_best_alignments();

        // TODO Move to field so it doesn't reallocate.
        let mut bit_counts = Vec::new();
//        let mut orig_bit_counts = Vec::new();
//        let mut smoothed_bit_counts = Vec::new();

        let mut gaussian_filter = GaussianFilter::new(8.0, 3);

        self.segments.clear();
        for item in &self.best_alignments {
            let offset_diff = item.1 as isize - fp2.len() as isize;
            let offset1 = cmp::max(offset_diff, 0) as usize;
            let offset2 = cmp::max(-offset_diff, 0) as usize;

            let len = cmp::min(fp1.len() - offset1, fp2.len() - offset2);
            bit_counts.clear();
            bit_counts.reserve(len);

            for (&v1, &v2) in fp1[..offset1].iter().zip(fp2[..offset2].iter()) {
                bit_counts.push(hamming_distance(v1, v2) as f32 +
                    rand::thread_rng().gen_range(0.0_f32, 0.001));
            }

            let smoothed_bit_counts = gaussian_filter.apply(&bit_counts);

//            std::vector<float> orig_bit_counts = bit_counts;
//            std::vector<float> smoothed_bit_counts;
//            GaussianFilter(bit_counts, smoothed_bit_counts, 8.0, 3);
//
//            std::vector<float> gradient(size);
//            Gradient(smoothed_bit_counts.begin(), smoothed_bit_counts.end(), gradient.begin());
//
//            for (size_t i = 0; i < size; i++) {
//                gradient[i] = std::abs(gradient[i]);
//            }
//
//            std::vector<size_t> gradient_peaks;
//            for (size_t i = 0; i < size; i++) {
//                const auto gi = gradient[i];
//                if (i > 0 && i < size - 1 && gi > 0.15 && gi >= gradient[i - 1] && gi >= gradient[i + 1]) {
//                    if (gradient_peaks.empty() || gradient_peaks.back() + 1 < i) {
//                        gradient_peaks.push_back(i);
//                    }
//                }
//            }
//            gradient_peaks.push_back(size);
//
//            size_t begin = 0;
//            for (size_t end : gradient_peaks) {
//                const auto duration = end - begin;
//                const auto score = std::accumulate(orig_bit_counts.begin() + begin, orig_bit_counts.begin() + end, 0.0) / duration;
//                if (score < m_match_threshold) {
//                    bool added = false;
//                    if (!m_segments.empty()) {
//                        auto &s1 = m_segments.back();
//                        if (std::abs(s1.score - score) < 0.7) {
//                            s1 = s1.merged(Segment(offset1 + begin, offset2 + begin, duration, score));
//                            added = true;
//                        }
//                    }
//                    if (!added) {
//                        m_segments.emplace_back(offset1 + begin, offset2 + begin, duration, score);
//                    }
//                }
//                begin = end;
//            }
            break;
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
            self.offsets.push((align_strip(v) << HASH_SHIFT) | (i as u32 & OFFSET_MASK));
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

    fn find_best_alignments(&mut self) {
        self.best_alignments.clear();
            for i in 0..self.histogram.len() {
                let count = self.histogram[i];
                if count > 1 {
                    let is_peak_left = i > 0 && self.histogram[i - 1] <= count || i == 0;
                    let is_peak_right = i < self.histogram.len() - 1 && self.histogram[i + 1] <= count ||
                        i < self.histogram.len() - 1;
                    if is_peak_left && is_peak_right {
                        self.best_alignments.push((count, i));
                    }
                }
            }
            self.best_alignments.sort();
    }

    fn get_hash_time(&self, i: usize) -> f64 {
        self.item_duration_secs * i as f64
    }

    fn get_hash_duration(&self, i: usize) -> f64 {
        self.get_hash_time(i) + self.delay_secs
    }
}

struct Segment {
    pos1: usize,
    pos2: usize,
    duration: usize,
    score: f64,
    left_score: f64,
    right_score: f64,
}

impl Segment {
    pub fn public_score(&self) -> u32 {
        (self.score * 100.0 + 0.5) as u32
    }

    pub fn merge(&self, o: &Self) -> Self {
        assert_eq!(self.pos1 + self.duration, o.pos1);
        assert_eq!(self.pos2 + self.duration, o.pos2);
        let duration = self.duration + o.duration;
        let score = (self.score * self.duration as f64 + o.score * o.duration as f64)
            / duration as f64;
        Self {
            pos1: o.pos1,
            pos2: o.pos2,
            duration,
            score,
            left_score: score,
            right_score: score,
        }
    }
}

fn align_strip(v: u32) -> u32 {
    v >> (32 - ALIGN_BITS)
}