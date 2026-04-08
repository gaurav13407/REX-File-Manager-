use std::path::{Path, PathBuf};

pub fn get_trash_dir() -> PathBuf {
    let mut path = dirs::home_dir().expect("Cannot find home directory");
    path.push(".rex_trash");
    path
}

/// Move `src` into ~/.rex_trash.
/// Returns Ok(()) on success, Err with a message on failure.
pub fn move_to_trash(src: &Path) -> Result<(), String> {
    let trash_dir = get_trash_dir();
    std::fs::create_dir_all(&trash_dir)
        .map_err(|e| format!("Cannot create trash dir: {e}"))?;

    let file_name = src
        .file_name()
        .ok_or_else(|| "Path has no filename".to_string())?;

    let mut dest = trash_dir;
    dest.push(file_name);

    // If a file with the same name already exists in trash, append a counter.
    let dest = unique_dest(dest);

    std::fs::rename(src, &dest)
        .map_err(|e| format!("Move to trash failed: {e}"))
}

/// Make a destination path unique by appending `_1`, `_2`, … if needed.
/// Public so callers can predict the final path before actually moving.
pub fn unique_dest_pub(path: PathBuf) -> PathBuf {
    unique_dest(path)
}

fn unique_dest(path: PathBuf) -> PathBuf {
    if !path.exists() {
        return path;
    }
    let stem = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let ext = path
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    let parent = path.parent().unwrap().to_path_buf();

    let mut n = 1u32;
    loop {
        let candidate = parent.join(format!("{stem}_{n}{ext}"));
        if !candidate.exists() {
            return candidate;
        }
        n += 1;
    }
}
