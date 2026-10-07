//! Library behind the A1 Slice window.
//!
//! Screens call these functions. They do not spawn ffmpeg themselves.

pub mod agent;
pub mod caption_edit;
pub mod captions;
pub mod clip_find;
pub mod clips;
pub mod crop;
pub mod ffmpeg_cmd;
pub mod preview;
pub mod project;
pub mod settings;
pub mod sidecars;
pub mod types;
pub mod whisper;
pub mod wizard;

// Filled in as the ports land.
#[allow(unused_imports)]
pub use types::*;
