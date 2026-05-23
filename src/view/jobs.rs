use egui::{self, RichText};

use crate::model::{self, Action};

pub fn ui_panel(ui: &mut egui::Ui, model: &mut model::Model) {
    ui.heading("Jobs");
    ui.horizontal(|ui| {
        if ui.button("start demo job").clicked() {
            model.actions.push(Action::StartDemoJob(Default::default()));
        }
        if ui.button("start 3 demo jobs").clicked() {
            model.actions.extend(
                std::iter::repeat_with(|| Action::StartDemoJob(Default::default())).take(3),
            );
        }
    });
    ui.label("Phase 1 keeps this explicit: a fake job proves the shared progress/result flow.");
    ui.add_space(4.0);

    let active_jobs = model.job_mgr.jobs().cloned().collect::<Vec<_>>();
    if active_jobs.is_empty() {
        ui.label("No active jobs.");
    } else {
        for job in active_jobs {
            ui.group(|ui| {
                ui.label(RichText::new(&job.label).strong());
                ui.label(format!("kind: {:?}", job.kind));
                ui.label(format!("status: {:?}", job.status));
                ui.label(format!("stage: {}", job.progress.stage_name));
                if let Some(message) = &job.message {
                    ui.label(message);
                }
                let progress = job.progress.overall_fraction.clamp(0.0, 1.0);
                ui.add(
                    egui::ProgressBar::new(progress)
                        .show_percentage()
                        .text(format!("job {}", job.job_id)),
                );
            });
            ui.add_space(4.0);
        }
    }

    ui.add_space(8.0);
    ui.heading("Recent");
    let finished_jobs = model
        .job_mgr
        .recent_finished()
        .take(5)
        .cloned()
        .collect::<Vec<_>>();
    if finished_jobs.is_empty() {
        ui.label("No completed jobs yet.");
    } else {
        for job in finished_jobs {
            ui.label(format!(
                "#{} {} ({:?}): {}",
                job.job_id, job.label, job.status, job.summary
            ));
        }
    }
}
