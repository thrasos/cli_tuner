use std::collections::VecDeque;
use std::env;
use std::error::Error;
use std::f32::consts::PI;
use std::fmt;
use std::io;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};

const A4_FREQ: f32 = 440.0;
const MIN_FREQ: f32 = 70.0;
const MAX_FREQ: f32 = 550.0;
const ANALYSIS_WINDOW_MS: usize = 250;
const HISTORY_SECONDS: usize = 3;

#[derive(Clone, Copy, Debug)]
enum Tuning {
    Trichordo,
    Tetrachordo,
    ClassicGuitar,
}

const TRICHORDO_COURSES: [TargetPitch; 3] = [
    TargetPitch::new("D3", "low Re", 146.83),
    TargetPitch::new("A3", "La", 220.00),
    TargetPitch::new("D4", "high Re", 293.66),
];

const TETRACHORDO_COURSES: [TargetPitch; 4] = [
    TargetPitch::new("C3", "Do", 130.81),
    TargetPitch::new("F3", "Fa", 174.61),
    TargetPitch::new("A3", "La", 220.00),
    TargetPitch::new("D4", "Re", 293.66),
];

const CLASSIC_GUITAR_STRINGS: [TargetPitch; 6] = [
    TargetPitch::new("E2", "low E", 82.41),
    TargetPitch::new("A2", "A", 110.00),
    TargetPitch::new("D3", "D", 146.83),
    TargetPitch::new("G3", "G", 196.00),
    TargetPitch::new("B3", "B", 246.94),
    TargetPitch::new("E4", "high E", 329.63),
];

impl Tuning {
    fn from_str(value: &str) -> Option<Self> {
        match value {
            "trichordo" | "tri" | "3" => Some(Self::Trichordo),
            "tetrachordo" | "tetra" | "4" => Some(Self::Tetrachordo),
            "classic-guitar" | "classical-guitar" | "classic" | "guitar" => {
                Some(Self::ClassicGuitar)
            }
            _ => None,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Trichordo => "trichordo",
            Self::Tetrachordo => "tetrachordo",
            Self::ClassicGuitar => "classic-guitar",
        }
    }

    fn courses(self) -> &'static [TargetPitch] {
        match self {
            Self::Trichordo => &TRICHORDO_COURSES,
            Self::Tetrachordo => &TETRACHORDO_COURSES,
            Self::ClassicGuitar => &CLASSIC_GUITAR_STRINGS,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct TargetPitch {
    note: &'static str,
    course: &'static str,
    frequency: f32,
}

impl TargetPitch {
    const fn new(note: &'static str, course: &'static str, frequency: f32) -> Self {
        Self {
            note,
            course,
            frequency,
        }
    }
}

#[derive(Debug)]
struct Config {
    tuning: Tuning,
    reference_hz: f32,
    list_only: bool,
}

#[derive(Debug)]
struct CliError(String);

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Error for CliError {}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        eprintln!();
        print_usage();
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let config = parse_args()?;

    if config.list_only {
        print_targets(config.tuning);
        return Ok(());
    }

    print_banner(&config);
    run_tuner(config)
}

fn parse_args() -> Result<Config, Box<dyn Error>> {
    let mut tuning = Tuning::Tetrachordo;
    let mut reference_hz = A4_FREQ;
    let mut list_only = false;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            "--list" => list_only = true,
            "--tuning" | "-t" => {
                let value = args.next().ok_or_else(|| {
                    CliError(
                        "missing value for --tuning; expected trichordo, tetrachordo, or classic-guitar"
                            .into(),
                    )
                })?;
                tuning = Tuning::from_str(&value).ok_or_else(|| {
                    CliError(format!(
                        "unsupported tuning '{value}'; expected trichordo, tetrachordo, or classic-guitar"
                    ))
                })?;
            }
            "--reference" | "-r" => {
                let value = args.next().ok_or_else(|| {
                    CliError("missing value for --reference; expected a frequency in Hz".into())
                })?;
                reference_hz = value.parse::<f32>().map_err(|_| {
                    CliError(format!(
                        "invalid reference frequency '{value}'; expected a number like 440"
                    ))
                })?;
                if !(400.0..=490.0).contains(&reference_hz) {
                    return Err(Box::new(CliError(
                        "reference frequency must be between 400 and 490 Hz".into(),
                    )));
                }
            }
            _ => {
                return Err(Box::new(CliError(format!("unknown argument '{arg}'"))));
            }
        }
    }

    Ok(Config {
        tuning,
        reference_hz,
        list_only,
    })
}

fn print_usage() {
    println!("cli_tuner: microphone tuner for bouzouki and classic guitar");
    println!();
    println!("Usage:");
    println!(
        "  cargo run -- [--tuning tetrachordo|trichordo|classic-guitar] [--reference 440] [--list]"
    );
    println!();
    println!("Options:");
    println!(
        "  --tuning, -t     Instrument tuning preset (default: tetrachordo)"
    );
    println!("  --reference, -r  Concert A frequency in Hz (default: 440)");
    println!("  --list           Print target strings for the selected tuning");
    println!("  --help, -h       Show this help");
}

fn print_targets(tuning: Tuning) {
    println!("{} tuning targets:", tuning.label());
    for target in tuning.courses() {
        println!(
            "  {:<3} {:<8} {:>6.2} Hz",
            target.note, target.course, target.frequency
        );
    }
}

fn print_banner(config: &Config) {
    println!("String instrument tuner");
    println!("tuning: {}", config.tuning.label());
    println!("reference A4: {:.2} Hz", config.reference_hz);
    println!("input: default microphone");
    println!("press Ctrl+C to stop");
    println!();
}

fn run_tuner(config: Config) -> Result<(), Box<dyn Error>> {
    let host = cpal::default_host();
    let device = host.default_input_device().ok_or_else(|| {
        Box::new(CliError("no input device available".into())) as Box<dyn Error>
    })?;
    let supported_config = device.default_input_config()?;
    let sample_rate = supported_config.sample_rate().0 as usize;
    let channel_count = supported_config.channels() as usize;
    let history_len = sample_rate * HISTORY_SECONDS;

    let buffer = Arc::new(Mutex::new(VecDeque::<f32>::with_capacity(history_len)));
    let err_fn = |err| eprintln!("stream error: {err}");

    let stream = match supported_config.sample_format() {
        SampleFormat::F32 => build_input_stream::<f32>(
            &device,
            &supported_config.into(),
            channel_count,
            history_len,
            buffer.clone(),
            err_fn,
        )?,
        SampleFormat::I16 => build_input_stream::<i16>(
            &device,
            &supported_config.into(),
            channel_count,
            history_len,
            buffer.clone(),
            err_fn,
        )?,
        SampleFormat::U16 => build_input_stream::<u16>(
            &device,
            &supported_config.into(),
            channel_count,
            history_len,
            buffer.clone(),
            err_fn,
        )?,
        format => {
            return Err(Box::new(CliError(format!(
                "unsupported sample format: {format:?}"
            ))));
        }
    };

    stream.play()?;

    loop {
        thread::sleep(Duration::from_millis(200));

        let samples = latest_window(&buffer, sample_rate, ANALYSIS_WINDOW_MS);
        if samples.len() < sample_rate / 10 {
            continue;
        }

        if let Some(freq) = detect_pitch(&samples, sample_rate as f32) {
            let analysis = analyze_pitch(freq, config.tuning, config.reference_hz);
            println!(
                "{:>7.2} Hz | {:<6} {:<8} | {:+6.1} cents | {}",
                freq,
                analysis.target.note,
                analysis.target.course,
                analysis.cents_off,
                meter(analysis.cents_off)
            );
        }
    }
}

fn build_input_stream<T>(
    device: &cpal::Device,
    config: &StreamConfig,
    channel_count: usize,
    history_len: usize,
    buffer: Arc<Mutex<VecDeque<f32>>>,
    err_fn: impl FnMut(cpal::StreamError) + Send + 'static,
) -> Result<Stream, Box<dyn Error>>
where
    T: cpal::SizedSample,
    f32: cpal::FromSample<T>,
{
    let stream = device.build_input_stream(
        config,
        move |data: &[T], _| {
            let mut guard = match buffer.lock() {
                Ok(guard) => guard,
                Err(_) => return,
            };

            for frame in data.chunks(channel_count) {
                let mut mixed = 0.0_f32;
                for sample in frame {
                    mixed += sample.to_sample::<f32>();
                }
                mixed /= frame.len() as f32;

                if guard.len() == history_len {
                    guard.pop_front();
                }
                guard.push_back(mixed);
            }
        },
        err_fn,
        None,
    )?;
    Ok(stream)
}

fn latest_window(
    buffer: &Arc<Mutex<VecDeque<f32>>>,
    sample_rate: usize,
    window_ms: usize,
) -> Vec<f32> {
    let wanted = sample_rate * window_ms / 1000;
    let guard = match buffer.lock() {
        Ok(guard) => guard,
        Err(_) => return Vec::new(),
    };

    let len = guard.len();
    let start = len.saturating_sub(wanted);
    guard
        .iter()
        .skip(start)
        .copied()
        .enumerate()
        .map(|(i, sample)| {
            let angle = 2.0 * PI * i as f32 / wanted.max(1) as f32;
            let window = 0.5 - 0.5 * angle.cos();
            sample * window
        })
        .collect()
}

fn detect_pitch(samples: &[f32], sample_rate: f32) -> Option<f32> {
    let rms = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
    if rms < 0.01 {
        return None;
    }

    let min_lag = (sample_rate / MAX_FREQ).floor() as usize;
    let max_lag = (sample_rate / MIN_FREQ).ceil() as usize;
    if samples.len() <= max_lag + 2 {
        return None;
    }

    let mut best_lag = 0usize;
    let mut best_score = f32::MIN;

    for lag in min_lag..=max_lag {
        let slice_len = samples.len() - lag;
        let mut corr = 0.0;
        let mut energy = 0.0;

        for i in 0..slice_len {
            let a = samples[i];
            let b = samples[i + lag];
            corr += a * b;
            energy += a * a + b * b;
        }

        if energy <= f32::EPSILON {
            continue;
        }

        let score = 2.0 * corr / energy;
        if score > best_score {
            best_score = score;
            best_lag = lag;
        }
    }

    if best_lag == 0 || best_score < 0.65 {
        return None;
    }

    let refined_lag = parabolic_lag(samples, best_lag);
    Some(sample_rate / refined_lag)
}

fn parabolic_lag(samples: &[f32], lag: usize) -> f32 {
    let score = |shift: usize| -> f32 {
        let slice_len = samples.len() - shift;
        let mut corr = 0.0;
        let mut energy = 0.0;
        for i in 0..slice_len {
            let a = samples[i];
            let b = samples[i + shift];
            corr += a * b;
            energy += a * a + b * b;
        }
        if energy <= f32::EPSILON {
            0.0
        } else {
            2.0 * corr / energy
        }
    };

    if lag == 0 || lag + 1 >= samples.len() {
        return lag as f32;
    }

    let y0 = score(lag.saturating_sub(1));
    let y1 = score(lag);
    let y2 = score(lag + 1);
    let denom = y0 - 2.0 * y1 + y2;
    if denom.abs() < 1e-6 {
        lag as f32
    } else {
        lag as f32 + 0.5 * (y0 - y2) / denom
    }
}

#[derive(Clone, Copy)]
struct PitchAnalysis {
    target: TargetPitch,
    cents_off: f32,
}

fn analyze_pitch(freq: f32, tuning: Tuning, reference_hz: f32) -> PitchAnalysis {
    let ratio = reference_hz / A4_FREQ;
    let mut best_target = tuning.courses()[0];
    let mut best_cents = cents_between(freq, best_target.frequency * ratio);

    for target in &tuning.courses()[1..] {
        let cents = cents_between(freq, target.frequency * ratio);
        if cents.abs() < best_cents.abs() {
            best_target = *target;
            best_cents = cents;
        }
    }

    PitchAnalysis {
        target: best_target,
        cents_off: best_cents,
    }
}

fn cents_between(observed: f32, target: f32) -> f32 {
    1200.0 * (observed / target).log2()
}

fn meter(cents_off: f32) -> &'static str {
    if cents_off.abs() <= 5.0 {
        "in tune"
    } else if cents_off < 0.0 {
        "tune up"
    } else {
        "tune down"
    }
}

#[allow(dead_code)]
fn wait_for_enter() -> io::Result<()> {
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine_wave(freq: f32, sample_rate: usize, ms: usize) -> Vec<f32> {
        let samples = sample_rate * ms / 1000;
        (0..samples)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                (2.0 * PI * freq * t).sin() * 0.5
            })
            .collect()
    }

    #[test]
    fn detects_a3_pitch() {
        let samples = sine_wave(220.0, 48_000, 300);
        let freq = detect_pitch(&samples, 48_000.0).unwrap();
        assert!((freq - 220.0).abs() < 1.0, "detected frequency: {freq}");
    }

    #[test]
    fn classifies_tetrachordo_target() {
        let analysis = analyze_pitch(293.0, Tuning::Tetrachordo, 440.0);
        assert_eq!(analysis.target.note, "D4");
        assert!(analysis.cents_off.abs() < 5.0);
    }

    #[test]
    fn classifies_trichordo_target() {
        let analysis = analyze_pitch(147.0, Tuning::Trichordo, 440.0);
        assert_eq!(analysis.target.note, "D3");
        assert!(analysis.cents_off.abs() < 5.0);
    }

    #[test]
    fn classifies_classic_guitar_target() {
        let analysis = analyze_pitch(329.5, Tuning::ClassicGuitar, 440.0);
        assert_eq!(analysis.target.note, "E4");
        assert!(analysis.cents_off.abs() < 5.0);
    }
}
