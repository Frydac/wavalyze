use egui::{RichText, Ui};

pub struct KeyValueGrid {
    id_source: u64,
    key_col_width: Option<f32>,
    rows: Vec<(RichText, RichText, Option<RichText>)>,
}

impl KeyValueGrid {
    pub fn new(id_source: u64) -> Self {
        Self {
            id_source,
            key_col_width: None,
            rows: Vec::new(),
        }
    }

    pub fn key_col_width(mut self, width: f32) -> Self {
        self.key_col_width = Some(width);
        self
    }

    pub fn row(&mut self, key: impl Into<RichText>, value: impl Into<RichText>) -> &mut Self {
        self.rows.push((key.into(), value.into(), None));
        self
    }

    pub fn row_with_hover(
        &mut self,
        key: impl Into<RichText>,
        value: impl Into<RichText>,
        hover: impl Into<RichText>,
    ) -> &mut Self {
        self.rows
            .push((key.into(), value.into(), Some(hover.into())));
        self
    }

    pub fn show(self, ui: &mut Ui) {
        let mut grid = egui::Grid::new(egui::Id::new(self.id_source))
            .num_columns(2)
            .spacing([10.0, 4.0])
            .striped(true);

        if let Some(width) = self.key_col_width {
            grid = grid.min_col_width(width);
        }

        grid.show(ui, |ui| {
            for (key, value, hover) in self.rows {
                ui.label(key);
                let response = ui.label(value);
                if let Some(hover) = hover {
                    response.on_hover_text(hover);
                }
                ui.end_row();
            }
        });
    }

    // pub fn ui(&mut self, ui: &mut egui::Ui, add_contents: impl FnOnce(&mut Self)) {
    //     ui.group(|ui| {
    //         ui.vertical(|ui| {
    //             ui.heading("Grid");
    //             ui.separator();
    //             self.show(ui, add_contents);
    //         });
    //     });
    // }
}
