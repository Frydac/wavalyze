use bitflags::bitflags;

use super::Id;

// TODO: move to channel::Layout ?

bitflags! {
    #[derive(Debug)]
    pub struct Layout: u64 {
        const NONE = 0;
        const LEFT = 1 << Id::Left as u64;
        const CENTER = 1 << Id::Center as u64;
        const RIGHT = 1 << Id::Right as u64;
        const LFE = 1 << Id::LFE as u64;
        const LEFT_SURROUND = 1 << Id::LeftSurround as u64;
        const RIGHT_SURROUND = 1 << Id::RightSurround as u64;
        const CENTER_SURROUND = 1 << Id::CenterSurround as u64;
        const LEFT_BACK = 1 << Id::LeftBack as u64;
        const RIGHT_BACK = 1 << Id::RightBack as u64;
        const HEIGHT_LEFT = 1 << Id::HeightLeft as u64;
        const HEIGHT_RIGHT = 1 << Id::HeightRight as u64;
        const HEIGHT_CENTER = 1 << Id::HeightCenter as u64;
        const TOP = 1 << Id::Top as u64;
        const HEIGHT_LEFT_SURROUND = 1 << Id::HeightLeftSurround as u64;
        const HEIGHT_RIGHT_SURROUND = 1 << Id::HeightRightSurround as u64;
        const HEIGHT_CENTER_SURROUND = 1 << Id::HeightCenterSurround as u64;
        const HEIGHT_LEFT_BACK = 1 << Id::HeightLeftBack as u64;
        const HEIGHT_RIGHT_BACK = 1 << Id::HeightRightBack as u64;
        const LEFT_CENTER = 1 << Id::LeftCenter as u64;
        const RIGHT_CENTER = 1 << Id::RightCenter as u64;
        const LFE2 = 1 << Id::LFE2 as u64;
        const BOTTOM_LEFT = 1 << Id::BottomLeft as u64;
        const BOTTOM_RIGHT = 1 << Id::BottomRight as u64;
        const BOTTOM_CENTER = 1 << Id::BottomCenter as u64;
        const BOTTOM_LEFT_SURROUND = 1 << Id::BottomLeftSurround as u64;
        const BOTTOM_RIGHT_SURROUND = 1 << Id::BottomRightSurround as u64;
        const LEFT_WIDE = 1 << Id::LeftWide as u64;
        const RIGHT_WIDE = 1 << Id::RightWide as u64;
        const TOP_LEFT = 1 << Id::TopLeft as u64;
        const TOP_RIGHT = 1 << Id::TopRight as u64;
        const MONO = 1 << Id::Mono as u64;

        const LOWER_FRONTS = Self::LEFT.bits() | Self::RIGHT.bits();
        const LOWER_SURROUNDS = Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits();
        const LOWER_BACKS = Self::LEFT_BACK.bits() | Self::RIGHT_BACK.bits();
        const LOWER_WIDES = Self::LEFT_WIDE.bits() | Self::RIGHT_WIDE.bits();
        const LOWER_CORNERS = Self::LOWER_FRONTS.bits() | Self::LOWER_SURROUNDS.bits();
        const LOWER_LAYER = Self::LOWER_CORNERS.bits() | Self::LOWER_BACKS.bits() | Self::CENTER.bits() | Self::CENTER_SURROUND.bits() | Self::LEFT_CENTER.bits() | Self::RIGHT_CENTER.bits();
        const HEIGHT_FRONTS = Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits();
        const HEIGHT_SURROUNDS = Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits();
        const HEIGHT_BACKS = Self::HEIGHT_LEFT_BACK.bits() | Self::HEIGHT_RIGHT_BACK.bits();
        const HEIGHT_CORNERS = Self::HEIGHT_FRONTS.bits() | Self::HEIGHT_SURROUNDS.bits();
        const HEIGHT_LAYER = Self::HEIGHT_CORNERS.bits() | Self::HEIGHT_BACKS.bits() | Self::HEIGHT_CENTER.bits() | Self::HEIGHT_CENTER_SURROUND.bits();
        const BOTTOM_FRONTS = Self::BOTTOM_LEFT.bits() | Self::BOTTOM_RIGHT.bits();
        const BOTTOM_SURROUNDS = Self::BOTTOM_LEFT_SURROUND.bits() | Self::BOTTOM_RIGHT_SURROUND.bits();
        const FRONT_CENTERS = Self::LEFT_CENTER.bits() | Self::RIGHT_CENTER.bits();
        const TOP_LAYER = Self::TOP.bits() | Self::TOP_LEFT.bits() | Self::TOP_RIGHT.bits();
        const STEREO_TOP = Self::TOP_LEFT.bits() | Self::TOP_RIGHT.bits();
        const MONO_TOP = Self::TOP.bits();
        const LAYOUT_0D_ONLY = Self::LFE.bits() | Self::LFE2.bits() | Self::MONO.bits();
        const LAYOUT_1D_ONLY = Self::LOWER_FRONTS.bits();
        const LAYOUT_2D_ONLY = Self::CENTER.bits() | Self::LOWER_SURROUNDS.bits() | Self::CENTER_SURROUND.bits() | Self::LOWER_BACKS.bits() | Self::FRONT_CENTERS.bits() | Self::LOWER_WIDES.bits();
        const LAYOUT_3D_ONLY = Self::HEIGHT_FRONTS.bits() | Self::HEIGHT_CENTER.bits() | Self::HEIGHT_SURROUNDS.bits() | Self::HEIGHT_CENTER_SURROUND.bits() | Self::HEIGHT_BACKS.bits() | Self::TOP.bits() | Self::STEREO_TOP.bits() | Self::BOTTOM_FRONTS.bits() | Self::BOTTOM_CENTER.bits() | Self::BOTTOM_SURROUNDS.bits();

        const LAYOUT_0_1 = Self::LFE.bits();
        const LAYOUT_1_0 = Self::CENTER.bits();
        const LAYOUT_1_1 = Self::CENTER.bits() | Self::LFE.bits();
        const LAYOUT_2_0 = Self::LEFT.bits() | Self::RIGHT.bits();
        const LAYOUT_2_1 = Self::LAYOUT_2_0.bits() | Self::LFE.bits();
        const LAYOUT_3_0 = Self::LAYOUT_2_0.bits() | Self::CENTER.bits();
        const LAYOUT_3_1 = Self::LAYOUT_3_0.bits() | Self::LFE.bits();
        const LAYOUT_4_0 = Self::LAYOUT_2_0.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits();
        const LAYOUT_4_1 = Self::LAYOUT_4_0.bits() | Self::LFE.bits();
        const LAYOUT_5_0 = Self::LAYOUT_3_0.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits();
        const LAYOUT_5_1 = Self::LAYOUT_5_0.bits() | Self::LFE.bits();
        const LAYOUT_6_0 = Self::LAYOUT_5_0.bits() | Self::CENTER_SURROUND.bits();
        const LAYOUT_6_1 = Self::LAYOUT_6_0.bits() | Self::LFE.bits();
        const LAYOUT_7_0 = Self::LAYOUT_5_0.bits() | Self::LEFT_BACK.bits() | Self::RIGHT_BACK.bits();
        const LAYOUT_7_1 = Self::LAYOUT_7_0.bits() | Self::LFE.bits();
        const LAYOUT_7_0_NO_C = Self::LAYOUT_7_0.bits() & !Self::CENTER.bits();
        const LAYOUT_7_1_NO_C = Self::LAYOUT_7_1.bits() & !Self::CENTER.bits();

        // --- Height layouts ---
        const LAYOUT_2_0_2H = Self::LAYOUT_2_0.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits();
        const LAYOUT_2_1_2H = Self::LAYOUT_2_1.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits();
        const LAYOUT_3_0_2H = Self::LAYOUT_3_0.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits();
        const LAYOUT_3_1_2H = Self::LAYOUT_3_1.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits();
        const LAYOUT_3_0_3H = Self::LAYOUT_3_0_2H.bits() | Self::HEIGHT_CENTER.bits();
        const LAYOUT_3_1_3H = Self::LAYOUT_3_1_2H.bits() | Self::HEIGHT_CENTER.bits();
        const LAYOUT_4_0_2H = Self::LAYOUT_4_0.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits();
        const LAYOUT_4_1_2H = Self::LAYOUT_4_1.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits();
        const LAYOUT_4_0_4H = Self::LAYOUT_4_0_2H.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits();
        const LAYOUT_4_1_4H = Self::LAYOUT_4_1_2H.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits();
        const LAYOUT_4_0_5H = Self::LAYOUT_4_0_4H.bits() | Self::HEIGHT_CENTER.bits();
        const LAYOUT_4_1_5H = Self::LAYOUT_4_1_4H.bits() | Self::HEIGHT_CENTER.bits();
        const LAYOUT_5_0_2H = Self::LAYOUT_5_0.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits();
        const LAYOUT_5_1_2H = Self::LAYOUT_5_1.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits();
        const LAYOUT_5_0_4H = Self::LAYOUT_5_0.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits();
        const LAYOUT_5_1_4H = Self::LAYOUT_5_1.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits();
        const LAYOUT_5_0_5H = Self::LAYOUT_5_0_4H.bits() | Self::HEIGHT_CENTER.bits();
        const LAYOUT_5_1_5H = Self::LAYOUT_5_1_4H.bits() | Self::HEIGHT_CENTER.bits();

        // --- 7 channels with heights ---
        const LAYOUT_7_0_2H = Self::LAYOUT_7_0.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits();
        const LAYOUT_7_1_2H = Self::LAYOUT_7_1.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits();
        const LAYOUT_7_0_4H = Self::LAYOUT_7_0_2H.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits();
        const LAYOUT_7_1_4H = Self::LAYOUT_7_1_2H.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits();
        const LAYOUT_7_0_5H = Self::LAYOUT_7_0_4H.bits() | Self::HEIGHT_CENTER.bits();
        const LAYOUT_7_1_5H = Self::LAYOUT_7_1_4H.bits() | Self::HEIGHT_CENTER.bits();

        // --- With Top Layer ---
        const LAYOUT_4_0_4H_1T = Self::LAYOUT_4_0_4H.bits() | Self::TOP.bits();
        const LAYOUT_4_1_4H_1T = Self::LAYOUT_4_1_4H.bits() | Self::TOP.bits();
        const LAYOUT_4_0_5H_1T = Self::LAYOUT_4_0_5H.bits() | Self::TOP.bits();
        const LAYOUT_4_1_5H_1T = Self::LAYOUT_4_1_5H.bits() | Self::TOP.bits();
        const LAYOUT_5_0_4H_1T = Self::LAYOUT_5_0_4H.bits() | Self::TOP.bits();
        const LAYOUT_5_1_4H_1T = Self::LAYOUT_5_1_4H.bits() | Self::TOP.bits();
        const LAYOUT_5_0_5H_1T = Self::LAYOUT_5_0_5H.bits() | Self::TOP.bits();
        const LAYOUT_5_1_5H_1T = Self::LAYOUT_5_1_5H.bits() | Self::TOP.bits();
        const LAYOUT_7_0_4H_1T = Self::LAYOUT_7_0_4H.bits() | Self::TOP.bits();
        const LAYOUT_7_1_4H_1T = Self::LAYOUT_7_1_4H.bits() | Self::TOP.bits();
        const LAYOUT_7_0_5H_1T = Self::LAYOUT_7_0_5H.bits() | Self::TOP.bits();
        const LAYOUT_7_1_5H_1T = Self::LAYOUT_7_1_5H.bits() | Self::TOP.bits();

        // --- Synonyms ---
        const LAYOUT_TESTMIX1_0 = Self::LAYOUT_1_0.bits();
        const LAYOUT_AURO222 = Self::LAYOUT_4_0_2H.bits();
        const LAYOUT_AURO8_0 = Self::LAYOUT_4_0_4H.bits();
        const LAYOUT_AURO9_0 = Self::LAYOUT_5_0_4H.bits();
        const LAYOUT_AURO9_1 = Self::LAYOUT_5_1_4H.bits();
        const LAYOUT_AURO11_0_74 = Self::LAYOUT_7_0_4H.bits();
        const LAYOUT_AURO11_1_74 = Self::LAYOUT_7_1_4H.bits();
        const LAYOUT_AURO10_0 = Self::LAYOUT_5_0_4H_1T.bits();
        const LAYOUT_AURO10_1 = Self::LAYOUT_5_1_4H_1T.bits();
        const LAYOUT_AURO11_0_551 = Self::LAYOUT_5_0_5H_1T.bits();
        const LAYOUT_AURO11_1_551 = Self::LAYOUT_5_1_5H_1T.bits();
        const LAYOUT_AURO12_0 = Self::LAYOUT_7_0_4H_1T.bits();
        const LAYOUT_AURO12_1 = Self::LAYOUT_7_1_4H_1T.bits();
        const LAYOUT_AURO13_0 = Self::LAYOUT_7_0_5H_1T.bits();
        const LAYOUT_AURO13_1 = Self::LAYOUT_7_1_5H_1T.bits();

        // --- Special layouts (Test / NHK / Cube) ---
        const LAYOUT_LCRS = Self::LEFT.bits() | Self::RIGHT.bits() | Self::CENTER.bits() | Self::CENTER_SURROUND.bits();
        const LAYOUT_TESTMIX2_0 = Self::CENTER.bits() | Self::HEIGHT_CENTER.bits();
        const LAYOUT_TESTMIX3_0 = Self::CENTER.bits() | Self::HEIGHT_CENTER.bits() | Self::TOP.bits();
        const LAYOUT_NHK_22_2 = Self::LEFT.bits() | Self::RIGHT.bits() | Self::CENTER.bits() | Self::LFE.bits()
            | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::CENTER_SURROUND.bits()
            | Self::LEFT_BACK.bits() | Self::RIGHT_BACK.bits()
            | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits() | Self::HEIGHT_CENTER.bits()
            | Self::TOP.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits() | Self::HEIGHT_CENTER_SURROUND.bits()
            | Self::HEIGHT_LEFT_BACK.bits() | Self::HEIGHT_RIGHT_BACK.bits()
            | Self::LEFT_CENTER.bits() | Self::RIGHT_CENTER.bits()
            | Self::LFE2.bits()
            | Self::BOTTOM_LEFT.bits() | Self::BOTTOM_RIGHT.bits() | Self::BOTTOM_CENTER.bits();
        const LAYOUT_NHK_22_0 = Self::LAYOUT_NHK_22_2.bits() & !Self::LFE.bits();
        const LAYOUT_CUBE = Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits()
            | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits()
            | Self::BOTTOM_LEFT.bits() | Self::BOTTOM_RIGHT.bits()
            | Self::BOTTOM_LEFT_SURROUND.bits() | Self::BOTTOM_RIGHT_SURROUND.bits();

        // --- Extended 2T / 5H layouts ---
        const LAYOUT_5_1_4H_2T = Self::LAYOUT_5_1_4H.bits() | Self::TOP_LEFT.bits() | Self::TOP_RIGHT.bits();
        const LAYOUT_7_1_4H_2T = Self::LAYOUT_7_1_4H.bits() | Self::TOP_LEFT.bits() | Self::TOP_RIGHT.bits();
        const LAYOUT_9_1_4H_2T = Self::LAYOUT_7_1_4H_2T.bits() | Self::LEFT_WIDE.bits() | Self::RIGHT_WIDE.bits();
        const LAYOUT_5_1_5H_2T = Self::LAYOUT_5_1_5H.bits() | Self::TOP_LEFT.bits() | Self::TOP_RIGHT.bits();
        const LAYOUT_7_1_5H_2T = Self::LAYOUT_7_1_5H.bits() | Self::TOP_LEFT.bits() | Self::TOP_RIGHT.bits();

        // --- Final top & height variants for AURO 13+ ---
        const LAYOUT_AURO13_1_2T = Self::LAYOUT_7_1_5H_2T.bits();
        const LAYOUT_AURO12_1_2T = Self::LAYOUT_7_1_4H_1T.bits() | Self::TOP_LEFT.bits() | Self::TOP_RIGHT.bits();
        const LAYOUT_AURO10_1_2T = Self::LAYOUT_5_1_4H_1T.bits() | Self::TOP_LEFT.bits() | Self::TOP_RIGHT.bits();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_channel() {
        let mask = Layout::LEFT;
        assert_eq!(mask.bits(), 1 << 0);
    }

    #[test]
    fn test_combine_channels() {
        let mask = Layout::LEFT | Layout::RIGHT;
        assert_eq!(mask.bits(), (1 << 0) | (1 << 1));
    }

    #[test]
    fn test_stereo_layout() {
        let stereo = Layout::LEFT | Layout::RIGHT;
        assert!(stereo.contains(Layout::LEFT));
        assert!(stereo.contains(Layout::RIGHT));
        assert!(!stereo.contains(Layout::CENTER));
    }

    #[test]
    fn test_layout_5_1() {
        let layout = Layout::LAYOUT_5_1;
        dbg!(&layout);
        assert!(layout.contains(Layout::LEFT));
        assert!(layout.contains(Layout::RIGHT));
        assert!(layout.contains(Layout::CENTER));
        assert!(layout.contains(Layout::LFE));
        assert!(layout.contains(Layout::LEFT_SURROUND));
        assert!(layout.contains(Layout::RIGHT_SURROUND));
        assert!(!layout.contains(Layout::LEFT_BACK));
    }
}
