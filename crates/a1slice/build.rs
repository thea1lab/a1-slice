fn main() {
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("../../resources/icon.ico");
        if let Err(e) = res.compile() {
            eprintln!("Failed to compile Windows resource: {e}");
        }

        if let Ok(ffmpeg_dir) = std::env::var("FFMPEG_DIR") {
            let bin_dir = std::path::Path::new(&ffmpeg_dir).join("bin");
            if let Ok(out_dir) = std::env::var("OUT_DIR") {
                let out_path = std::path::PathBuf::from(out_dir);
                if let Some(target_dir) = out_path.ancestors().nth(3) {
                    if let Ok(entries) = std::fs::read_dir(&bin_dir) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if path.extension().and_then(|ext| ext.to_str()) == Some("dll") {
                                if let Some(file_name) = path.file_name() {
                                    let dest = target_dir.join(file_name);
                                    let _ = std::fs::copy(&path, &dest);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
