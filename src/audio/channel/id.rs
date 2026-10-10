// Generated from data/channel/id.json by scripts/generate_channels.rb.
// Do not edit; run rake channels:generate instead.

use strum_macros::{AsRefStr, EnumIter, EnumString};

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, EnumString, AsRefStr)]
#[repr(u32)]
pub enum Id {
    Left = 0,
    Right = 1,
    Center = 2,
    LFE = 3,
    LeftSurround = 4,
    RightSurround = 5,
    CenterSurround = 6,
    LeftBack = 7,
    RightBack = 8,
    HeightLeft = 9,
    HeightRight = 10,
    HeightCenter = 11,
    Top = 12,
    HeightLeftSurround = 13,
    HeightRightSurround = 14,
    HeightCenterSurround = 15,
    HeightLeftBack = 16,
    HeightRightBack = 17,
    LeftCenter = 18,
    RightCenter = 19,
    LFE2 = 20,
    BottomLeft = 21,
    BottomRight = 22,
    BottomCenter = 23,
    BottomLeftSurround = 24,
    BottomRightSurround = 25,
    LeftWide = 26,
    RightWide = 27,
    TopLeft = 28,
    TopRight = 29,
    Mono = 30,
}

impl Id {
    pub fn short_name(&self) -> &'static str {
        match self {
            Id::Left => "L",
            Id::Right => "R",
            Id::Center => "C",
            Id::LFE => "LFE",
            Id::LeftSurround => "Ls",
            Id::RightSurround => "Rs",
            Id::CenterSurround => "Cs",
            Id::LeftBack => "Lb",
            Id::RightBack => "Rb",
            Id::HeightLeft => "HL",
            Id::HeightRight => "HR",
            Id::HeightCenter => "HC",
            Id::Top => "T",
            Id::HeightLeftSurround => "HLs",
            Id::HeightRightSurround => "HRs",
            Id::HeightCenterSurround => "HCs",
            Id::HeightLeftBack => "HLb",
            Id::HeightRightBack => "HRb",
            Id::LeftCenter => "LC",
            Id::RightCenter => "RC",
            Id::LFE2 => "LFE2",
            Id::BottomLeft => "BL",
            Id::BottomRight => "BR",
            Id::BottomCenter => "BC",
            Id::BottomLeftSurround => "BLs",
            Id::BottomRightSurround => "BRs",
            Id::LeftWide => "Lw",
            Id::RightWide => "Rw",
            Id::TopLeft => "TL",
            Id::TopRight => "TR",
            Id::Mono => "M",
        }
    }

    /// Long name: same as enum variant name.
    pub fn long_name(&self) -> &str {
        self.as_ref()
    }
}

#[cfg(test)]
#[path = "id_tests.rs"]
mod tests;
