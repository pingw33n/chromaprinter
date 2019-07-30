#![allow(dead_code)]
#![deny(non_snake_case)]
//#![deny(unused_imports)]
#![deny(unused_must_use)]

#[cfg(__fail_bad_fft_feature)]
compile_error!("Exactly one FFT library must be selected via features: fftw, vdsp. \
                Did you forgot to disable default features?");

mod audio;
mod chroma;
mod fingerprint;
mod pipeline;
#[cfg(test)]
mod test;
mod util;

use crate::chroma::Chroma;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Algorithm {
    Test2,
}

impl Algorithm {
    fn fp_config(&self) -> &FpConfig {
        use Algorithm::*;
        match self {
            Test2 => &FP_CONFIG_TEST2,
        }
    }
}

struct RollingIntImage {

}

enum FilterKind {
    F0,
    F1,
    F2,
    F3,
    F4,
    F5,
}

struct Filter {
    kind: FilterKind,
    y: u32,
    height: u32,
    width: u32,
}

impl Filter {
    pub const fn new(kind: FilterKind, y: u32, height: u32, width: u32) -> Self {
        Self {
            kind,
            y,
            height,
            width,
        }
    }
}

#[derive(Debug)]
struct Quantizer(f64, f64, f64);

impl Quantizer {
    fn quantize(&self, value: f64) -> u32 {
        debug_assert!(self.0 <= self.1 && self.1 <= self.2);
        if value < self.1 {
            if value < self.0 {
                0
            } else {
                1
            }
        } else if value < self.2 {
            2
        } else {
            3
        }
    }
}

struct Classifier {
    filter: Filter,
    quantizer: Quantizer,
}

impl Classifier {
    pub const fn new(filter: Filter, quantizer: Quantizer) -> Self {
        Self {
            filter,
            quantizer,
        }
    }
}

const DEFAULT_SAMPLE_RATE: u32 = 11025;

struct FpConfig {
    classifiers: &'static [Classifier],
    filter_coefficients: &'static [f64],
    interpolate: bool,
    remove_silence: bool,
    silence_threshold: u32,
    frame_size: u32,
    frame_overlap: u32,
    max_filter_width: u32,
}

impl FpConfig {
    pub const fn new(
        classifiers: &'static [Classifier],
        max_filter_width: u32,
        filter_coefficients: &'static [f64],
        interpolate: bool,
        remove_silence: bool,
        silence_threshold: u32,
        frame_size: u32,
        frame_overlap: u32,
    ) -> Self {
        Self {
            classifiers,
            filter_coefficients,
            interpolate,
            remove_silence,
            silence_threshold,
            frame_size,
            frame_overlap,
            max_filter_width,
        }
    }

    fn sample_rate(&self) -> u32 {
        DEFAULT_SAMPLE_RATE
    }

    fn item_duration(&self) -> u32 {
        self.frame_size - self.frame_overlap
    }

    fn item_duration_in_seconds(&self) -> f64 {
        self.item_duration() as f64 / self.sample_rate() as f64
    }

    fn delay(&self) -> u32 {
        ((self.filter_coefficients.len() as u32 - 1) + (self.max_filter_width - 1))
            * self.item_duration() + self.frame_overlap
    }

    fn delay_in_seconds(&self) -> f64 {
        self.delay() as f64 / self.sample_rate() as f64
    }
}

const DEFAULT_FRAME_SIZE: u32 = 4096;
const DEFAULT_FRAME_OVERLAP: u32 = DEFAULT_FRAME_SIZE - DEFAULT_FRAME_SIZE / 3;


static CHROMA_FILTER_COEFFICIENTS: &[f64] = &[0.25, 0.75, 1.0, 0.75, 0.25];

static CLASSIFIERS_TEST2: &[Classifier] = &[
	Classifier::new(Filter::new(FilterKind::F0, 4, 3, 15), Quantizer(1.98215, 2.35817, 2.63523)),
	Classifier::new(Filter::new(FilterKind::F4, 4, 6, 15), Quantizer(-1.03809, -0.651211, -0.282167)),
	Classifier::new(Filter::new(FilterKind::F1, 0, 4, 16), Quantizer(-0.298702, 0.119262, 0.558497)),
	Classifier::new(Filter::new(FilterKind::F3, 8, 2, 12), Quantizer(-0.105439, 0.0153946, 0.135898)),
	Classifier::new(Filter::new(FilterKind::F3, 4, 4, 8), Quantizer(-0.142891, 0.0258736, 0.200632)),
	Classifier::new(Filter::new(FilterKind::F4, 0, 3, 5), Quantizer(-0.826319, -0.590612, -0.368214)),
	Classifier::new(Filter::new(FilterKind::F1, 2, 2, 9), Quantizer(-0.557409, -0.233035, 0.0534525)),
	Classifier::new(Filter::new(FilterKind::F2, 7, 3, 4), Quantizer(-0.0646826, 0.00620476, 0.0784847)),
	Classifier::new(Filter::new(FilterKind::F2, 6, 2, 16), Quantizer(-0.192387, -0.029699, 0.215855)),
	Classifier::new(Filter::new(FilterKind::F2, 1, 3, 2), Quantizer(-0.0397818, -0.00568076, 0.0292026)),
	Classifier::new(Filter::new(FilterKind::F5, 10, 1, 15), Quantizer(-0.53823, -0.369934, -0.190235)),
	Classifier::new(Filter::new(FilterKind::F3, 6, 2, 10), Quantizer(-0.124877, 0.0296483, 0.139239)),
	Classifier::new(Filter::new(FilterKind::F2, 1, 1, 14), Quantizer(-0.101475, 0.0225617, 0.231971)),
	Classifier::new(Filter::new(FilterKind::F3, 5, 6, 4), Quantizer(-0.0799915, -0.00729616, 0.063262)),
	Classifier::new(Filter::new(FilterKind::F1, 9, 2, 12), Quantizer(-0.272556, 0.019424, 0.302559)),
	Classifier::new(Filter::new(FilterKind::F3, 4, 2, 14), Quantizer(-0.164292, -0.0321188, 0.0846339)),
];
const CLASSIFIERS_TEST2_MAX_FILTER_WIDTH: u32 = 16;

// Trained on 60k pairs based on eMusic samples (mp3)
static FP_CONFIG_TEST2: FpConfig = FpConfig::new(
    CLASSIFIERS_TEST2,
    CLASSIFIERS_TEST2_MAX_FILTER_WIDTH,
    CHROMA_FILTER_COEFFICIENTS,
    false,
    false,
    0,
    DEFAULT_FRAME_SIZE,
    DEFAULT_FRAME_OVERLAP);



struct Fingerprinter {
    chroma: Chroma,
//	ChromaNormalizer *m_chroma_normalizer;
//    chroma_filter: ChromaFilter,
//	FFT *m_fft;
//	AudioProcessor *m_audio_processor;
//	FingerprintCalculator *m_fingerprint_calculator;
//	FingerprinterConfiguration *m_config;
//	SilenceRemover *m_silence_remover;
}

pub struct Chromaprint {
    algorithm: Algorithm,

}




