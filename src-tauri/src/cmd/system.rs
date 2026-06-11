//! System toolbox for plugins.
//!
//! 8 generic primitives that cover the "I just need to persist some data"
//! use case. Composed by plugins like sticky-notes, todo-list, snippet-manager
//! to avoid writing per-plugin Rust code.
//!
//! ## Design notes
//!
//! - All paths must be absolute. Plugin authors are responsible for keeping
//!   their data under their own subdirectory; we do not scope paths by
//!   plugin id yet (see plugin-loader-phase1.md for the v0.4 story).
//! - File I/O is UTF-8. There is no binary read/write API.
//! - 10 MB cap on read/write (covers "all my notes" use case, blocks abuse).
//! - `file_write` is atomic: write to `<path>.tmp`, fsync, then `rename`.
//! - Clipboard uses the `arboard` crate (small cross-platform wrapper). On
//!   headless Linux without an X or Wayland server, clipboard ops return
//!   `SystemError::Clipboard`.
//! - `file_delete` on a non-empty directory errors. Recursion is the caller's
//!   responsibility (use `file_list` + `file_delete` in a loop).

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;
use thiserror::Error;

use super::dispatch;

const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum SystemError {
    #[error("path must be absolute: {0}")]
    PathNotAbsolute(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("file too large: {size} bytes (max {max})")]
    FileTooLarge { size: u64, max: u64 },
    #[error("clipboard error: {0}")]
    Clipboard(String),
    #[error("path is not a directory: {0}")]
    NotADirectory(String),
}

impl From<SystemError> for String {
    fn from(e: SystemError) -> Self {
        e.to_string()
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
}

fn ensure_absolute(path: &str) -> Result<PathBuf, SystemError> {
    let p = PathBuf::from(path);
    if !p.is_absolute() {
        return Err(SystemError::PathNotAbsolute(path.to_string()));
    }
    Ok(p)
}

fn tmp_path(p: &Path) -> PathBuf {
    let mut s = p.as_os_str().to_owned();
    s.push(".tmp");
    PathBuf::from(s)
}

// ===== file_read =====

#[derive(serde::Deserialize)]
struct FileReadArgs {
    path: String,
}

fn file_read_inner(path: &str) -> Result<String, SystemError> {
    let p = ensure_absolute(path)?;
    let meta = fs::metadata(&p)?;
    if meta.len() > MAX_FILE_SIZE {
        return Err(SystemError::FileTooLarge {
            size: meta.len(),
            max: MAX_FILE_SIZE,
        });
    }
    Ok(fs::read_to_string(&p)?)
}

fn file_read_dispatch(args: Value) -> Result<Value, String> {
    let parsed: FileReadArgs = dispatch::parse_args(args)?;
    Ok(Value::String(file_read_inner(&parsed.path)?))
}

// ===== file_write (atomic) =====

#[derive(serde::Deserialize)]
struct FileWriteArgs {
    path: String,
    content: String,
}

fn file_write_inner(path: &str, content: &str) -> Result<(), SystemError> {
    let p = ensure_absolute(path)?;
    if (content.len() as u64) > MAX_FILE_SIZE {
        return Err(SystemError::FileTooLarge {
            size: content.len() as u64,
            max: MAX_FILE_SIZE,
        });
    }
    if let Some(parent) = p.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }
    let tmp = tmp_path(&p);
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(content.as_bytes())?;
        f.sync_all()?;
    }
    fs::rename(&tmp, &p)?;
    Ok(())
}

fn file_write_dispatch(args: Value) -> Result<Value, String> {
    let parsed: FileWriteArgs = dispatch::parse_args(args)?;
    file_write_inner(&parsed.path, &parsed.content)?;
    Ok(Value::Null)
}

// ===== file_list =====

#[derive(serde::Deserialize)]
struct FileListArgs {
    dir: String,
}

fn file_list_inner(dir: &str) -> Result<Vec<FileEntry>, SystemError> {
    let d = ensure_absolute(dir)?;
    if !d.is_dir() {
        return Err(SystemError::NotADirectory(dir.to_string()));
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(&d)? {
        let entry = entry?;
        let meta = entry.metadata()?;
        out.push(FileEntry {
            name: entry.file_name().to_string_lossy().into_owned(),
            path: entry.path().to_string_lossy().into_owned(),
            is_dir: meta.is_dir(),
            size: meta.len(),
        });
    }
    out.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });
    Ok(out)
}

fn file_list_dispatch(args: Value) -> Result<Value, String> {
    let parsed: FileListArgs = dispatch::parse_args(args)?;
    let v = file_list_inner(&parsed.dir)?;
    serde_json::to_value(v).map_err(|e| e.to_string())
}

// ===== file_delete =====

#[derive(serde::Deserialize)]
struct FileDeleteArgs {
    path: String,
}

fn file_delete_inner(path: &str) -> Result<(), SystemError> {
    let p = ensure_absolute(path)?;
    let meta = match fs::symlink_metadata(&p) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.into()),
    };
    if meta.is_dir() {
        fs::remove_dir(&p)?;
    } else {
        fs::remove_file(&p)?;
    }
    Ok(())
}

fn file_delete_dispatch(args: Value) -> Result<Value, String> {
    let parsed: FileDeleteArgs = dispatch::parse_args(args)?;
    file_delete_inner(&parsed.path)?;
    Ok(Value::Null)
}

// ===== dir_ensure =====

#[derive(serde::Deserialize)]
struct DirEnsureArgs {
    dir: String,
}

fn dir_ensure_inner(dir: &str) -> Result<(), SystemError> {
    let d = ensure_absolute(dir)?;
    fs::create_dir_all(&d)?;
    Ok(())
}

fn dir_ensure_dispatch(args: Value) -> Result<Value, String> {
    let parsed: DirEnsureArgs = dispatch::parse_args(args)?;
    dir_ensure_inner(&parsed.dir)?;
    Ok(Value::Null)
}

// ===== file_exists =====

#[derive(serde::Deserialize)]
struct FileExistsArgs {
    path: String,
}

fn file_exists_inner(path: &str) -> Result<bool, SystemError> {
    let p = ensure_absolute(path)?;
    Ok(p.exists())
}

fn file_exists_dispatch(args: Value) -> Result<Value, String> {
    let parsed: FileExistsArgs = dispatch::parse_args(args)?;
    Ok(Value::Bool(file_exists_inner(&parsed.path)?))
}

// ===== clipboard_read =====

fn clipboard_read_inner() -> Result<String, SystemError> {
    let mut cb =
        arboard::Clipboard::new().map_err(|e| SystemError::Clipboard(e.to_string()))?;
    cb.get_text()
        .map_err(|e| SystemError::Clipboard(e.to_string()))
}

fn clipboard_read_dispatch(_args: Value) -> Result<Value, String> {
    Ok(Value::String(clipboard_read_inner()?))
}

// ===== clipboard_write =====

#[derive(serde::Deserialize)]
struct ClipboardWriteArgs {
    text: String,
}

fn clipboard_write_inner(text: &str) -> Result<(), SystemError> {
    let mut cb =
        arboard::Clipboard::new().map_err(|e| SystemError::Clipboard(e.to_string()))?;
    cb.set_text(text.to_owned())
        .map_err(|e| SystemError::Clipboard(e.to_string()))
}

fn clipboard_write_dispatch(args: Value) -> Result<Value, String> {
    let parsed: ClipboardWriteArgs = dispatch::parse_args(args)?;
    clipboard_write_inner(&parsed.text)?;
    Ok(Value::Null)
}

// ===== register =====

pub fn register(r: &mut dispatch::CommandRegistry) {
    r.register("file_read", file_read_dispatch);
    r.register("file_write", file_write_dispatch);
    r.register("file_list", file_list_dispatch);
    r.register("file_delete", file_delete_dispatch);
    r.register("dir_ensure", dir_ensure_dispatch);
    r.register("file_exists", file_exists_dispatch);
    r.register("clipboard_read", clipboard_read_dispatch);
    r.register("clipboard_write", clipboard_write_dispatch);
}

// ===== tests =====

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static SEQ: AtomicU64 = AtomicU64::new(0);

    fn unique_tmp_dir() -> PathBuf {
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        let pid = std::process::id();
        let dir = std::env::temp_dir().join(format!("toolbench-system-test-{pid}-{n}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn cleanup(p: &PathBuf) {
        let _ = fs::remove_dir_all(p);
    }

    #[test]
    fn file_read_rejects_relative_path() {
        let r = file_read_inner("not/absolute");
        assert!(matches!(r, Err(SystemError::PathNotAbsolute(_))));
    }

    #[test]
    fn file_read_missing_file_errors() {
        let dir = unique_tmp_dir();
        let p = dir.join("nope.txt");
        let r = file_read_inner(&p.to_string_lossy());
        assert!(matches!(r, Err(SystemError::Io(_))));
        cleanup(&dir);
    }

    #[test]
    fn file_write_then_read_roundtrip() {
        let dir = unique_tmp_dir();
        let path = dir.join("note.txt");
        file_write_inner(&path.to_string_lossy(), "hello world").unwrap();
        let content = file_read_inner(&path.to_string_lossy()).unwrap();
        assert_eq!(content, "hello world");
        // Atomic write should not leave a .tmp file behind.
        assert!(!tmp_path(&path).exists());
        cleanup(&dir);
    }

    #[test]
    fn file_write_creates_parent_dirs() {
        let dir = unique_tmp_dir();
        let path = dir.join("a/b/c/note.txt");
        file_write_inner(&path.to_string_lossy(), "deep").unwrap();
        assert!(path.exists());
        cleanup(&dir);
    }

    #[test]
    fn file_write_rejects_relative_path() {
        let r = file_write_inner("rel/path", "x");
        assert!(matches!(r, Err(SystemError::PathNotAbsolute(_))));
    }

    #[test]
    fn file_write_too_large_rejected_and_no_tmp_left() {
        let dir = unique_tmp_dir();
        let path = dir.join("big.txt");
        let huge = "x".repeat((MAX_FILE_SIZE + 1) as usize);
        let r = file_write_inner(&path.to_string_lossy(), &huge);
        assert!(matches!(r, Err(SystemError::FileTooLarge { .. })));
        // Crucially, no .tmp file should be left behind.
        assert!(!tmp_path(&path).exists());
        cleanup(&dir);
    }

    #[test]
    fn file_list_sorts_dirs_first_then_alphabetical() {
        let dir = unique_tmp_dir();
        fs::write(dir.join("z.txt"), "z").unwrap();
        fs::write(dir.join("a.txt"), "a").unwrap();
        fs::create_dir(dir.join("b_dir")).unwrap();
        let entries = file_list_inner(&dir.to_string_lossy()).unwrap();
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["b_dir", "a.txt", "z.txt"]);
        cleanup(&dir);
    }

    #[test]
    fn file_list_rejects_non_directory() {
        let dir = unique_tmp_dir();
        let file = dir.join("not_a_dir.txt");
        fs::write(&file, "x").unwrap();
        let r = file_list_inner(&file.to_string_lossy());
        assert!(matches!(r, Err(SystemError::NotADirectory(_))));
        cleanup(&dir);
    }

    #[test]
    fn file_delete_file_and_missing() {
        let dir = unique_tmp_dir();
        let f = dir.join("a.txt");
        fs::write(&f, "x").unwrap();
        file_delete_inner(&f.to_string_lossy()).unwrap();
        assert!(!f.exists());
        // Deleting a non-existent file is a no-op (returns Ok).
        file_delete_inner(&f.to_string_lossy()).unwrap();
        cleanup(&dir);
    }

    #[test]
    fn file_delete_empty_dir_succeeds() {
        let dir = unique_tmp_dir();
        let sub = dir.join("empty");
        fs::create_dir(&sub).unwrap();
        file_delete_inner(&sub.to_string_lossy()).unwrap();
        assert!(!sub.exists());
        cleanup(&dir);
    }

    #[test]
    fn dir_ensure_idempotent() {
        let dir = unique_tmp_dir();
        let target = dir.join("nested/dir");
        dir_ensure_inner(&target.to_string_lossy()).unwrap();
        assert!(target.is_dir());
        // Second call is a no-op.
        dir_ensure_inner(&target.to_string_lossy()).unwrap();
        assert!(target.is_dir());
        cleanup(&dir);
    }

    #[test]
    fn file_exists_reports_correctly() {
        let dir = unique_tmp_dir();
        let f = dir.join("a.txt");
        assert!(!file_exists_inner(&f.to_string_lossy()).unwrap());
        fs::write(&f, "x").unwrap();
        assert!(file_exists_inner(&f.to_string_lossy()).unwrap());
        cleanup(&dir);
    }

    #[test]
    fn dispatch_via_registry_round_trip() {
        use serde_json::json;
        let mut r = dispatch::CommandRegistry::new();
        register(&mut r);
        let path = std::env::temp_dir().join("__tb_dispatch_test.txt");
        let path_str = path.to_string_lossy().into_owned();
        // file_write
        r.dispatch(
            "file_write",
            json!({ "path": path_str, "content": "via-registry" }),
        )
        .unwrap();
        // file_read
        let v = r
            .dispatch("file_read", json!({ "path": path_str }))
            .unwrap();
        assert_eq!(v, json!("via-registry"));
        // cleanup
        let _ = fs::remove_file(&path);
    }

    // Clipboard tests: environment-dependent. Run with `cargo test -- --ignored`
    // on a system that has a clipboard (macOS / Windows desktop / Linux + X11/Wayland).
    #[test]
    #[ignore]
    fn clipboard_write_then_read() {
        let original = clipboard_read_inner().unwrap_or_default();
        clipboard_write_inner("tool-bench-test").unwrap();
        let got = clipboard_read_inner().unwrap();
        assert_eq!(got, "tool-bench-test");
        // Restore original so we don't pollute the user's clipboard.
        clipboard_write_inner(&original).unwrap();
    }
}
