mod app;
mod backend;

fn load_icon() -> Option<egui::IconData> {
    const ICON_PNG: &[u8] = include_bytes!("../../../resources/icon.png");
    match eframe::icon_data::from_png_bytes(ICON_PNG) {
        Ok(icon) => Some(icon),
        Err(err) => {
            eprintln!("Failed to load app icon: {err}");
            None
        }
    }
}

#[test]
fn test_load_icon() {
    let icon = load_icon().expect("should load icon");
    println!(
        "Loaded icon: {}x{}, bytes: {}",
        icon.width,
        icon.height,
        icon.rgba.len()
    );
}

#[cfg(target_os = "linux")]
fn ensure_linux_desktop_entry() {
    let Some(data_dir) = std::env::var_os("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".local/share"))
        })
    else {
        return;
    };

    let apps_dir = data_dir.join("applications");
    let icons_svg_dir = data_dir.join("icons/hicolor/scalable/apps");
    let icons_png_dir = data_dir.join("icons/hicolor/256x256/apps");

    let _ = std::fs::create_dir_all(&apps_dir);
    let _ = std::fs::create_dir_all(&icons_svg_dir);
    let _ = std::fs::create_dir_all(&icons_png_dir);

    const ICON_SVG: &[u8] = include_bytes!("../../../resources/icon.svg");
    const ICON_PNG: &[u8] = include_bytes!("../../../resources/icon.png");

    let svg_dest = icons_svg_dir.join("a1slice.svg");
    let mut icon_updated = false;
    if !svg_dest.exists()
        || std::fs::read(&svg_dest)
            .map(|b| b != ICON_SVG)
            .unwrap_or(true)
    {
        if std::fs::write(&svg_dest, ICON_SVG).is_ok() {
            icon_updated = true;
        }
    }

    let png_dest = icons_png_dir.join("a1slice.png");
    if !png_dest.exists()
        || std::fs::read(&png_dest)
            .map(|b| b != ICON_PNG)
            .unwrap_or(true)
    {
        if std::fs::write(&png_dest, ICON_PNG).is_ok() {
            icon_updated = true;
        }
    }

    if icon_updated {
        let hicolor_dir = data_dir.join("icons/hicolor");
        let _ = std::process::Command::new("gtk-update-icon-cache")
            .arg("-q")
            .arg("-f")
            .arg("-t")
            .arg(&hicolor_dir)
            .status();
    }

    let desktop_dest = apps_dir.join("a1slice.desktop");
    let exe = std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "a1slice".to_string());

    let desktop_content = format!(
        "[Desktop Entry]\n\
         Name=A1 Slice\n\
         Comment=Desktop app for working on one video at a time\n\
         Exec={exe}\n\
         Icon=a1slice\n\
         Terminal=false\n\
         Type=Application\n\
         Categories=AudioVideo;Video;AudioVideoEditing;\n\
         StartupWMClass=a1slice\n"
    );

    if std::fs::read_to_string(&desktop_dest)
        .map(|c| c != desktop_content)
        .unwrap_or(true)
    {
        let _ = std::fs::write(&desktop_dest, desktop_content);
    }
}

fn main() -> eframe::Result<()> {
    #[cfg(target_os = "linux")]
    ensure_linux_desktop_entry();

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1280.0, 960.0])
        .with_min_inner_size([800.0, 680.0])
        .with_title("A1 Slice")
        .with_app_id("a1slice");

    if let Some(icon) = load_icon() {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "A1 Slice",
        options,
        Box::new(|cc| Ok(Box::new(app::A1App::new(cc)))),
    )
}
