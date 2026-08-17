//! Developer-facing Linux `perf` capture support.

use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

#[cfg_attr(
    not(all(target_os = "linux", not(target_arch = "wasm32"))),
    allow(dead_code)
)]
#[derive(Debug)]
enum CaptureEvent {
    Recording {
        capture_id: u64,
    },
    Finalizing {
        capture_id: u64,
    },
    Finished {
        capture_id: u64,
        result: Result<std::path::PathBuf, String>,
    },
}

#[cfg_attr(
    not(all(target_os = "linux", not(target_arch = "wasm32"))),
    allow(dead_code)
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CapturePhase {
    Waiting,
    Recording,
    Finalizing,
}

/// Configuration and runtime state for an ad-hoc `perf record` capture.
#[derive(Debug)]
pub struct PerfCapture {
    label: String,
    delay_secs: u64,
    duration_secs: u64,
    phase: Option<CapturePhase>,
    phase_deadline: Option<web_time::Instant>,
    result: Option<Result<std::path::PathBuf, String>>,
    #[cfg_attr(
        not(all(target_os = "linux", not(target_arch = "wasm32"))),
        allow(dead_code)
    )]
    next_capture_id: u64,
    active_capture_id: Option<u64>,
    #[cfg_attr(
        not(all(target_os = "linux", not(target_arch = "wasm32"))),
        allow(dead_code)
    )]
    event_tx: Sender<CaptureEvent>,
    event_rx: Receiver<CaptureEvent>,
}

impl Default for PerfCapture {
    fn default() -> Self {
        let (event_tx, event_rx) = std::sync::mpsc::channel();
        Self {
            label: "scenario".to_owned(),
            delay_secs: 2,
            duration_secs: 10,
            phase: None,
            phase_deadline: None,
            result: None,
            next_capture_id: 1,
            active_capture_id: None,
            event_tx,
            event_rx,
        }
    }
}

impl PerfCapture {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        self.drain_events();

        ui.heading("Profile");
        if !cfg!(all(target_os = "linux", not(target_arch = "wasm32"))) {
            ui.label("Perf capture is available only in the native Linux application.");
            return;
        }

        let active = self.phase.is_some();
        ui.add_enabled_ui(!active, |ui| {
            ui.horizontal(|ui| {
                ui.label("Scenario");
                ui.text_edit_singleline(&mut self.label);
            });
            ui.horizontal(|ui| {
                ui.label("Delay (seconds)");
                ui.add(egui::DragValue::new(&mut self.delay_secs));
            });
            ui.horizontal(|ui| {
                ui.label("Duration (seconds)");
                ui.add(egui::DragValue::new(&mut self.duration_secs));
            });
        });

        if ui
            .add_enabled(!active, egui::Button::new("Start capture"))
            .clicked()
            && let Err(error) = self.start()
        {
            self.result = Some(Err(error));
        }

        ui.add_space(8.0);
        self.show_status(ui);
        ui.add_space(8.0);
        Self::show_usage_guidance(ui);

        if active {
            ui.ctx().request_repaint_after(Duration::from_millis(50));
        }
    }

    fn show_usage_guidance(ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Profiling workflow")
            .default_open(false)
            .show(ui, |ui| {
                ui.label("1. Launch an optimized build with source information:");
                ui.monospace("cargo +1.93.0 run --profile profiling");
                ui.label("2. Open the Profile tab in the right panel.");
                ui.label("3. Enter a scenario label, delay, and duration.");
                ui.label("4. Start the capture and perform one focused interaction.");
                ui.label("5. Open the generated perf data file in Hotspot.");
                ui.add_space(4.0);
                ui.label(
                    "Perf attachment requires suitable Linux perf_event/ptrace permissions. If it \
                     fails, check kernel.perf_event_paranoid and the error shown above.",
                );
                ui.label(
                    "Do not rebuild or replace the profiled executable before Hotspot resolves its \
                     source locations; use the exact binary that produced the capture.",
                );
            });
    }

    fn show_status(&self, ui: &mut egui::Ui) {
        match self.phase {
            Some(CapturePhase::Waiting) => {
                ui.heading(format!(
                    "Waiting — capture starts in {}",
                    self.remaining_text()
                ));
                ui.label("Prepare the operation you want to measure.");
            }
            Some(CapturePhase::Recording) => {
                egui::Frame::group(ui.style())
                    .fill(ui.visuals().error_fg_color.gamma_multiply(0.15))
                    .stroke(egui::Stroke::new(2.0, ui.visuals().error_fg_color))
                    .show(ui, |ui| {
                        ui.colored_label(
                            ui.visuals().error_fg_color,
                            egui::RichText::new(format!(
                                "● RECORDING — {} remaining",
                                self.remaining_text()
                            ))
                            .strong()
                            .size(18.0),
                        );
                        ui.label("Perform the operation now.");
                    });
            }
            Some(CapturePhase::Finalizing) => {
                ui.heading("Finalizing capture…");
                ui.spinner();
            }
            None => match &self.result {
                None => {
                    ui.label("Status: Idle");
                }
                Some(Ok(path)) => {
                    ui.colored_label(
                        ui.visuals().warn_fg_color,
                        egui::RichText::new("Completed").strong(),
                    );
                    ui.label(format!("Capture saved to {}", path.display()));
                }
                Some(Err(error)) => {
                    ui.colored_label(
                        ui.visuals().error_fg_color,
                        egui::RichText::new("Failed").strong(),
                    );
                    ui.label(error);
                }
            },
        }
    }

    fn remaining_text(&self) -> String {
        let seconds = self
            .phase_deadline
            .map(|deadline| {
                deadline
                    .saturating_duration_since(web_time::Instant::now())
                    .as_secs_f32()
            })
            .unwrap_or_default();
        format!("{seconds:.1}s")
    }

    fn drain_events(&mut self) {
        while let Ok(event) = self.event_rx.try_recv() {
            let capture_id = match &event {
                CaptureEvent::Recording { capture_id }
                | CaptureEvent::Finalizing { capture_id }
                | CaptureEvent::Finished { capture_id, .. } => *capture_id,
            };
            if self.active_capture_id != Some(capture_id) {
                continue;
            }

            match event {
                CaptureEvent::Recording { .. } => {
                    self.phase = Some(CapturePhase::Recording);
                    self.phase_deadline =
                        Some(web_time::Instant::now() + Duration::from_secs(self.duration_secs));
                }
                CaptureEvent::Finalizing { .. } => {
                    self.phase = Some(CapturePhase::Finalizing);
                    self.phase_deadline = None;
                }
                CaptureEvent::Finished { result, .. } => {
                    self.phase = None;
                    self.phase_deadline = None;
                    self.active_capture_id = None;
                    self.result = Some(result);
                }
            }
        }
    }

    fn start(&mut self) -> Result<(), String> {
        if self.duration_secs == 0 {
            return Err("Capture duration must be greater than zero".to_owned());
        }
        if self.phase.is_some() {
            return Err("A Perf capture is already active".to_owned());
        }

        #[cfg(all(target_os = "linux", not(target_arch = "wasm32")))]
        {
            let capture_id = self.next_capture_id;
            self.next_capture_id += 1;
            let delay = Duration::from_secs(self.delay_secs);
            let duration_secs = self.duration_secs;
            let output_path = output_path(&self.label)?;
            let pid = std::process::id();
            let event_tx = self.event_tx.clone();

            self.phase = Some(CapturePhase::Waiting);
            self.phase_deadline = Some(web_time::Instant::now() + delay);
            self.result = None;
            self.active_capture_id = Some(capture_id);
            std::thread::spawn(move || {
                std::thread::sleep(delay);
                let result = run_perf(capture_id, pid, duration_secs, output_path, &event_tx);
                let _ = event_tx.send(CaptureEvent::Finished { capture_id, result });
            });
            Ok(())
        }

        #[cfg(not(all(target_os = "linux", not(target_arch = "wasm32"))))]
        Err("Perf capture is available only in the native Linux application".to_owned())
    }
}

#[cfg_attr(
    not(all(target_os = "linux", not(target_arch = "wasm32"))),
    allow(dead_code)
)]
fn sanitize_label(label: &str) -> String {
    let sanitized = label
        .trim()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    if sanitized.is_empty() {
        "scenario".to_owned()
    } else {
        sanitized
    }
}

#[cfg_attr(
    not(all(target_os = "linux", not(target_arch = "wasm32"))),
    allow(dead_code)
)]
fn output_filename(timestamp: u128, label: &str) -> String {
    format!("perf.{timestamp}.{}.data", sanitize_label(label))
}

#[cfg(all(target_os = "linux", not(target_arch = "wasm32")))]
fn output_path(label: &str) -> Result<std::path::PathBuf, String> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| format!("System clock is before the Unix epoch: {error}"))?
        .as_millis();
    Ok(std::path::PathBuf::from(output_filename(timestamp, label)))
}

#[cfg(all(target_os = "linux", not(target_arch = "wasm32")))]
fn run_perf(
    capture_id: u64,
    pid: u32,
    duration_secs: u64,
    output_path: std::path::PathBuf,
    event_tx: &Sender<CaptureEvent>,
) -> Result<std::path::PathBuf, String> {
    let child = std::process::Command::new("perf")
        .args(["record", "--pid"])
        .arg(pid.to_string())
        .args(["--call-graph", "dwarf", "--output"])
        .arg(&output_path)
        .arg("--")
        .arg("sleep")
        .arg(duration_secs.to_string())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|error| format!("Failed to start perf: {error}"))?;

    let _ = event_tx.send(CaptureEvent::Recording { capture_id });
    let finalizing_tx = event_tx.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(duration_secs));
        let _ = finalizing_tx.send(CaptureEvent::Finalizing { capture_id });
    });

    let output = child
        .wait_with_output()
        .map_err(|error| format!("Failed while waiting for perf: {error}"))?;
    if output.status.success() {
        Ok(output_path)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        Err(if stderr.is_empty() {
            format!("Perf exited with status {}", output.status)
        } else {
            format!("Perf failed: {stderr}")
        })
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{CaptureEvent, CapturePhase, PerfCapture, output_filename, sanitize_label};

    #[test]
    fn sanitizes_label_for_filename() {
        assert_eq!(sanitize_label(" zoom / pan #1 "), "zoom___pan__1");
        assert_eq!(sanitize_label(""), "scenario");
        assert_eq!(sanitize_label("load-WAV_2"), "load-WAV_2");
    }

    #[test]
    fn builds_deterministic_output_filename() {
        assert_eq!(
            output_filename(123_456, "zoom / pan"),
            "perf.123456.zoom___pan.data"
        );
    }

    #[test]
    fn rejects_zero_duration_without_starting_capture() {
        let mut capture = PerfCapture {
            duration_secs: 0,
            ..Default::default()
        };

        assert_eq!(
            capture.start(),
            Err("Capture duration must be greater than zero".to_owned())
        );
        assert_eq!(capture.phase, None);
        assert_eq!(capture.active_capture_id, None);
    }

    #[test]
    fn applies_capture_events_in_order() {
        let mut capture = PerfCapture {
            active_capture_id: Some(7),
            phase: Some(CapturePhase::Waiting),
            ..Default::default()
        };

        capture
            .event_tx
            .send(CaptureEvent::Recording { capture_id: 7 })
            .unwrap();
        capture.drain_events();
        assert_eq!(capture.phase, Some(CapturePhase::Recording));
        assert!(capture.phase_deadline.is_some());

        capture
            .event_tx
            .send(CaptureEvent::Finalizing { capture_id: 7 })
            .unwrap();
        capture.drain_events();
        assert_eq!(capture.phase, Some(CapturePhase::Finalizing));
        assert_eq!(capture.phase_deadline, None);

        let path = PathBuf::from("perf.123.scenario.data");
        capture
            .event_tx
            .send(CaptureEvent::Finished {
                capture_id: 7,
                result: Ok(path.clone()),
            })
            .unwrap();
        capture.drain_events();
        assert_eq!(capture.phase, None);
        assert_eq!(capture.active_capture_id, None);
        assert_eq!(capture.result, Some(Ok(path)));
    }

    #[test]
    fn ignores_events_from_stale_capture() {
        let mut capture = PerfCapture {
            active_capture_id: Some(8),
            phase: Some(CapturePhase::Waiting),
            ..Default::default()
        };
        capture
            .event_tx
            .send(CaptureEvent::Recording { capture_id: 7 })
            .unwrap();

        capture.drain_events();

        assert_eq!(capture.phase, Some(CapturePhase::Waiting));
        assert_eq!(capture.active_capture_id, Some(8));
    }
}
