// Don't want my model to depend on egui to much, so we create a 'boundary'/'proxy' for egui::Pos2
use egui;

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Pos {
    pub x: f32,
    pub y: f32,
}

impl Pos {
    pub fn new(x: f32, y: f32) -> Pos {
        Pos { x, y }
    }
}

impl From<&egui::Pos2> for Pos {
    fn from(pos: &egui::Pos2) -> Pos {
        Pos::new(pos.x, pos.y)
    }
}

impl From<&Pos> for egui::Pos2 {
    fn from(pos: &Pos) -> egui::Pos2 {
        egui::pos2(pos.x, pos.y)
    }
}

impl From<Pos> for egui::Vec2 {
    fn from(pos: Pos) -> egui::Vec2 {
        egui::vec2(pos.x, pos.y)
    }
}
