pub fn freq_to_index(freq: f64, frame_size: u32, sample_rate: u32) -> u32 {
    (frame_size as f64 * freq / sample_rate as f64).round() as u32
}

pub fn index_to_freq(i: u32, frame_size: u32, sample_rate: u32) -> f64 {
    i as f64 * sample_rate as f64 / frame_size as f64
}

pub fn hamming_distance(a: u32, b: u32) -> u32 {
    (a ^ b).count_ones()
}