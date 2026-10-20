//! Helpers for installing the running app bundle into /Applications.

use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The `.app` bundle that contains `exe`, if the process runs from one.
pub fn bundle_of(exe: &Path) -> Option<PathBuf> {
    exe.ancestors()
        .find(|p| p.extension().is_some_and(|e| e == "app"))
        .map(Path::to_path_buf)
}

/// Is the bundle already in `/Applications` or `~/Applications`?
pub fn is_in_applications(bundle: &Path, home: &Path) -> bool {
    match bundle.parent() {
        Some(p) => p == Path::new("/Applications") || p == home.join("Applications"),
        None => false,
    }
}

/// Copy `src` into `dest_dir` (replacing an existing copy) and clear the
/// quarantine flag on the result. Returns the new bundle path.
pub fn copy_bundle(src: &Path, dest_dir: &Path) -> io::Result<PathBuf> {
    std::fs::create_dir_all(dest_dir)?;
    let name = src
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "bundle has no name"))?;
    let dest = dest_dir.join(name);
    if dest.exists() {
        std::fs::remove_dir_all(&dest)?;
    }
    let status = Command::new("/usr/bin/ditto")
        .arg(src)
        .arg(&dest)
        .status()?;
    if !status.success() {
        return Err(io::Error::other("ditto failed"));
    }
    let _ = Command::new("/usr/bin/xattr")
        .args(["-dr", "com.apple.quarantine"])
        .arg(&dest)
        .status();
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_bundle_from_executable_path() {
        let exe = Path::new("/Volumes/MacClip/MacClip.app/Contents/MacOS/MacClip");
        assert_eq!(
            bundle_of(exe),
            Some(PathBuf::from("/Volumes/MacClip/MacClip.app"))
        );
        assert_eq!(bundle_of(Path::new("/repo/target/release/macclip")), None);
    }

    #[test]
    fn recognises_applications_folders() {
        let home = Path::new("/Users/me");
        assert!(is_in_applications(
            Path::new("/Applications/MacClip.app"),
            home
        ));
        assert!(is_in_applications(
            Path::new("/Users/me/Applications/MacClip.app"),
            home
        ));
        assert!(!is_in_applications(
            Path::new("/Users/me/Downloads/MacClip.app"),
            home
        ));
        assert!(!is_in_applications(
            Path::new("/Volumes/MacClip/MacClip.app"),
            home
        ));
        assert!(!is_in_applications(
            Path::new("/Applications/Sub/MacClip.app"),
            home
        ));
    }

    #[test]
    fn copy_replaces_existing_bundle() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("src/MacClip.app/Contents");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("new.txt"), "new").unwrap();
        let dest_dir = d.path().join("Applications");
        std::fs::create_dir_all(dest_dir.join("MacClip.app")).unwrap();
        std::fs::write(dest_dir.join("MacClip.app/old.txt"), "old").unwrap();

        let out = copy_bundle(&d.path().join("src/MacClip.app"), &dest_dir).unwrap();
        assert_eq!(out, dest_dir.join("MacClip.app"));
        assert!(out.join("Contents/new.txt").exists());
        assert!(!out.join("old.txt").exists());
    }
}
