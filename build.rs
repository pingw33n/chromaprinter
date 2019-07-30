const FFTW: &str = "fftw";
const VDSP: &str = "vdsp";
const FFT_FEATURES: &[&str] = &[FFTW, VDSP];

fn main() {
    let in_features = foreman::features().unwrap();

    let fft: Vec<_> = in_features.iter().filter(|f| FFT_FEATURES.contains(&f.as_str())).collect();

    if fft.len() != 1 {
        foreman::cfg("__fail_bad_fft_feature");
    }

    #[cfg(target_os="macos")]
    {
        println!("cargo:rustc-link-lib=framework=Accelerate");
    }
}
