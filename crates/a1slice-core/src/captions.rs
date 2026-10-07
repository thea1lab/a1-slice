//! Caption colour, type size, and the ASS force-style string.

use crate::types::{
    CaptionColor, CaptionFont, CaptionLook, CaptionPosition, CaptionProject, CaptionSize,
    CaptionSource, CaptionStyle,
};
use serde_json::Value;

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

    fn values(self) -> [CaptionMetrics; 3] {
        [self.small, self.medium, self.large]
    }

    fn preset_for(self, font_size: i64) -> Option<CaptionSize> {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedCaptionInk {
    pub hex: String,
    pub ass: String,
    pub outline: String,
}

pub fn parse_hex_color(value: &Value) -> Option<String> {
    normalize_hex(value.as_str()?)
}

pub fn clamp_caption_font_size(value: f64) -> i64 {
    if !value.is_finite() {
        return CAPTION_METRICS.large.font_size;
    }
    js_round(value).clamp(MIN_CAPTION_FONT_SIZE, MAX_CAPTION_FONT_SIZE)
}

pub fn parse_caption_style(value: &Value) -> Option<CaptionStyle> {
    if !value.is_object() {
        return None;
    }
    let color = parse_color(value.get("color"))?;
    let position = parse_position(value.get("position"))?;
    let size = parse_size(value.get("size"))?;
    let font = parse_font(value.get("font"))?;
    let mut style = CaptionStyle {
        color,
        custom_color: None,
        position,
        size,
        font_size: None,
        font,
    };
    if let Some(custom) = value.get("customColor").and_then(parse_hex_color) {
        style.custom_color = Some(custom);
    }
    if let Some(font_size) = parse_font_size(value.get("fontSize")) {
        if let Some(preset) = CAPTION_METRICS.preset_for(font_size) {
            style.size = preset;
        } else {
            style.font_size = Some(font_size);
        }
    }
    Some(style)
}

pub fn caption_ink(style: &CaptionStyle) -> ResolvedCaptionInk {
    if let Some(custom) = style.custom_color.as_deref().and_then(normalize_hex) {
        let outline = if relative_luminance(&custom) < 0.2 {
            "&H00FFFFFF"
        } else {
            "&H00000000"
        };
        return ResolvedCaptionInk {
            hex: custom.clone(),
            ass: hex_to_ass(&custom),
            outline: outline.to_string(),
        };
    }
    let ink = CAPTION_INK.get(style.color);
    ResolvedCaptionInk {
        hex: ink.hex.to_string(),
        ass: ink.ass.to_string(),
        outline: ink.outline.to_string(),
    }
}

pub fn caption_metrics(style: &CaptionStyle) -> CaptionMetrics {
    let Some(raw) = style.font_size else {
        return CAPTION_METRICS.get(style.size);
    };
    let font_size = clamp_caption_font_size(raw as f64);
    if let Some(preset) = CAPTION_METRICS
        .values()
        .into_iter()
        .find(|metrics| metrics.font_size == font_size)
    {
        return preset;
    }
    let margin = js_round(
        (font_size as f64 * CAPTION_METRICS.large.margin as f64)
            / CAPTION_METRICS.large.font_size as f64,
    );
    CaptionMetrics { font_size, margin }
}

pub fn style_with_preset_size(style: &CaptionStyle, size: CaptionSize) -> CaptionStyle {
    CaptionStyle {
        size,
        font_size: None,
        ..style.clone()
    }
}

pub fn style_with_font_size(style: &CaptionStyle, px: f64) -> CaptionStyle {
    let font_size = clamp_caption_font_size(px);
    if let Some(preset) = CAPTION_METRICS.preset_for(font_size) {
        style_with_preset_size(style, preset)
    } else {
        CaptionStyle {
            font_size: Some(font_size),
            ..style.clone()
        }
    }
}

pub fn style_with_color(style: &CaptionStyle, hex: &str) -> CaptionStyle {
    let Some(normalized) = normalize_hex(hex) else {
        return style.clone();
    };
    if let Some(named) = CAPTION_PALETTE
        .iter()
        .find_map(|item| (item.hex == normalized).then_some(item.named).flatten())
    {
        return CaptionStyle {
            color: named,
            custom_color: None,
            ..style.clone()
        };
    }
    let color = if relative_luminance(&normalized) < 0.2 {
        CaptionColor::Black
    } else {
        CaptionColor::White
    };
    CaptionStyle {
        color,
        custom_color: Some(normalized),
        ..style.clone()
    }
}

pub fn coerce_caption_look(look: &Value) -> Option<CaptionLook> {
    match look.as_str() {
        Some("srt") => Some(CaptionLook::Srt),
        Some("burn" | "burn-large" | "burn-small") => Some(CaptionLook::Burn),
        _ => None,
    }
}

pub fn coerce_caption_style(look: &Value, style: &Value) -> CaptionStyle {
    parse_caption_style(style).unwrap_or_else(|| {
        let mut next = DEFAULT_CAPTION_STYLE.clone();
        next.size = if look.as_str() == Some("burn-small") {
            CaptionSize::Small
        } else {
            CaptionSize::Large
        };
        next
    })
}

pub fn caption_force_style(style: &CaptionStyle, font_name: &str, cell_ratio: f64) -> String {
    let ink = caption_ink(style);
    let metrics = caption_metrics(style);
    let ratio = if cell_ratio.is_finite() && cell_ratio > 0.0 {
        cell_ratio
    } else {
        1.0
    };
    let alignment = match style.position {
        CaptionPosition::Top => 8,
        CaptionPosition::Middle => 5,
        CaptionPosition::Bottom => 2,
    };
    let margin = if style.position == CaptionPosition::Middle {
        0
    } else {
        metrics.margin
    };
    let font_size = js_round(metrics.font_size as f64 * ratio);
    [
        format!("FontName={font_name}"),
        format!("FontSize={font_size}"),
        format!("PrimaryColour={}", ink.ass),
        format!("OutlineColour={}", ink.outline),
        "BorderStyle=1".to_string(),
        format!("Outline={CAPTION_BURN_OUTLINE}"),
        "Shadow=0".to_string(),
        "Bold=0".to_string(),
        format!("Alignment={alignment}"),
        format!("MarginV={margin}"),
    ]
    .join(",")
}

pub fn blank_caption_project(has_transcript: bool) -> CaptionProject {
    CaptionProject {
        source: if has_transcript {
            CaptionSource::Transcript
        } else {
            CaptionSource::Manual
        },
        look: CaptionLook::Srt,
        style: DEFAULT_CAPTION_STYLE.clone(),
        cues: Vec::new(),
        file_path: None,
    }
}

pub fn present_caption_project(
    saved: Option<CaptionProject>,
    has_transcript: bool,
) -> CaptionProject {
    let Some(saved) = saved else {
        return blank_caption_project(has_transcript);
    };
    let source = if saved.source == CaptionSource::Manual {
        CaptionSource::Manual
    } else {
        CaptionSource::Transcript
    };
    let file_path = match saved.file_path {
        Some(path) if source == CaptionSource::Manual && !js_trim(&path).is_empty() => Some(path),
        _ => None,
    };
    let look_json = Value::String(
        match saved.look {
            CaptionLook::Srt => "srt",
            CaptionLook::Burn => "burn",
        }
        .to_string(),
    );
    let style_json = serde_json::to_value(&saved.style).unwrap_or(Value::Null);
    CaptionProject {
        source,
        look: coerce_caption_look(&look_json).unwrap_or(CaptionLook::Srt),
        style: coerce_caption_style(&look_json, &style_json),
        cues: saved.cues,
        file_path,
    }
}

fn parse_color(value: Option<&Value>) -> Option<CaptionColor> {
    match value.and_then(Value::as_str) {
        Some("white") => Some(CaptionColor::White),
        Some("cream") => Some(CaptionColor::Cream),
        Some("yellow") => Some(CaptionColor::Yellow),
        Some("black") => Some(CaptionColor::Black),
        _ => None,
    }
}

fn parse_position(value: Option<&Value>) -> Option<CaptionPosition> {
    match value.and_then(Value::as_str) {
        Some("bottom") => Some(CaptionPosition::Bottom),
        Some("middle") => Some(CaptionPosition::Middle),
        Some("top") => Some(CaptionPosition::Top),
        _ => None,
    }
}

fn parse_size(value: Option<&Value>) -> Option<CaptionSize> {
    match value.and_then(Value::as_str) {
        Some("small") => Some(CaptionSize::Small),
        Some("medium") => Some(CaptionSize::Medium),
        Some("large") => Some(CaptionSize::Large),
        _ => None,
    }
}

fn parse_font(value: Option<&Value>) -> Option<CaptionFont> {
    match value.and_then(Value::as_str) {
        Some("sans") => Some(CaptionFont::Sans),
        Some("serif") => Some(CaptionFont::Serif),
        Some("mono") => Some(CaptionFont::Mono),
        _ => None,
    }
}

fn parse_font_size(value: Option<&Value>) -> Option<i64> {
    let number = value?.as_f64()?;
    if !number.is_finite() {
        return None;
    }
    Some(clamp_caption_font_size(number))
}

fn normalize_hex(value: &str) -> Option<String> {
    let value = js_trim(value);
    let rest = value.strip_prefix('#')?;
    if rest.len() != 6 || !rest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    Some(format!("#{}", rest.to_ascii_lowercase()))
}

fn hex_to_ass(hex: &str) -> String {
    let red = &hex[1..3];
    let green = &hex[3..5];
    let blue = &hex[5..7];
    format!("&H00{blue}{green}{red}").to_uppercase()
}

fn channel_linear(value: f64) -> f64 {
    let s = value / 255.0;
    if s <= 0.04045 {
        s / 12.92
    } else {
        ((s + 0.055) / 1.055).powf(2.4)
    }
}

fn relative_luminance(hex: &str) -> f64 {
    let red = u32::from_str_radix(&hex[1..3], 16).unwrap_or(0) as f64;
    let green = u32::from_str_radix(&hex[3..5], 16).unwrap_or(0) as f64;
    let blue = u32::from_str_radix(&hex[5..7], 16).unwrap_or(0) as f64;
    0.2126 * channel_linear(red) + 0.7152 * channel_linear(green) + 0.0722 * channel_linear(blue)
}

/// `Math.round`: halves go toward +∞.
fn js_round(n: f64) -> i64 {
    if !n.is_finite() {
        return 0;
    }
    let floor = n.floor();
    let rounded = if n - floor >= 0.5 { floor + 1.0 } else { floor };
    if rounded >= i64::MAX as f64 {
        i64::MAX
    } else if rounded <= i64::MIN as f64 {
        i64::MIN
    } else {
        rounded as i64
    }
}

fn is_js_ws(c: char) -> bool {
    matches!(
        c,
        '\u{0009}'
            | '\u{000A}'
            | '\u{000B}'
            | '\u{000C}'
            | '\u{000D}'
            | '\u{0020}'
            | '\u{00A0}'
            | '\u{1680}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202F}'
            | '\u{205F}'
            | '\u{3000}'
            | '\u{FEFF}'
    ) || ('\u{2000}'..='\u{200A}').contains(&c)
}

fn js_trim(s: &str) -> &str {
    let Some(start) = s.find(|c: char| !is_js_ws(c)) else {
        return "";
    };
    let end = s
        .rfind(|c: char| !is_js_ws(c))
        .map(|index| index + s[index..].chars().next().map(|c| c.len_utf8()).unwrap_or(0))
        .unwrap_or(s.len());
    &s[start..end]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn force(style: &CaptionStyle, font_name: &str) -> String {
        caption_force_style(style, font_name, 1.0)
    }

    #[test]
    fn builds_an_ass_force_style_for_colour_place_size_and_font_name() {
        let force = force(
            &CaptionStyle {
                color: CaptionColor::Yellow,
                position: CaptionPosition::Top,
                size: CaptionSize::Medium,
                font: CaptionFont::Serif,
                ..DEFAULT_CAPTION_STYLE.clone()
            },
            "Noto Serif",
        );
        assert!(force.contains("FontName=Noto Serif"));
        assert!(force.contains(&format!("FontSize={}", CAPTION_METRICS.medium.font_size)));
        assert!(force.contains(&format!("PrimaryColour={}", CAPTION_INK.yellow.ass)));
        assert!(force.contains("Alignment=8"));
        assert!(force.contains(&format!("MarginV={}", CAPTION_METRICS.medium.margin)));
        assert!(force.contains(&format!("Outline={CAPTION_BURN_OUTLINE}")));
        assert!(force.contains("Shadow=0"));
    }

    #[test]
    fn centers_a_caption_and_drops_the_edge_margin() {
        let force = force(
            &CaptionStyle {
                position: CaptionPosition::Middle,
                ..DEFAULT_CAPTION_STYLE.clone()
            },
            "Noto Sans",
        );
        assert!(force.contains("Alignment=5"));
        assert!(force.contains("MarginV=0"));
    }

    #[test]
    fn outlines_black_type_in_white() {
        let force = force(
            &CaptionStyle {
                color: CaptionColor::Black,
                ..DEFAULT_CAPTION_STYLE.clone()
            },
            "Noto Sans",
        );
        assert!(force.contains(&format!("PrimaryColour={}", CAPTION_INK.black.ass)));
        assert!(force.contains(&format!("OutlineColour={}", CAPTION_INK.black.outline)));
    }

    #[test]
    fn keeps_the_older_large_and_small_sizes() {
        assert_eq!(CAPTION_METRICS.large.font_size, 28);
        assert_eq!(CAPTION_METRICS.large.margin, 90);
        assert_eq!(CAPTION_METRICS.small.font_size, 18);
        assert_eq!(CAPTION_METRICS.small.margin, 36);
    }

    #[test]
    fn rejects_a_partial_style() {
        assert!(parse_caption_style(&json!({
            "color": "pink",
            "position": "top",
            "size": "large",
            "font": "sans"
        }))
        .is_none());
        assert!(parse_caption_style(&json!(null)).is_none());
    }

    #[test]
    fn burns_a_custom_colour_and_a_custom_font_size() {
        let style = CaptionStyle {
            custom_color: Some("#ff5a4a".to_string()),
            font_size: Some(40),
            ..DEFAULT_CAPTION_STYLE.clone()
        };
        let force = force(&style, "Noto Sans");
        assert!(force.contains("FontSize=40"));
        assert!(force.contains(&format!("PrimaryColour={}", caption_ink(&style).ass)));
        assert!(force.contains("PrimaryColour=&H004A5AFF"));
        assert!(force.contains("OutlineColour=&H00000000"));
        assert!(force.contains("MarginV=129"));
    }

    #[test]
    fn scales_the_burn_to_the_font_cell_and_leaves_the_margin_alone() {
        let style = CaptionStyle {
            font_size: Some(13),
            font: CaptionFont::Mono,
            ..DEFAULT_CAPTION_STYLE.clone()
        };
        let force = caption_force_style(&style, "Noto Sans Mono Medium", 1.618);
        assert!(force.contains("FontSize=21"));
        assert!(force.contains("MarginV=42"));
        assert!(force.contains("FontName=Noto Sans Mono Medium"));
        assert!(caption_force_style(&style, "Noto Sans Mono", f64::NAN).contains("FontSize=13"));
    }

    #[test]
    fn outlines_a_dark_custom_colour_in_white() {
        let style = CaptionStyle {
            custom_color: Some("#112233".to_string()),
            ..DEFAULT_CAPTION_STYLE.clone()
        };
        let force = force(&style, "Noto Sans");
        assert!(force.contains("OutlineColour=&H00FFFFFF"));
        assert!(force.contains("PrimaryColour=&H00332211"));
    }

    #[test]
    fn stores_a_palette_colour_by_name_and_a_typed_size_by_preset() {
        assert_eq!(
            style_with_color(&DEFAULT_CAPTION_STYLE, "#ffe14a"),
            CaptionStyle {
                color: CaptionColor::Yellow,
                ..DEFAULT_CAPTION_STYLE.clone()
            }
        );
        assert_eq!(
            style_with_color(&DEFAULT_CAPTION_STYLE, "#ff5a4a")
                .custom_color
                .as_deref(),
            Some("#ff5a4a")
        );
        assert_eq!(
            style_with_font_size(&DEFAULT_CAPTION_STYLE, 28.0),
            CaptionStyle {
                size: CaptionSize::Large,
                ..DEFAULT_CAPTION_STYLE.clone()
            }
        );
        assert_eq!(
            style_with_font_size(&DEFAULT_CAPTION_STYLE, 40.0).font_size,
            Some(40)
        );
        assert_eq!(
            style_with_font_size(&DEFAULT_CAPTION_STYLE, 4.0).font_size,
            Some(8)
        );
        assert_eq!(
            style_with_font_size(&DEFAULT_CAPTION_STYLE, 200.0).font_size,
            Some(96)
        );
    }

    #[test]
    fn reads_a_custom_colour_and_size_and_keeps_a_named_style_unchanged() {
        assert_eq!(
            parse_caption_style(&json!({
                "color": "white",
                "customColor": "#FF5A4A",
                "position": "bottom",
                "size": "large",
                "fontSize": 40,
                "font": "sans"
            })),
            Some(CaptionStyle {
                custom_color: Some("#ff5a4a".to_string()),
                font_size: Some(40),
                ..DEFAULT_CAPTION_STYLE.clone()
            })
        );
        assert_eq!(
            parse_caption_style(&json!({
                "color": "white",
                "position": "bottom",
                "size": "large",
                "font": "sans",
                "fontSize": 18
            })),
            Some(CaptionStyle {
                size: CaptionSize::Small,
                ..DEFAULT_CAPTION_STYLE.clone()
            })
        );
    }

    #[test]
    fn reads_a_complete_style_and_falls_back_when_it_is_missing() {
        assert_eq!(
            parse_caption_style(&json!({
                "color": "cream",
                "position": "bottom",
                "size": "small",
                "font": "mono"
            })),
            Some(CaptionStyle {
                color: CaptionColor::Cream,
                custom_color: None,
                position: CaptionPosition::Bottom,
                size: CaptionSize::Small,
                font_size: None,
                font: CaptionFont::Mono,
            })
        );
        assert_eq!(
            coerce_caption_style(&json!("burn-small"), &json!({ "color": "nope" })),
            CaptionStyle {
                size: CaptionSize::Small,
                ..DEFAULT_CAPTION_STYLE.clone()
            }
        );
        assert_eq!(
            coerce_caption_style(&json!("burn-large"), &json!(null)),
            DEFAULT_CAPTION_STYLE
        );
    }
}
