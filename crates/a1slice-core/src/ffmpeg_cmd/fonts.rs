//! Find a caption font and measure its em box.

use crate::types::CaptionFont;
use std::path::{Path, PathBuf};

struct FontFile {
    file: &'static str,
    name: &'static str,
}

const SANS_FILES: &[FontFile] = &[
    FontFile {
        file: "/usr/share/fonts/noto/NotoSans-Medium.ttf",
        name: "Noto Sans Medium",
    },
    FontFile {
        file: "/usr/share/fonts/noto/NotoSans-Regular.ttf",
        name: "Noto Sans",
    },
    FontFile {
        file: "/usr/share/fonts/liberation/LiberationSans-Regular.ttf",
        name: "Liberation Sans",
    },
    FontFile {
        file: "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        name: "Liberation Sans",
    },
    FontFile {
        file: "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        name: "DejaVu Sans",
    },
    FontFile {
        file: "/usr/share/fonts/truetype/freefont/FreeSans.ttf",
        name: "FreeSans",
    },
    FontFile {
        file: "/System/Library/Fonts/Supplemental/Arial.ttf",
        name: "Arial",
    },
    FontFile {
        file: "/Library/Fonts/Arial.ttf",
        name: "Arial",
    },
    FontFile {
        file: "C:\\Windows\\Fonts\\arial.ttf",
        name: "Arial",
    },
];

const SERIF_FILES: &[FontFile] = &[
    FontFile {
        file: "/usr/share/fonts/noto/NotoSerif-Medium.ttf",
        name: "Noto Serif Medium",
    },
    FontFile {
        file: "/usr/share/fonts/noto/NotoSerif-Regular.ttf",
        name: "Noto Serif",
    },
    FontFile {
        file: "/usr/share/fonts/liberation/LiberationSerif-Regular.ttf",
        name: "Liberation Serif",
    },
    FontFile {
        file: "/usr/share/fonts/truetype/liberation/LiberationSerif-Regular.ttf",
        name: "Liberation Serif",
    },
    FontFile {
        file: "/usr/share/fonts/truetype/dejavu/DejaVuSerif.ttf",
        name: "DejaVu Serif",
    },
    FontFile {
        file: "/usr/share/fonts/truetype/freefont/FreeSerif.ttf",
        name: "FreeSerif",
    },
    FontFile {
        file: "/System/Library/Fonts/Supplemental/Times New Roman.ttf",
        name: "Times New Roman",
    },
    FontFile {
        file: "C:\\Windows\\Fonts\\times.ttf",
        name: "Times New Roman",
    },
];

const MONO_FILES: &[FontFile] = &[
    FontFile {
        file: "/usr/share/fonts/noto/NotoSansMono-Medium.ttf",
        name: "Noto Sans Mono Medium",
    },
    FontFile {
        file: "/usr/share/fonts/noto/NotoSansMono-Regular.ttf",
        name: "Noto Sans Mono",
    },
    FontFile {
        file: "/usr/share/fonts/liberation/LiberationMono-Regular.ttf",
        name: "Liberation Mono",
    },
    FontFile {
        file: "/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf",
        name: "Liberation Mono",
    },
    FontFile {
        file: "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
        name: "DejaVu Sans Mono",
    },
    FontFile {
        file: "/usr/share/fonts/truetype/freefont/FreeMono.ttf",
        name: "FreeMono",
    },
    FontFile {
        file: "/System/Library/Fonts/Supplemental/Courier New.ttf",
        name: "Courier New",
    },
    FontFile {
        file: "C:\\Windows\\Fonts\\consola.ttf",
        name: "Consolas",
    },
];

fn system_fonts(face: CaptionFont) -> &'static [FontFile] {
    match face {
        CaptionFont::Sans => SANS_FILES,
        CaptionFont::Serif => SERIF_FILES,
        CaptionFont::Mono => MONO_FILES,
    }
}

fn bundled_faces(face: CaptionFont) -> &'static [FontFile] {
    match face {
        CaptionFont::Sans => &[
            FontFile {
                file: "NotoSans-Medium.ttf",
                name: "Noto Sans Medium",
            },
            FontFile {
                file: "NotoSans-Regular.ttf",
                name: "Noto Sans",
            },
        ],
        CaptionFont::Serif => &[
            FontFile {
                file: "NotoSerif-Medium.ttf",
                name: "Noto Serif Medium",
            },
            FontFile {
                file: "NotoSerif-Regular.ttf",
                name: "Noto Serif",
            },
        ],
        CaptionFont::Mono => &[
            FontFile {
                file: "NotoSansMono-Medium.ttf",
                name: "Noto Sans Mono Medium",
            },
            FontFile {
                file: "NotoSansMono-Regular.ttf",
                name: "Noto Sans Mono",
            },
        ],
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CaptionFontMatch {
    pub dir: String,
    pub name: String,
    pub file: String,
    /// (usWinAscent + usWinDescent) / unitsPerEm. 1 when the face could not be read.
    pub cell_ratio: f64,
}

pub fn find_caption_font(face: CaptionFont, extra_dirs: &[&str]) -> Option<CaptionFontMatch> {
    let mut order = Vec::new();
    push_bundled(&mut order, face, extra_dirs);
    push_system(&mut order, face);
    push_bundled(&mut order, CaptionFont::Sans, extra_dirs);
    push_system(&mut order, CaptionFont::Sans);
    for (file, name) in order {
        if file.as_os_str().is_empty() || !file.exists() {
            continue;
        }
        let file_str = path_to_string(&file);
        return Some(CaptionFontMatch {
            dir: dir_name(&file),
            name: name.to_string(),
            file: file_str.clone(),
            cell_ratio: font_cell_ratio(&file_str),
        });
    }
    None
}

fn push_bundled(out: &mut Vec<(PathBuf, &'static str)>, face: CaptionFont, extra_dirs: &[&str]) {
    for dir in extra_dirs {
        if dir.is_empty() {
            continue;
        }
        for face_file in bundled_faces(face) {
            out.push((Path::new(dir).join(face_file.file), face_file.name));
        }
    }
}

fn push_system(out: &mut Vec<(PathBuf, &'static str)>, face: CaptionFont) {
    for face_file in system_fonts(face) {
        out.push((PathBuf::from(face_file.file), face_file.name));
    }
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn dir_name(path: &Path) -> String {
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => path_to_string(parent),
        _ => ".".to_string(),
    }
}

/// libass sizes by the Windows cell. CSS sizes by the em. Returns 1 if the face cannot be read.
pub fn font_cell_ratio(file_path: &str) -> f64 {
    match std::fs::read(file_path) {
        Ok(data) => cell_ratio_from_sfnt(&data),
        Err(_) => 1.0,
    }
}

fn cell_ratio_from_sfnt(data: &[u8]) -> f64 {
    if data.len() < 12 {
        return 1.0;
    }
    let scaler = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
    const TRUETYPE: u32 = 0x0001_0000;
    const TRUE_TAG: u32 = 0x7472_7565;
    const OTTO: u32 = 0x4F54_544F;
    if scaler != TRUETYPE && scaler != TRUE_TAG && scaler != OTTO {
        return 1.0;
    }
    let num_tables = u16::from_be_bytes([data[4], data[5]]) as usize;
    let mut head: Option<(usize, usize)> = None;
    let mut os2: Option<(usize, usize)> = None;
    for i in 0..num_tables {
        let Some(rec) = 12usize.checked_add(i.saturating_mul(16)) else {
            return 1.0;
        };
        if rec
            .checked_add(16)
            .map(|end| end > data.len())
            .unwrap_or(true)
        {
            return 1.0;
        }
        let tag = &data[rec..rec + 4];
        let offset = u32::from_be_bytes(data[rec + 8..rec + 12].try_into().unwrap()) as usize;
        let length = u32::from_be_bytes(data[rec + 12..rec + 16].try_into().unwrap()) as usize;
        if tag == b"head" {
            head = Some((offset, length));
        }
        if tag == b"OS/2" {
            os2 = Some((offset, length));
        }
    }
    let Some((head_off, head_len)) = head else {
        return 1.0;
    };
    let Some((os2_off, os2_len)) = os2 else {
        return 1.0;
    };
    if head_len < 20 || os2_len < 78 {
        return 1.0;
    }
    if head_off
        .checked_add(20)
        .map(|end| end > data.len())
        .unwrap_or(true)
        || os2_off
            .checked_add(78)
            .map(|end| end > data.len())
            .unwrap_or(true)
    {
        return 1.0;
    }
    let units_per_em = u16::from_be_bytes(data[head_off + 18..head_off + 20].try_into().unwrap());
    let win_ascent = u16::from_be_bytes(data[os2_off + 74..os2_off + 76].try_into().unwrap());
    let win_descent = u16::from_be_bytes(data[os2_off + 76..os2_off + 78].try_into().unwrap());
    if units_per_em == 0 {
        return 1.0;
    }
    let ratio = (win_ascent as f64 + win_descent as f64) / units_per_em as f64;
    if !ratio.is_finite() || ratio < 0.5 || ratio > 2.5 {
        return 1.0;
    }
    ratio
}
