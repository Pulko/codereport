use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct ResolvedAuthor {
    pub git: Option<String>,
    pub codeowner: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct BlameCacheEntry {
    path: String,
    start: u32,
    end: u32,
    oid: String,
    email: String,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct BlameCache {
    entries: Vec<BlameCacheEntry>,
}

fn blame_cache_path(repo_root: &Path) -> std::path::PathBuf {
    repo_root.join(".codereports").join(".blame-cache")
}

fn load_blame_cache(repo_root: &Path) -> BlameCache {
    let path = blame_cache_path(repo_root);
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return BlameCache::default(),
    };
    serde_json::from_str(&content).unwrap_or_default()
}

fn save_blame_cache(repo_root: &Path, cache: &BlameCache) {
    let path = blame_cache_path(repo_root);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(cache) {
        let _ = std::fs::write(&path, json);
    }
}

/// Returns the blob OID of the file at HEAD, or None if not in tree (e.g. new file).
fn blob_oid_at_head(repo: &git2::Repository, path: &str) -> Option<String> {
    let path_git = path.replace('\\', "/");
    let head = repo.head().ok()?;
    let commit = head.peel_to_commit().ok()?;
    let tree = commit.tree().ok()?;
    let entry = tree.get_path(Path::new(&path_git)).ok()?;
    Some(entry.id().to_string())
}

/// (1) CODEOWNERS: best match for path. (2) Fallback: git blame for line range (with cache).
pub fn resolve_author(repo_root: &Path, path: &str, start: u32, end: u32) -> ResolvedAuthor {
    let mut author = ResolvedAuthor::default();

    // Try CODEOWNERS first
    if let Some(codeowner) = codeowner_for_path(repo_root, path) {
        author.codeowner = Some(codeowner);
    }

    let repo = match git2::Repository::open(repo_root) {
        Ok(r) => r,
        Err(_) => return author,
    };
    let file_path = repo_root.join(path);
    if !file_path.exists() {
        return author;
    }

    let path_for_blame = Path::new(path);
    let oid_opt = blob_oid_at_head(&repo, path);

    // Try cache lookup (only when we have a blob OID so cache is valid)
    if let Some(ref oid) = oid_opt {
        let cache = load_blame_cache(repo_root);
        if let Some(entry) = cache.entries.iter().find(|e| {
            e.path == path && e.start == start && e.end == end && e.oid == *oid
        }) {
            if author.git.is_none() && !entry.email.is_empty() {
                author.git = Some(entry.email.clone());
            }
            return author;
        }
    }

    // Cache miss or no OID: run blame
    let mut opts = git2::BlameOptions::new();
    opts.min_line(start as usize)
        .max_line(end.max(start) as usize);
    let email_opt = if let Ok(blame) = repo.blame_file(path_for_blame, Some(&mut opts)) {
        let line_no = (start as usize).max(1);
        blame
            .get_line(line_no)
            .and_then(|hunk| hunk.final_signature().email().map(|s| s.to_string()))
    } else {
        None
    };
    if author.git.is_none() && email_opt.is_some() {
        author.git = email_opt.clone();
    }

    // Persist to cache (only when we have OID; skip for new/uncommitted files)
    if let (Some(oid), Some(email)) = (oid_opt, email_opt) {
        let mut cache = load_blame_cache(repo_root);
        // Remove existing entry with same key if any
        cache.entries.retain(|e| !(e.path == path && e.start == start && e.end == end && e.oid == oid));
        cache.entries.push(BlameCacheEntry {
            path: path.to_string(),
            start,
            end,
            oid,
            email,
        });
        save_blame_cache(repo_root, &cache);
    }

    author
}

/// Find CODEOWNERS: .git/CODEOWNERS or repo root CODEOWNERS.
/// Returns the owner string for the best (last) matching rule (e.g. "@backend" or "user@example.com").
fn codeowner_for_path(repo_root: &Path, path: &str) -> Option<String> {
    let path_forward = path.replace('\\', "/");
    let content = read_codeowners(repo_root)?;
    let mut last_match: Option<String> = None;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut tokens = line.split_whitespace();
        let pattern = tokens.next()?;
        let owners: Vec<&str> = tokens.collect();
        if owners.is_empty() {
            continue;
        }
        if codeowners_pattern_matches(pattern, &path_forward) {
            last_match = Some(owners[0].to_string());
        }
    }
    last_match
}

fn read_codeowners(repo_root: &Path) -> Option<String> {
    let in_dot_git = repo_root.join(".git").join("CODEOWNERS");
    if in_dot_git.exists() {
        return std::fs::read_to_string(&in_dot_git).ok();
    }
    let at_root = repo_root.join("CODEOWNERS");
    if at_root.exists() {
        return std::fs::read_to_string(&at_root).ok();
    }
    None
}

/// Simple CODEOWNERS-style match: pattern can be path prefix or suffix.
/// - "/path" or "path" matches if path starts with it (after stripping leading /).
/// - "*" and "**" not fully implemented; we do prefix/suffix and exact.
fn codeowners_pattern_matches(pattern: &str, path: &str) -> bool {
    let pattern = pattern.trim_start_matches('/');
    let path = path.trim_start_matches('/');
    if pattern.is_empty() {
        return false;
    }
    if pattern == "*" || path == pattern {
        return true;
    }
    if pattern.ends_with('/') {
        return path.starts_with(pattern) || path.starts_with(pattern.trim_end_matches('/'));
    }
    path.starts_with(pattern)
        || path == pattern
        || path.ends_with(pattern)
        || path.contains(&format!("/{}", pattern))
}
