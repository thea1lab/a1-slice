//! Caption colour, type size, and the ASS force-style string.

mod palette;
mod style;

pub use palette::{
    CaptionFontFamily, CaptionInk, CaptionInkTable, CaptionMetrics, CaptionMetricsTable,
    CaptionPaletteEntry, CAPTION_BURN_OUTLINE, CAPTION_FONT_FAMILY, CAPTION_INK, CAPTION_METRICS,
    CAPTION_PALETTE, DEFAULT_CAPTION_STYLE, MAX_CAPTION_FONT_SIZE, MIN_CAPTION_FONT_SIZE,
};
pub use style::{
    blank_caption_project, caption_force_style, caption_frame, caption_ink, caption_metrics,
    clamp_caption_font_size, coerce_caption_look, coerce_caption_style, parse_caption_style,
    parse_hex_color, present_caption_project, style_with_color, style_with_font_size,
    style_with_preset_size, CaptionFrame, ResolvedCaptionInk,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{CaptionColor, CaptionFont, CaptionPosition, CaptionSize, CaptionStyle};
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
        assert!(force.contains("MarginL=10"));
        assert!(force.contains("MarginR=10"));
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
    fn places_a_caption_on_the_same_frame_the_burn_uses() {
        let bottom = caption_frame(&DEFAULT_CAPTION_STYLE, 384.0, 288.0);
        assert_eq!(bottom.font_px, 28.0);
        assert_eq!(bottom.margin_v, 90.0);
        assert_eq!(bottom.margin_x, 10.0);
        assert!((bottom.outline_px - 0.55).abs() < 1e-9);
        let video = caption_frame(&DEFAULT_CAPTION_STYLE, 1920.0, 1080.0);
        assert_eq!(video.font_px, 105.0);
        assert_eq!(video.margin_v, 337.5);
        assert_eq!(video.margin_x, 50.0);
        let middle = caption_frame(
            &CaptionStyle {
                position: CaptionPosition::Middle,
                ..DEFAULT_CAPTION_STYLE.clone()
            },
            800.0,
            450.0,
        );
        assert_eq!(middle.margin_v, 0.0);
        assert!(middle.font_px > 28.0);
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
    fn keeps_the_large_size_and_a_smaller_small() {
        assert_eq!(CAPTION_METRICS.large.font_size, 28);
        assert_eq!(CAPTION_METRICS.large.margin, 90);
        assert_eq!(CAPTION_METRICS.medium.font_size, 24);
        assert_eq!(CAPTION_METRICS.small.font_size, 12);
        assert_eq!(CAPTION_METRICS.small.margin, 24);
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
                "fontSize": 12
            })),
            Some(CaptionStyle {
                size: CaptionSize::Small,
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
            }))
            .unwrap()
            .font_size,
            Some(18)
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
