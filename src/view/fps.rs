use std::collections::VecDeque;
use std::time::Duration;

#[derive(Debug, Clone, Default)]
pub struct Fps {
    pub durations: VecDeque<Duration>,
    pub max_nr_durations: usize,

    #[cfg(not(target_arch = "wasm32"))]
    start_time: Option<std::time::Instant>,
}

impl Fps {
    pub fn new(max_nr_durations: usize) -> Self {
        Self {
            durations: VecDeque::with_capacity(max_nr_durations),
            max_nr_durations,
            #[cfg(not(target_arch = "wasm32"))]
            start_time: None,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn start_frame(&mut self) {
        self.start_time = Some(std::time::Instant::now());
    }

    #[cfg(target_arch = "wasm32")]
    pub fn start_frame(&mut self) {}

    #[cfg(not(target_arch = "wasm32"))]
    pub fn end_frame(&mut self) {
        let Some(start_time) = self.start_time else {
            return;
        };
        let duration = std::time::Instant::now() - start_time;
        self.durations.push_back(duration);

        if self.durations.len() > self.max_nr_durations {
            self.durations.pop_front();
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn end_frame(&mut self) {}

    pub fn measure<F, R>(&mut self, func: F) -> R
    where
        F: FnOnce() -> R,
    {
        self.start_frame();
        let result = func();
        self.end_frame();
        result
    }

    pub fn ui(&self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.heading("FPS");
            ui.separator();
            if self.durations.is_empty() {
                ui.label("FPS metrics unavailable on wasm.");
                return;
            }
            let sum_duration = self.durations.iter().map(|d| d.as_secs_f64()).sum::<f64>();
            let avg_duration = sum_duration / self.durations.len() as f64;
            ui.label(format!(
                "Measured frame duration (avg {} frames): {:.3} ms",
                self.durations.len(),
                avg_duration * 1000.0
            ));
            ui.label(format!("Max FPS: {:.3}", 1.0 / avg_duration));

            let frame_time = ui.ctx().input(|i| i.stable_dt);
            ui.label(format!("egui frame time: {:.3} ms", frame_time * 1000.0));
            ui.label(format!("FPS from frame time: {:.3}", 1.0 / frame_time));
        });
    }
}
