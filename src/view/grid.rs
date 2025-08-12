use egui::{RichText, Ui};

pub struct KeyValueGrid<'a> {
    id_source: &'a str,
    key_col_width: Option<f32>,
    rows: Vec<(RichText, RichText)>,
}

impl<'a> KeyValueGrid<'a> {
    pub fn new(id_source: &'a str) -> Self {
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
        self.rows.push((key.into(), value.into()));
        self
    }

    pub fn show(self, ui: &mut Ui) {
        let mut grid = egui::Grid::new(self.id_source).num_columns(2).spacing([10.0, 4.0]).striped(true);

        if let Some(width) = self.key_col_width {
            grid = grid.min_col_width(width);
        }

        grid.show(ui, |ui| {
            for (key, value) in self.rows {
                ui.label(key);
                ui.label(value);
                ui.end_row();
            }
        });
    }
}
