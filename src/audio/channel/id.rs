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
            Id::LeftCenter => "Lc",
            Id::RightCenter => "Rc",
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

    /// Long name: same as enum variant name
    pub fn long_name(&self) -> &str {
        self.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator;

    #[test]
    fn test_short_names() {
        assert_eq!(Id::Left.short_name(), "L");
        assert_eq!(Id::Right.short_name(), "R");
        assert_eq!(Id::Center.short_name(), "C");
        assert_eq!(Id::LFE.short_name(), "LFE");
        assert_eq!(Id::Mono.short_name(), "M");
    }

    #[test]
    fn test_long_names() {
        assert_eq!(Id::Left.long_name(), "Left");
        assert_eq!(Id::Right.long_name(), "Right");
        assert_eq!(Id::Center.long_name(), "Center");
        assert_eq!(Id::LFE.long_name(), "LFE");
        assert_eq!(Id::Mono.long_name(), "Mono");
    }

    #[test]
    fn test_iteration_covers_all() {
        // Just check that we can iterate over all variants without panicking
        let variants: Vec<_> = Id::iter().collect();
        assert!(variants.contains(&Id::Left));
        assert!(variants.contains(&Id::Mono));
        // and that the numeric repr works
        assert_eq!(Id::Left as u32, 0);
        assert_eq!(Id::Mono as u32, 30);
    }
}
