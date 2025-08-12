// Generate a endless sinewave
// TODO: make generic for float, and integer based PCM16, PCM24, PCM32?

pub struct Sine {
    frequency: f32,
    amplitude: f32,
    sample_rate: u32,
    current_sample: usize,
}

impl Sine {
    pub fn new(frequency: f32, amplitude: f32, sample_rate: u32) -> Self {
        Self {
            frequency,
            amplitude,
            sample_rate,
            current_sample: 0,
        }
    }

    /// Create a generator with the sine's period in samples.
    pub fn new_with_sample_period(period_nr_samples: usize, amplitude: f32, sample_rate: u32) -> Self {
        let frequency = sample_rate as f32 / period_nr_samples as f32;
        Self {
            frequency,
            amplitude,
            sample_rate,
            current_sample: 0,
        }
    }
}

impl Iterator for Sine {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        let t = self.current_sample as f32 / self.sample_rate as f32;
        let sine = self.amplitude * (t * self.frequency * 2.0 * std::f32::consts::PI).sin();
        self.current_sample += 1;
        Some(sine)
    }
}

mod tests {
    #[test]
    fn test_print_sine() {
        let sine = super::Sine::new(1.0, 1.0, 44100);
        for sample in sine.take(20) {
            println!("{}", sample);
        }

        let sine = super::Sine::new_with_sample_period(20, 1.0, 44100);
        for (index, sample) in sine.take(40).enumerate() {
            println!("{}: {}", index, sample);
        }
    }
}
