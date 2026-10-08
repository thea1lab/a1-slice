//! Caption colours, sizes, and the font names the burn uses.

use crate::types::{CaptionColor, CaptionFont, CaptionPosition, CaptionSize, CaptionStyle};

pub const DEFAULT_CAPTION_STYLE: CaptionStyle = CaptionStyle {
    color: CaptionColor::White,
    custom_color: None,
    position: CaptionPosition::Bottom,
    size: CaptionSize::Large,
    font_size: None,
    font: CaptionFont::Sans,
};

/// ASS colour is `&HAABBGGRR`. Alpha 00 is opaque.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptionInk {
    pub hex: &'static str,
    pub ass: &'static str,
    pub outline: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptionInkTable {
    pub white: CaptionInk,
    pub cream: CaptionInk,
    pub yellow: CaptionInk,
    pub black: CaptionInk,
}

impl CaptionInkTable {
    pub fn get(self, color: CaptionColor) -> CaptionInk {
        match color {
            CaptionColor::White => self.white,
            CaptionColor::Cream => self.cream,
            CaptionColor::Yellow => self.yellow,
            CaptionColor::Black => self.black,
        }
    }
}

pub const CAPTION_INK: CaptionInkTable = CaptionInkTable {
    white: CaptionInk {
        hex: "#ffffff",
        ass: "&H00FFFFFF",
        outline: "&H00000000",
    },
    cream: CaptionInk {
        hex: "#fff8e0",
        ass: "&H00E0F8FF",
        outline: "&H00000000",
    },
    yellow: CaptionInk {
        hex: "#ffe14a",
        ass: "&H004AE1FF",
        outline: "&H00000000",
    },
    black: CaptionInk {
        hex: "#111111",
        ass: "&H00111111",
        outline: "&H00FFFFFF",
    },
};

/// CSS pixels on a 288-tall caption frame. libass then scales that frame to the video.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptionMetrics {
    pub font_size: i64,
    pub margin: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptionMetricsTable {
    pub small: CaptionMetrics,
    pub medium: CaptionMetrics,
    pub large: CaptionMetrics,
}

impl CaptionMetricsTable {
    pub fn get(self, size: CaptionSize) -> CaptionMetrics {
        match size {
            CaptionSize::Small => self.small,
            CaptionSize::Medium => self.medium,
            CaptionSize::Large => self.large,
        }
    }

    pub(super) fn values(self) -> [CaptionMetrics; 3] {
        [self.small, self.medium, self.large]
    }

    pub(super) fn preset_for(self, font_size: i64) -> Option<CaptionSize> {
        if self.small.font_size == font_size {
            Some(CaptionSize::Small)
        } else if self.medium.font_size == font_size {
            Some(CaptionSize::Medium)
        } else if self.large.font_size == font_size {
            Some(CaptionSize::Large)
        } else {
            None
        }
    }
}

pub const CAPTION_METRICS: CaptionMetricsTable = CaptionMetricsTable {
    small: CaptionMetrics {
        font_size: 18,
        margin: 36,
    },
    medium: CaptionMetrics {
        font_size: 24,
        margin: 60,
    },
    large: CaptionMetrics {
        font_size: 28,
        margin: 90,
    },
};

/// libass outline in PlayResY 288 script pixels. Outline 2 fills the letters.
pub const CAPTION_BURN_OUTLINE: f64 = 0.55;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptionFontFamily {
    pub sans: &'static str,
    pub serif: &'static str,
    pub mono: &'static str,
}

impl CaptionFontFamily {
    pub fn get(self, font: CaptionFont) -> &'static str {
        match font {
            CaptionFont::Sans => self.sans,
            CaptionFont::Serif => self.serif,
            CaptionFont::Mono => self.mono,
        }
    }
}

pub const CAPTION_FONT_FAMILY: CaptionFontFamily = CaptionFontFamily {
    sans: "\"Noto Sans\", \"Liberation Sans\", \"DejaVu Sans\", sans-serif",
    serif: "\"Noto Serif\", \"Liberation Serif\", \"DejaVu Serif\", Georgia, serif",
    mono: "\"Noto Sans Mono\", \"Liberation Mono\", \"DejaVu Sans Mono\", ui-monospace, monospace",
};

pub const MIN_CAPTION_FONT_SIZE: i64 = 8;
pub const MAX_CAPTION_FONT_SIZE: i64 = 96;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptionPaletteEntry {
    pub label: &'static str,
    pub hex: &'static str,
    pub named: Option<CaptionColor>,
}

pub const CAPTION_PALETTE: [CaptionPaletteEntry; 12] = [
    CaptionPaletteEntry {
        label: "White",
        hex: CAPTION_INK.white.hex,
        named: Some(CaptionColor::White),
    },
    CaptionPaletteEntry {
        label: "Cream",
        hex: CAPTION_INK.cream.hex,
        named: Some(CaptionColor::Cream),
    },
    CaptionPaletteEntry {
        label: "Yellow",
        hex: CAPTION_INK.yellow.hex,
        named: Some(CaptionColor::Yellow),
    },
    CaptionPaletteEntry {
        label: "Gold",
        hex: "#ffb83e",
        named: None,
    },
    CaptionPaletteEntry {
        label: "Orange",
        hex: "#ff9a3c",
        named: None,
    },
    CaptionPaletteEntry {
        label: "Red",
        hex: "#ff5a4a",
        named: None,
    },
    CaptionPaletteEntry {
        label: "Pink",
        hex: "#ff8ad4",
        named: None,
    },
    CaptionPaletteEntry {
        label: "Green",
        hex: "#7dff6a",
        named: None,
    },
    CaptionPaletteEntry {
        label: "Cyan",
        hex: "#6aefff",
        named: None,
    },
    CaptionPaletteEntry {
        label: "Blue",
        hex: "#6aa6ff",
        named: None,
    },
    CaptionPaletteEntry {
        label: "Purple",
        hex: "#c48aff",
        named: None,
    },
    CaptionPaletteEntry {
        label: "Black",
        hex: CAPTION_INK.black.hex,
        named: Some(CaptionColor::Black),
    },
];
