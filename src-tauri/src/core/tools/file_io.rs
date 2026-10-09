use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub fn atomic_write(path: &Path, content: impl AsRef<[u8]>) -> io::Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let temporary = temporary_path(path, parent);
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(content.as_ref())?;
        file.sync_all()?;
        drop(file);
        if let Ok(metadata) = fs::metadata(path) {
            fs::set_permissions(&temporary, metadata.permissions())?;
        }
        replace_file(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn temporary_path(path: &Path, parent: &Path) -> PathBuf {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("file");
    parent.join(format!(".{name}.anya-{}.tmp", uuid::Uuid::new_v4()))
}

#[cfg(windows)]
fn replace_file(source: &Path, target: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let target: Vec<u16> = target.as_os_str().encode_wide().chain(Some(0)).collect();
    unsafe {
        MoveFileExW(
            PCWSTR(source.as_ptr()),
            PCWSTR(target.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
        .map_err(|error| {
            let code = error.code().0 as u32;
            if code & 0xffff_0000 == 0x8007_0000 {
                io::Error::from_raw_os_error((code & 0xffff) as i32)
            } else {
                io::Error::other(error)
            }
        })
    }
}

#[cfg(not(windows))]
fn replace_file(source: &Path, target: &Path) -> io::Result<()> {
    fs::rename(source, target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomically_replaces_existing_file() {
        let root = std::env::temp_dir().join(format!("peek-atomic-{}", uuid::Uuid::new_v4()));
        let path = root.join("nested").join("file.txt");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "old").unwrap();

        atomic_write(&path, "new").unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "new");
        assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
        let _ = fs::remove_dir_all(root);
    }
}
