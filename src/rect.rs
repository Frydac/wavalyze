// use anyhow::{ensure, Result};
use crate::pos::Pos;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Rect {
    pub min: Point,
    pub max: Point,
}

impl Rect {
    pub fn new(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> Self {
        Self {
            min: Point { x: min_x, y: min_y },
            max: Point { x: max_x, y: max_y },
        }
    }

    pub fn from_min_size(min_x: f32, min_y: f32, width: f32, height: f32) -> Self {
        Self::new(min_x, min_y, min_x + width, min_y + height)
    }

    pub fn width(&self) -> f32 {
        self.max.x - self.min.x
    }

    pub fn height(&self) -> f32 {
        self.max.y - self.min.y
    }

    pub fn expand_y(&mut self, v: f32) -> &mut Self {
        self.min.y -= v;
        self.max.y += v;
        self
    }

    pub fn expand_y_rel(&mut self, p: f32) -> &mut Self {
        let height = self.height();
        self.min.y -= height * p;
        self.max.y += height * p;
        self
    }

    pub fn contains_x(self, pos: Pos) -> bool {
        self.min.x <= pos.x && pos.x <= self.max.x
    }
    pub fn contains_y(self, pos: Pos) -> bool {
        self.min.y <= pos.y && pos.y <= self.max.y
    }
    pub fn contains(self, pos: Pos) -> bool {
        self.contains_x(pos) && self.contains_y(pos)
    }

    pub fn top(&self) -> f32 {
        self.min.y
    }

    pub fn bottom(&self) -> f32 {
        self.max.y
    }

    pub fn left(&self) -> f32 {
        self.min.x
    }

    pub fn right(&self) -> f32 {
        self.max.x
    }

    pub fn center(&self) -> Pos {
        Pos::new(self.left() + self.width() / 2.0, self.top() + self.height() / 2.0)
    }
}

// Conversion traits from and to egui::Rect
impl From<Rect> for egui::Rect {
    fn from(rect: Rect) -> Self {
        egui::Rect::from_min_max(egui::pos2(rect.min.x, rect.min.y), egui::pos2(rect.max.x, rect.max.y))
    }
}

impl From<egui::Rect> for Rect {
    fn from(rect: egui::Rect) -> Self {
        Self {
            min: Point {
                x: rect.min.x,
                y: rect.min.y,
            },
            max: Point {
                x: rect.max.x,
                y: rect.max.y,
            },
        }
    }
}
