# AGENTS.md

Rules for changing the A1 Slice Rust app. `CLAUDE.md` is the short map. This file is the one to follow when adding or moving code.

## Commands

```bash
cargo run -p a1slice          # Open the window
cargo build -p a1slice        # Compile the app
cargo test --offline          # Unit tests. One ffmpeg integration test is ignored.
cargo fmt                     # Format before finishing
bash scripts/build-whisper.sh # Build whisper-cli into resources/bin/
```

Rust 1.85 or newer. Edition 2021. `ffmpeg` and `ffprobe` must be on `PATH`. `ffplay` is optional and is used for playback audio. Offline builds use `cargo build -p a1slice --offline` when the Cargo cache is already filled.

## Crates

- `crates/a1slice` is the window and the processes. `main.rs` opens the window. `app/` draws screens. `backend/` runs ffmpeg, whisper-cli, and the installed agents.
- `crates/a1slice-core` is the library the window calls. It owns sidecar JSON, crop math, clip bounds, preview timing, wizard state, and settings. It does not spawn ffmpeg, whisper, or an agent.

`lib.rs` only lists modules. Do not put logic there.

## Where a change goes

| Change | File |
| --- | --- |
| A screen | `crates/a1slice/src/app/<screen>.rs`, as `impl A1App` |
| Shared buttons, sheets, rows | `app/widgets.rs` |
| Colours, type, sunset stripe | `app/theme.rs` |
| Picture, playhead, keyboard | `app/playback.rs`, `app/picture.rs`, `app/review_bar.rs`, `app/review_gestures.rs` |
| Bar geometry | `app/layout.rs`, with a unit test |
| Open a video or start a job | `app/jobs.rs` |
| ffmpeg, whisper, or an agent process | `backend/` |
| Clip JSON an agent prints | `backend/clip_parse.rs` |
| Wizard flow | `a1slice-core/src/wizard/` |
| Crop math or an ffmpeg argument | `crop.rs` or `ffmpeg_cmd/` |
| Caption text, diffs, prompts | `caption_edit/` |
| Files beside a video | `sidecars/` for the window, `project/` for the parser port |

`A1App` stays in `app/mod.rs`. Screen files are child modules so they can use its fields. Add a screen by adding the file, a `Screen` arm, and a call from `draw`.

The window calls `sidecars`. `project` is the other file and parser API, and `settings` uses `project::a1slice_dir`. Keep both until a change has tests for the path the window actually runs.

## Rust shape

- One file, one job. Split a file when it passes about 400 lines. A test module may be longer than the code it covers.
- A function should do one thing and take a name from that thing. A `match` that only forwards to a named function is fine. A 300-line function is not.
- Pure logic lives in `a1slice-core`, not in a screen. The window calls it.
- `pub` is the crate's API. Inside a folder module, share with `pub(super)`. Leave everything else private.
- Re-export a folder's public functions from its `mod.rs`, so `a1slice_core::wizard::wizard_reduce` stays stable.
- Format with `cargo fmt`. Do not add a rustfmt.toml unless the repo already has one.
- No new dependency for something `std` or the crates already in `Cargo.toml` can do.
- Sidecar JSON field names stay compatible with the files already written beside a video.
- Do not spawn a process from `a1slice-core`.

## Tests

Put a unit test in the same file as the function. `cargo test --offline` must pass. The ignored test is `generated_video_frame_reframe_and_caption_export` in `backend/`. It needs ffmpeg and ffprobe.

New parsing, time, crop, clip, and wizard behaviour needs a test that fails if the behaviour changes. Do not delete a test to make a move compile. Move the test with the function.

Screen drawing is covered by the geometry tests in `app/layout.rs`. Do not assert on egui paint calls.

## UI

Follow `DESIGN.md`. Charcoal window `#161616`, cream text, orange `#fa520f` for the next action. No light theme. Fonts come from the Noto Sans and Noto Serif files on the machine. Buttons go through `app/widgets.rs`, not a one-off `ui.button`, unless a screen already has a reason to paint its own control.
