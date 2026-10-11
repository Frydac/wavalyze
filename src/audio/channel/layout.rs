// Generated from data/channel/{id,layout,supplemental}.json by scripts/generate_channels.rb.
// Do not edit; run rake channels:generate instead.

use super::Id;
use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Layout: u64 {
        const NONE = 0;
        const LEFT = 1 << Id::Left as u64;
        const RIGHT = 1 << Id::Right as u64;
        const CENTER = 1 << Id::Center as u64;
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
        const LAYOUT_0_1 = Self::LFE.bits();
        const LAYOUT_1_0 = Self::CENTER.bits();
        const LAYOUT_TESTMIX1_0 = Self::LAYOUT_1_0.bits();
        const LAYOUT_1_1 = Self::CENTER.bits() | Self::LFE.bits();
        const LAYOUT_2_0 = Self::LEFT.bits() | Self::RIGHT.bits();
        const LAYOUT_2_0_2H = Self::LEFT.bits() | Self::RIGHT.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits();
        const LAYOUT_2_1 = Self::LEFT.bits() | Self::RIGHT.bits() | Self::LFE.bits();
        const LAYOUT_2_1_2H = Self::LEFT.bits() | Self::RIGHT.bits() | Self::LFE.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits();
        const LAYOUT_3_0 = Self::LEFT.bits() | Self::RIGHT.bits() | Self::CENTER.bits();
        const LAYOUT_3_0_2H = Self::LEFT.bits() | Self::RIGHT.bits() | Self::CENTER.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits();
        const LAYOUT_3_0_3H = Self::LEFT.bits() | Self::RIGHT.bits() | Self::CENTER.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits() | Self::HEIGHT_CENTER.bits();
        const LAYOUT_3_1 = Self::LEFT.bits() | Self::RIGHT.bits() | Self::CENTER.bits() | Self::LFE.bits();
        const LAYOUT_3_1_2H = Self::LEFT.bits() | Self::RIGHT.bits() | Self::CENTER.bits() | Self::LFE.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits();
        const LAYOUT_3_1_3H = Self::LEFT.bits() | Self::RIGHT.bits() | Self::CENTER.bits() | Self::LFE.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits() | Self::HEIGHT_CENTER.bits();
        const LAYOUT_4_0 = Self::LEFT.bits() | Self::RIGHT.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits();
        const LAYOUT_4_0_2H = Self::LEFT.bits() | Self::RIGHT.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits();
        const LAYOUT_AURO222 = Self::LAYOUT_4_0_2H.bits();
        const LAYOUT_4_0_4H = Self::LEFT.bits() | Self::RIGHT.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits();
        const LAYOUT_AURO8_0 = Self::LAYOUT_4_0_4H.bits();
        const LAYOUT_4_0_4H_1T = Self::LEFT.bits() | Self::RIGHT.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits() | Self::TOP.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits();
        const LAYOUT_4_0_5H = Self::LEFT.bits() | Self::RIGHT.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits() | Self::HEIGHT_CENTER.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits();
        const LAYOUT_4_0_5H_1T = Self::LEFT.bits() | Self::RIGHT.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits() | Self::HEIGHT_CENTER.bits() | Self::TOP.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits();
        const LAYOUT_4_1 = Self::LEFT.bits() | Self::RIGHT.bits() | Self::LFE.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits();
        const LAYOUT_4_1_2H = Self::LEFT.bits() | Self::RIGHT.bits() | Self::LFE.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits();
        const LAYOUT_4_1_4H = Self::LEFT.bits() | Self::RIGHT.bits() | Self::LFE.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits();
        const LAYOUT_AURO8_1 = Self::LAYOUT_4_1_4H.bits();
        const LAYOUT_4_1_4H_1T = Self::LEFT.bits() | Self::RIGHT.bits() | Self::LFE.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits() | Self::TOP.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits();
        const LAYOUT_4_1_5H = Self::LEFT.bits() | Self::RIGHT.bits() | Self::LFE.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits() | Self::HEIGHT_CENTER.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits();
        const LAYOUT_4_1_5H_1T = Self::LEFT.bits() | Self::RIGHT.bits() | Self::LFE.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits() | Self::HEIGHT_CENTER.bits() | Self::TOP.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits();
        const LAYOUT_5_0 = Self::LEFT.bits() | Self::RIGHT.bits() | Self::CENTER.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits();
        const LAYOUT_5_0_2H = Self::LEFT.bits() | Self::RIGHT.bits() | Self::CENTER.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits();
        const LAYOUT_5_0_4H = Self::LEFT.bits() | Self::RIGHT.bits() | Self::CENTER.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits();
        const LAYOUT_AURO9_0 = Self::LAYOUT_5_0_4H.bits();
        const LAYOUT_5_0_4H_1T = Self::LEFT.bits() | Self::RIGHT.bits() | Self::CENTER.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits() | Self::TOP.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits();
        const LAYOUT_AURO10_0 = Self::LAYOUT_5_0_4H_1T.bits();
        const LAYOUT_5_0_5H = Self::LEFT.bits() | Self::RIGHT.bits() | Self::CENTER.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits() | Self::HEIGHT_CENTER.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits();
        const LAYOUT_5_0_5H_1T = Self::LEFT.bits() | Self::RIGHT.bits() | Self::CENTER.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits() | Self::HEIGHT_CENTER.bits() | Self::TOP.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits();
        const LAYOUT_AURO11_0_551 = Self::LAYOUT_5_0_5H_1T.bits();
        const LAYOUT_5_1 = Self::LEFT.bits() | Self::RIGHT.bits() | Self::CENTER.bits() | Self::LFE.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits();
        const LAYOUT_5_1_2H = Self::LEFT.bits() | Self::RIGHT.bits() | Self::CENTER.bits() | Self::LFE.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits();
        const LAYOUT_5_1_4H = Self::LEFT.bits() | Self::RIGHT.bits() | Self::CENTER.bits() | Self::LFE.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits();
        const LAYOUT_AURO9_1 = Self::LAYOUT_5_1_4H.bits();
        const LAYOUT_5_1_4H_1T = Self::LEFT.bits() | Self::RIGHT.bits() | Self::CENTER.bits() | Self::LFE.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits() | Self::TOP.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits();
        const LAYOUT_AURO10_1 = Self::LAYOUT_5_1_4H_1T.bits();
        const LAYOUT_5_1_4H_2T = Self::LEFT.bits() | Self::RIGHT.bits() | Self::CENTER.bits() | Self::LFE.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits() | Self::TOP_LEFT.bits() | Self::TOP_RIGHT.bits();
        const LAYOUT_5_1_5H = Self::LEFT.bits() | Self::RIGHT.bits() | Self::CENTER.bits() | Self::LFE.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits() | Self::HEIGHT_CENTER.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits();
        const LAYOUT_5_1_5H_1T = Self::LEFT.bits() | Self::RIGHT.bits() | Self::CENTER.bits() | Self::LFE.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits() | Self::HEIGHT_CENTER.bits() | Self::TOP.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits();
        const LAYOUT_AURO11_1_551 = Self::LAYOUT_5_1_5H_1T.bits();
        const LAYOUT_5_1_5H_2T = Self::LEFT.bits() | Self::RIGHT.bits() | Self::CENTER.bits() | Self::LFE.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits() | Self::HEIGHT_CENTER.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits() | Self::TOP_LEFT.bits() | Self::TOP_RIGHT.bits();
        const LAYOUT_6_0 = Self::LEFT.bits() | Self::RIGHT.bits() | Self::CENTER.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::CENTER_SURROUND.bits();
        const LAYOUT_6_1 = Self::LEFT.bits() | Self::RIGHT.bits() | Self::CENTER.bits() | Self::LFE.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::CENTER_SURROUND.bits();
        const LAYOUT_7_0 = Self::LEFT.bits() | Self::RIGHT.bits() | Self::CENTER.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::LEFT_BACK.bits() | Self::RIGHT_BACK.bits();
        const LAYOUT_7_0_2H = Self::LEFT.bits() | Self::RIGHT.bits() | Self::CENTER.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::LEFT_BACK.bits() | Self::RIGHT_BACK.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits();
        const LAYOUT_7_0_2H_NO_C = Self::LEFT.bits() | Self::RIGHT.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::LEFT_BACK.bits() | Self::RIGHT_BACK.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits();
        const LAYOUT_7_0_4H = Self::LEFT.bits() | Self::RIGHT.bits() | Self::CENTER.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::LEFT_BACK.bits() | Self::RIGHT_BACK.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits();
        const LAYOUT_AURO11_0_74 = Self::LAYOUT_7_0_4H.bits();
        const LAYOUT_7_0_4H_1T = Self::LEFT.bits() | Self::RIGHT.bits() | Self::CENTER.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::LEFT_BACK.bits() | Self::RIGHT_BACK.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits() | Self::TOP.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits();
        const LAYOUT_AURO12_0 = Self::LAYOUT_7_0_4H_1T.bits();
        const LAYOUT_7_0_4H_1T_NO_C = Self::LEFT.bits() | Self::RIGHT.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::LEFT_BACK.bits() | Self::RIGHT_BACK.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits() | Self::TOP.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits();
        const LAYOUT_7_0_4H_NO_C = Self::LEFT.bits() | Self::RIGHT.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::LEFT_BACK.bits() | Self::RIGHT_BACK.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits();
        const LAYOUT_7_0_5H = Self::LEFT.bits() | Self::RIGHT.bits() | Self::CENTER.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::LEFT_BACK.bits() | Self::RIGHT_BACK.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits() | Self::HEIGHT_CENTER.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits();
        const LAYOUT_7_0_5H_1T = Self::LEFT.bits() | Self::RIGHT.bits() | Self::CENTER.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::LEFT_BACK.bits() | Self::RIGHT_BACK.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits() | Self::HEIGHT_CENTER.bits() | Self::TOP.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits();
        const LAYOUT_AURO13_0 = Self::LAYOUT_7_0_5H_1T.bits();
        const LAYOUT_7_0_5H_1T_NO_C = Self::LEFT.bits() | Self::RIGHT.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::LEFT_BACK.bits() | Self::RIGHT_BACK.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits() | Self::HEIGHT_CENTER.bits() | Self::TOP.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits();
        const LAYOUT_7_0_5H_NO_C = Self::LEFT.bits() | Self::RIGHT.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::LEFT_BACK.bits() | Self::RIGHT_BACK.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits() | Self::HEIGHT_CENTER.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits();
        const LAYOUT_7_0_NO_C = Self::LEFT.bits() | Self::RIGHT.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::LEFT_BACK.bits() | Self::RIGHT_BACK.bits();
        const LAYOUT_7_1 = Self::LEFT.bits() | Self::RIGHT.bits() | Self::CENTER.bits() | Self::LFE.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::LEFT_BACK.bits() | Self::RIGHT_BACK.bits();
        const LAYOUT_7_1_2H = Self::LEFT.bits() | Self::RIGHT.bits() | Self::CENTER.bits() | Self::LFE.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::LEFT_BACK.bits() | Self::RIGHT_BACK.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits();
        const LAYOUT_7_1_2H_NO_C = Self::LEFT.bits() | Self::RIGHT.bits() | Self::LFE.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::LEFT_BACK.bits() | Self::RIGHT_BACK.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits();
        const LAYOUT_7_1_4H = Self::LEFT.bits() | Self::RIGHT.bits() | Self::CENTER.bits() | Self::LFE.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::LEFT_BACK.bits() | Self::RIGHT_BACK.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits();
        const LAYOUT_AURO11_1_74 = Self::LAYOUT_7_1_4H.bits();
        const LAYOUT_7_1_4H_1T = Self::LEFT.bits() | Self::RIGHT.bits() | Self::CENTER.bits() | Self::LFE.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::LEFT_BACK.bits() | Self::RIGHT_BACK.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits() | Self::TOP.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits();
        const LAYOUT_AURO12_1 = Self::LAYOUT_7_1_4H_1T.bits();
        const LAYOUT_7_1_4H_1T_NO_C = Self::LEFT.bits() | Self::RIGHT.bits() | Self::LFE.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::LEFT_BACK.bits() | Self::RIGHT_BACK.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits() | Self::TOP.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits();
        const LAYOUT_7_1_4H_2T = Self::LEFT.bits() | Self::RIGHT.bits() | Self::CENTER.bits() | Self::LFE.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::LEFT_BACK.bits() | Self::RIGHT_BACK.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits() | Self::TOP_LEFT.bits() | Self::TOP_RIGHT.bits();
        const LAYOUT_7_1_4H_NO_C = Self::LEFT.bits() | Self::RIGHT.bits() | Self::LFE.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::LEFT_BACK.bits() | Self::RIGHT_BACK.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits();
        const LAYOUT_7_1_5H = Self::LEFT.bits() | Self::RIGHT.bits() | Self::CENTER.bits() | Self::LFE.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::LEFT_BACK.bits() | Self::RIGHT_BACK.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits() | Self::HEIGHT_CENTER.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits();
        const LAYOUT_7_1_5H_1T = Self::LEFT.bits() | Self::RIGHT.bits() | Self::CENTER.bits() | Self::LFE.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::LEFT_BACK.bits() | Self::RIGHT_BACK.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits() | Self::HEIGHT_CENTER.bits() | Self::TOP.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits();
        const LAYOUT_AURO13_1 = Self::LAYOUT_7_1_5H_1T.bits();
        const LAYOUT_7_1_5H_1T_NO_C = Self::LEFT.bits() | Self::RIGHT.bits() | Self::LFE.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::LEFT_BACK.bits() | Self::RIGHT_BACK.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits() | Self::HEIGHT_CENTER.bits() | Self::TOP.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits();
        const LAYOUT_7_1_5H_2T = Self::LEFT.bits() | Self::RIGHT.bits() | Self::CENTER.bits() | Self::LFE.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::LEFT_BACK.bits() | Self::RIGHT_BACK.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits() | Self::HEIGHT_CENTER.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits() | Self::TOP_LEFT.bits() | Self::TOP_RIGHT.bits();
        const LAYOUT_7_1_5H_NO_C = Self::LEFT.bits() | Self::RIGHT.bits() | Self::LFE.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::LEFT_BACK.bits() | Self::RIGHT_BACK.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits() | Self::HEIGHT_CENTER.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits();
        const LAYOUT_7_1_NO_C = Self::LEFT.bits() | Self::RIGHT.bits() | Self::LFE.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::LEFT_BACK.bits() | Self::RIGHT_BACK.bits();
        const LAYOUT_9_1_4H_2T = Self::LEFT.bits() | Self::RIGHT.bits() | Self::CENTER.bits() | Self::LFE.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::LEFT_BACK.bits() | Self::RIGHT_BACK.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits() | Self::LEFT_WIDE.bits() | Self::RIGHT_WIDE.bits() | Self::TOP_LEFT.bits() | Self::TOP_RIGHT.bits();
        const LAYOUT_9_1_5H_2T = Self::LEFT.bits() | Self::RIGHT.bits() | Self::CENTER.bits() | Self::LFE.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::LEFT_BACK.bits() | Self::RIGHT_BACK.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits() | Self::HEIGHT_CENTER.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits() | Self::LEFT_WIDE.bits() | Self::RIGHT_WIDE.bits() | Self::TOP_LEFT.bits() | Self::TOP_RIGHT.bits();
        const LAYOUT_CUBE = Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits() | Self::BOTTOM_LEFT.bits() | Self::BOTTOM_RIGHT.bits() | Self::BOTTOM_LEFT_SURROUND.bits() | Self::BOTTOM_RIGHT_SURROUND.bits();
        const LAYOUT_LCRS = Self::LEFT.bits() | Self::RIGHT.bits() | Self::CENTER.bits() | Self::CENTER_SURROUND.bits();
        const LAYOUT_NHK_22_0 = Self::LEFT.bits() | Self::RIGHT.bits() | Self::CENTER.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::CENTER_SURROUND.bits() | Self::LEFT_BACK.bits() | Self::RIGHT_BACK.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits() | Self::HEIGHT_CENTER.bits() | Self::TOP.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits() | Self::HEIGHT_CENTER_SURROUND.bits() | Self::HEIGHT_LEFT_BACK.bits() | Self::HEIGHT_RIGHT_BACK.bits() | Self::LEFT_CENTER.bits() | Self::RIGHT_CENTER.bits() | Self::BOTTOM_LEFT.bits() | Self::BOTTOM_RIGHT.bits() | Self::BOTTOM_CENTER.bits();
        const LAYOUT_NHK_22_2 = Self::LEFT.bits() | Self::RIGHT.bits() | Self::CENTER.bits() | Self::LFE.bits() | Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits() | Self::CENTER_SURROUND.bits() | Self::LEFT_BACK.bits() | Self::RIGHT_BACK.bits() | Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits() | Self::HEIGHT_CENTER.bits() | Self::TOP.bits() | Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits() | Self::HEIGHT_CENTER_SURROUND.bits() | Self::HEIGHT_LEFT_BACK.bits() | Self::HEIGHT_RIGHT_BACK.bits() | Self::LEFT_CENTER.bits() | Self::RIGHT_CENTER.bits() | Self::LFE2.bits() | Self::BOTTOM_LEFT.bits() | Self::BOTTOM_RIGHT.bits() | Self::BOTTOM_CENTER.bits();
        const LAYOUT_TESTMIX2_0 = Self::CENTER.bits() | Self::HEIGHT_CENTER.bits();
        const LAYOUT_TESTMIX3_0 = Self::CENTER.bits() | Self::HEIGHT_CENTER.bits() | Self::TOP.bits();
        const BOTTOM_FRONTS = Self::BOTTOM_LEFT.bits() | Self::BOTTOM_RIGHT.bits();
        const BOTTOM_SURROUNDS = Self::BOTTOM_LEFT_SURROUND.bits() | Self::BOTTOM_RIGHT_SURROUND.bits();
        const FRONT_CENTERS = Self::LEFT_CENTER.bits() | Self::RIGHT_CENTER.bits();
        const HEIGHT_BACKS = Self::HEIGHT_LEFT_BACK.bits() | Self::HEIGHT_RIGHT_BACK.bits();
        const HEIGHT_FRONTS = Self::HEIGHT_LEFT.bits() | Self::HEIGHT_RIGHT.bits();
        const HEIGHT_SURROUNDS = Self::HEIGHT_LEFT_SURROUND.bits() | Self::HEIGHT_RIGHT_SURROUND.bits();
        const HEIGHT_CORNERS = Self::HEIGHT_FRONTS.bits() | Self::HEIGHT_SURROUNDS.bits();
        const HEIGHT_LAYER = Self::HEIGHT_BACKS.bits() | Self::HEIGHT_CENTER.bits() | Self::HEIGHT_CENTER_SURROUND.bits() | Self::HEIGHT_CORNERS.bits();
        const LAYOUT_0D_ONLY = Self::LFE.bits() | Self::LFE2.bits() | Self::MONO.bits();
        const LOWER_FRONTS = Self::LEFT.bits() | Self::RIGHT.bits();
        const LAYOUT_1D_ONLY = Self::LOWER_FRONTS.bits();
        const LOWER_BACKS = Self::LEFT_BACK.bits() | Self::RIGHT_BACK.bits();
        const LOWER_SURROUNDS = Self::LEFT_SURROUND.bits() | Self::RIGHT_SURROUND.bits();
        const LOWER_WIDES = Self::LEFT_WIDE.bits() | Self::RIGHT_WIDE.bits();
        const LAYOUT_2D_ONLY = Self::CENTER.bits() | Self::CENTER_SURROUND.bits() | Self::FRONT_CENTERS.bits() | Self::LOWER_BACKS.bits() | Self::LOWER_SURROUNDS.bits() | Self::LOWER_WIDES.bits();
        const STEREO_TOP = Self::TOP_LEFT.bits() | Self::TOP_RIGHT.bits();
        const LAYOUT_3D_ONLY = Self::BOTTOM_CENTER.bits() | Self::BOTTOM_FRONTS.bits() | Self::BOTTOM_SURROUNDS.bits() | Self::HEIGHT_BACKS.bits() | Self::HEIGHT_CENTER.bits() | Self::HEIGHT_CENTER_SURROUND.bits() | Self::HEIGHT_FRONTS.bits() | Self::HEIGHT_SURROUNDS.bits() | Self::STEREO_TOP.bits() | Self::TOP.bits();
        const LAYOUT_AURO10_1_2T = Self::LAYOUT_5_1_4H_1T.bits() | Self::TOP_LEFT.bits() | Self::TOP_RIGHT.bits();
        const LAYOUT_AURO12_1_2T = Self::LAYOUT_7_1_4H_1T.bits() | Self::TOP_LEFT.bits() | Self::TOP_RIGHT.bits();
        const LAYOUT_AURO13_1_2T = Self::LAYOUT_7_1_5H_2T.bits();
        const LOWER_CORNERS = Self::LOWER_FRONTS.bits() | Self::LOWER_SURROUNDS.bits();
        const LOWER_LAYER = Self::CENTER.bits() | Self::CENTER_SURROUND.bits() | Self::LEFT_CENTER.bits() | Self::LOWER_BACKS.bits() | Self::LOWER_CORNERS.bits() | Self::RIGHT_CENTER.bits();
        const MONO_TOP = Self::TOP.bits();
        const TOP_LAYER = Self::TOP.bits() | Self::TOP_LEFT.bits() | Self::TOP_RIGHT.bits();
    }
}

impl Layout {
    /// Distinct layouts from the imported catalog, ordered by mask.
    pub fn catalog() -> &'static [Self] {
        const CATALOG: &[Layout] = &[
            Layout::from_bits_retain(0x3),
            Layout::from_bits_retain(0x4),
            Layout::from_bits_retain(0x7),
            Layout::from_bits_retain(0x8),
            Layout::from_bits_retain(0xb),
            Layout::from_bits_retain(0xc),
            Layout::from_bits_retain(0xf),
            Layout::from_bits_retain(0x33),
            Layout::from_bits_retain(0x37),
            Layout::from_bits_retain(0x3b),
            Layout::from_bits_retain(0x3f),
            Layout::from_bits_retain(0x47),
            Layout::from_bits_retain(0x77),
            Layout::from_bits_retain(0x7f),
            Layout::from_bits_retain(0x1b3),
            Layout::from_bits_retain(0x1b7),
            Layout::from_bits_retain(0x1bb),
            Layout::from_bits_retain(0x1bf),
            Layout::from_bits_retain(0x603),
            Layout::from_bits_retain(0x607),
            Layout::from_bits_retain(0x60b),
            Layout::from_bits_retain(0x60f),
            Layout::from_bits_retain(0x633),
            Layout::from_bits_retain(0x637),
            Layout::from_bits_retain(0x63b),
            Layout::from_bits_retain(0x63f),
            Layout::from_bits_retain(0x7b3),
            Layout::from_bits_retain(0x7b7),
            Layout::from_bits_retain(0x7bb),
            Layout::from_bits_retain(0x7bf),
            Layout::from_bits_retain(0x804),
            Layout::from_bits_retain(0xe07),
            Layout::from_bits_retain(0xe0f),
            Layout::from_bits_retain(0x1804),
            Layout::from_bits_retain(0x6633),
            Layout::from_bits_retain(0x6637),
            Layout::from_bits_retain(0x663b),
            Layout::from_bits_retain(0x663f),
            Layout::from_bits_retain(0x67b3),
            Layout::from_bits_retain(0x67b7),
            Layout::from_bits_retain(0x67bb),
            Layout::from_bits_retain(0x67bf),
            Layout::from_bits_retain(0x6e33),
            Layout::from_bits_retain(0x6e37),
            Layout::from_bits_retain(0x6e3b),
            Layout::from_bits_retain(0x6e3f),
            Layout::from_bits_retain(0x6fb3),
            Layout::from_bits_retain(0x6fb7),
            Layout::from_bits_retain(0x6fbb),
            Layout::from_bits_retain(0x6fbf),
            Layout::from_bits_retain(0x7633),
            Layout::from_bits_retain(0x7637),
            Layout::from_bits_retain(0x763b),
            Layout::from_bits_retain(0x763f),
            Layout::from_bits_retain(0x77b3),
            Layout::from_bits_retain(0x77b7),
            Layout::from_bits_retain(0x77bb),
            Layout::from_bits_retain(0x77bf),
            Layout::from_bits_retain(0x7e33),
            Layout::from_bits_retain(0x7e37),
            Layout::from_bits_retain(0x7e3b),
            Layout::from_bits_retain(0x7e3f),
            Layout::from_bits_retain(0x7fb3),
            Layout::from_bits_retain(0x7fb7),
            Layout::from_bits_retain(0x7fbb),
            Layout::from_bits_retain(0x7fbf),
            Layout::from_bits_retain(0xeffff7),
            Layout::from_bits_retain(0xffffff),
            Layout::from_bits_retain(0x3606600),
            Layout::from_bits_retain(0x3000663f),
            Layout::from_bits_retain(0x300067bf),
            Layout::from_bits_retain(0x30006e3f),
            Layout::from_bits_retain(0x30006fbf),
            Layout::from_bits_retain(0x3c0067bf),
            Layout::from_bits_retain(0x3c006fbf),
        ];
        CATALOG
    }

    /// Catalog names and synonyms for an exact mask match, or an empty slice.
    pub fn names(&self) -> &'static [&'static str] {
        match self.bits() {
            0x3 => &["2.0"],
            0x4 => &["1.0", "TestMix1.0"],
            0x7 => &["3.0"],
            0x8 => &["0.1"],
            0xb => &["2.1"],
            0xc => &["1.1"],
            0xf => &["3.1"],
            0x33 => &["4.0"],
            0x37 => &["5.0"],
            0x3b => &["4.1"],
            0x3f => &["5.1"],
            0x47 => &["LCRS"],
            0x77 => &["6.0"],
            0x7f => &["6.1"],
            0x1b3 => &["7.0_no_C"],
            0x1b7 => &["7.0"],
            0x1bb => &["7.1_no_C"],
            0x1bf => &["7.1"],
            0x603 => &["2.0_2H"],
            0x607 => &["3.0_2H"],
            0x60b => &["2.1_2H"],
            0x60f => &["3.1_2H"],
            0x633 => &["4.0_2H", "Auro222"],
            0x637 => &["5.0_2H"],
            0x63b => &["4.1_2H"],
            0x63f => &["5.1_2H"],
            0x7b3 => &["7.0_2H_no_C"],
            0x7b7 => &["7.0_2H"],
            0x7bb => &["7.1_2H_no_C"],
            0x7bf => &["7.1_2H"],
            0x804 => &["TestMix2.0"],
            0xe07 => &["3.0_3H"],
            0xe0f => &["3.1_3H"],
            0x1804 => &["TestMix3.0"],
            0x6633 => &["4.0_4H", "Auro8.0"],
            0x6637 => &["5.0_4H", "Auro9.0"],
            0x663b => &["4.1_4H", "Auro8.1"],
            0x663f => &["5.1_4H", "Auro9.1"],
            0x67b3 => &["7.0_4H_no_C"],
            0x67b7 => &["7.0_4H", "Auro11.0_74"],
            0x67bb => &["7.1_4H_no_C"],
            0x67bf => &["7.1_4H", "Auro11.1_74"],
            0x6e33 => &["4.0_5H"],
            0x6e37 => &["5.0_5H"],
            0x6e3b => &["4.1_5H"],
            0x6e3f => &["5.1_5H"],
            0x6fb3 => &["7.0_5H_no_C"],
            0x6fb7 => &["7.0_5H"],
            0x6fbb => &["7.1_5H_no_C"],
            0x6fbf => &["7.1_5H"],
            0x7633 => &["4.0_4H_1T"],
            0x7637 => &["5.0_4H_1T", "Auro10.0"],
            0x763b => &["4.1_4H_1T"],
            0x763f => &["5.1_4H_1T", "Auro10.1"],
            0x77b3 => &["7.0_4H_1T_no_C"],
            0x77b7 => &["7.0_4H_1T", "Auro12.0"],
            0x77bb => &["7.1_4H_1T_no_C"],
            0x77bf => &["7.1_4H_1T", "Auro12.1"],
            0x7e33 => &["4.0_5H_1T"],
            0x7e37 => &["5.0_5H_1T", "Auro11.0_551"],
            0x7e3b => &["4.1_5H_1T"],
            0x7e3f => &["5.1_5H_1T", "Auro11.1_551"],
            0x7fb3 => &["7.0_5H_1T_no_C"],
            0x7fb7 => &["7.0_5H_1T", "Auro13.0"],
            0x7fbb => &["7.1_5H_1T_no_C"],
            0x7fbf => &["7.1_5H_1T", "Auro13.1"],
            0xeffff7 => &["NHK_22.0"],
            0xffffff => &["NHK_22.2"],
            0x3606600 => &["Cube"],
            0x3000663f => &["5.1_4H_2T"],
            0x300067bf => &["7.1_4H_2T"],
            0x30006e3f => &["5.1_5H_2T"],
            0x30006fbf => &["7.1_5H_2T"],
            0x3c0067bf => &["9.1_4H_2T"],
            0x3c006fbf => &["9.1_5H_2T"],
            _ => &[],
        }
    }
}

#[cfg(test)]
#[path = "layout_tests.rs"]
mod tests;
