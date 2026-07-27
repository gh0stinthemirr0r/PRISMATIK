//! Portable directory-tree backup and restore.

use prismatik_determinism::{Clock, SystemClock};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Backup failures.
#[derive(Debug, Error)]
pub enum BackupError {
    /// Filesystem operation failed.
    #[error("backup I/O: {0}")]
    Io(#[from] std::io::Error),
    /// Source directory does not exist.
    #[error("backup source does not exist: {0}")]
    MissingSource(PathBuf),
}

/// Export a profile directory to a timestamped child of `backup_parent`.
pub fn export_backup(
    source: impl AsRef<Path>,
    backup_parent: impl AsRef<Path>,
) -> Result<PathBuf, BackupError> {
    if !source.as_ref().is_dir() {
        return Err(BackupError::MissingSource(source.as_ref().to_path_buf()));
    }
    std::fs::create_dir_all(backup_parent.as_ref())?;
    let timestamp = SystemClock::new().now().unix_timestamp_nanos();
    let destination = backup_parent.as_ref().join(format!("backup-{timestamp}"));
    copy_tree(source.as_ref(), &destination)?;
    Ok(destination)
}

/// Import a previously exported directory tree.
pub fn import_backup(
    backup: impl AsRef<Path>,
    destination: impl AsRef<Path>,
) -> Result<(), BackupError> {
    if !backup.as_ref().is_dir() {
        return Err(BackupError::MissingSource(backup.as_ref().to_path_buf()));
    }
    copy_tree(backup.as_ref(), destination.as_ref())?;
    Ok(())
}

fn copy_tree(source: &Path, destination: &Path) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(destination)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&source_path, &destination_path)?;
        } else {
            std::fs::copy(source_path, destination_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use prismatik_storage::{DesktopProfile, PreferenceStore, SqliteBackend};
    use tempfile::tempdir;

    #[test]
    fn sqlite_profile_survives_export_then_import() {
        let temp = tempdir().unwrap();
        let original = DesktopProfile::new(temp.path().join("profile-n"));
        original.ensure_dirs().unwrap();
        {
            let db = SqliteBackend::open(original.sqlite_path()).unwrap();
            db.set_pref("generation", "\"n\"").unwrap();
        }
        let exported = export_backup(&original.root, temp.path().join("exports")).unwrap();
        let restored = DesktopProfile::new(temp.path().join("profile-n-plus-one"));
        import_backup(exported, &restored.root).unwrap();
        let restored_db = SqliteBackend::open(restored.sqlite_path()).unwrap();
        assert_eq!(
            restored_db.get_pref("generation").unwrap().as_deref(),
            Some("\"n\"")
        );
    }
}
