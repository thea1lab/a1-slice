//! Read a caption style and build the ASS force-style string.

use crate::types::{
    CaptionColor, CaptionFont, CaptionLook, CaptionPosition, CaptionProject, CaptionSize,
    CaptionSource, CaptionStyle,
};
use serde_json::Value;

use super::palette::{
    CaptionMetrics, CAPTION_BURN_OUTLINE, CAPTION_INK, CAPTION_METRICS, CAPTION_PALETTE,
    CAPTION_PLAY_RES_X, CAPTION_PLAY_RES_Y, CAPTION_SIDE_MARGIN, DEFAULT_CAPTION_STYLE,
    MAX_CAPTION_FONT_SIZE, MIN_CAPTION_FONT_SIZE,
};

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

/// Where a burned caption sits, in the same units as `frame_w` and `frame_h`.
///
/// Font size and `margin_v` scale with the picture height. The side inset scales
/// with the width. That is how libass maps PlayRes 384×288 onto the video.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CaptionFrame {
    pub font_px: f64,
    pub margin_v: f64,
    pub margin_x: f64,
    pub outline_px: f64,
    pub position: CaptionPosition,
}

pub fn caption_frame(style: &CaptionStyle, frame_w: f64, frame_h: f64) -> CaptionFrame {
    let metrics = caption_metrics(style);
    let height = positive_or(frame_h, CAPTION_PLAY_RES_Y);
    let width = positive_or(frame_w, CAPTION_PLAY_RES_X);
    let scale_y = height / CAPTION_PLAY_RES_Y;
    let scale_x = width / CAPTION_PLAY_RES_X;
    let margin_v = if style.position == CaptionPosition::Middle {
        0.0
    } else {
        metrics.margin as f64 * scale_y
    };
    CaptionFrame {
        font_px: metrics.font_size as f64 * scale_y,
        margin_v,
        margin_x: CAPTION_SIDE_MARGIN as f64 * scale_x,
        outline_px: CAPTION_BURN_OUTLINE * scale_y,
        position: style.position,
    }
}

fn positive_or(value: f64, fallback: f64) -> f64 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        fallback
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
        format!("MarginL={CAPTION_SIDE_MARGIN}"),
        format!("MarginR={CAPTION_SIDE_MARGIN}"),
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
