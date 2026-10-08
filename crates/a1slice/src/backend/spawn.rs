//! Background jobs, cancel, and opening a folder.

use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use super::job::{JobDone, Running};

pub fn open_path(path: &Path) -> Result<(), String> {
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(windows) {
        "explorer"
    } else {
        "xdg-open"
    };
    Command::new(opener)
        .arg(path)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn spawn_job<F>(work: F) -> Running
where
    F: FnOnce(Arc<AtomicBool>, Arc<Mutex<Option<std::process::Child>>>) -> Result<JobDone, String>
        + Send
        + 'static,
{
    let cancel = Arc::new(AtomicBool::new(false));
    let child = Arc::new(Mutex::new(None));
    let cancel_thread = Arc::clone(&cancel);
    let child_thread = Arc::clone(&child);
    let handle = std::thread::spawn(move || work(cancel_thread, child_thread));
    Running {
        cancel,
        child,
        handle,
    }
}

pub fn cancel_job(running: &Running) {
    running.cancel.store(true, Ordering::Relaxed);
    if let Some(mut child) = running.child.lock().unwrap().take() {
        let _ = child.kill();
    }
}
