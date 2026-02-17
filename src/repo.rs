use std::path::{Path, PathBuf};

/// Walks up from `cwd` until a directory containing `.git` is found.
/// Returns that directory (repo root), or `None` if not inside a git repo.
pub fn find_repo_root(cwd: &Path) -> Option<PathBuf> {
    let mut current = cwd.canonicalize().ok()?;
    loop {
        if current.join(".git").exists() {
            return Some(current);
        }
        current = current.parent()?.to_path_buf();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_repo_root_finds_current_repo() {
        let cwd = std::env::current_dir().unwrap();
        let root = find_repo_root(&cwd);
        // When run from a git repo (e.g. tutorials or codereport), should find it
        assert!(root.is_some());
        let root = root.unwrap();
        assert!(root.join(".git").exists());
    }

    #[test]
    fn find_repo_root_from_subdir() {
        let cwd = std::env::current_dir().unwrap();
        let root = find_repo_root(&cwd).expect("must be in repo for test");
        let sub = root.join("codereport").join("src");
        if sub.exists() {
            let found = find_repo_root(&sub).unwrap();
            assert_eq!(found, root);
        }
    }
}
