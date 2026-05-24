//! File-filtering rules for indexing: built-in binary blacklist + user globs +
//! `.gitignore` (handled by the `ignore` crate).

use std::path::Path;

use ignore::overrides::OverrideBuilder;
use ignore::WalkBuilder;

/// Extensions we never index — binaries, media, archives, fonts, model weights.
pub const BINARY_EXTS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "svg", "ico", "bmp", "tiff",
    "pdf", "zip", "tar", "gz", "bz2", "xz", "7z", "rar",
    "exe", "dll", "so", "dylib", "bin", "wasm", "a", "o", "obj", "lib",
    "woff", "woff2", "ttf", "otf", "eot",
    "mp3", "mp4", "mov", "avi", "mkv", "ogg", "wav", "flac", "webm",
    "parquet", "onnx", "safetensors", "pt", "ckpt", "h5", "pkl",
    "class", "jar", "war",
];

/// Directories we always want to skip entirely.
pub const DEFAULT_IGNORED_DIRS: &[&str] = &[
    ".git",
    "bin",
    "node_modules",
    "target",
];

/// Build a `WalkBuilder` for `root` configured with:
/// - `.gitignore`, `.ignore`, `.git/info/exclude` (default)
/// - user extra globs (negative — i.e. exclusions)
/// - built-in binary extension blacklist (also negative)
pub fn walker(root: &Path, extra_globs: &[String]) -> WalkBuilder {
    let mut ob = OverrideBuilder::new(root);
    
    // Handle user globs
    for g in extra_globs {
        let pat = if let Some(rest) = g.strip_prefix('!') {
            // user is RE-INCLUDING something; pass through as positive.
            rest.to_string()
        } else {
            format!("!{g}")
        };
        let _ = ob.add(&pat);
    }

    // Handle built-in ignored directories
    for dir in DEFAULT_IGNORED_DIRS {
        // !dir ignores the folder; !dir/** ignores everything inside it recursively
        let _ = ob.add(&format!("!{dir}"));
        let _ = ob.add(&format!("!{dir}/**"));
    }

    // Handle binaries
    for ext in BINARY_EXTS {
        let _ = ob.add(&format!("!*.{ext}"));
    }
    let overrides = ob.build().expect("override patterns compile");

    let mut wb = WalkBuilder::new(root);
    wb.standard_filters(true)
        .git_ignore(true)
        .git_exclude(true)
        .git_global(true)
        .hidden(false)
        .overrides(overrides);
    wb
}

/// Quick check: should we skip this path based on size?
pub fn too_large(path: &Path, max_bytes: u64) -> bool {
    match std::fs::metadata(path) {
        Ok(m) => m.len() > max_bytes,
        Err(_) => true,
    }
}
