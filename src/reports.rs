use std::path::Path;

const REPORTS_VERSION: u32 = 1;
const REPORTS_FILENAME: &str = "reports.yaml";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReportEntry {
    pub id: String,
    pub path: String,
    pub range: LineRange,
    pub tag: String,
    pub message: String,
    pub author: Author,
    pub created_at: String,
    pub expires_at: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LineRange {
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Author {
    pub git: Option<String>,
    pub codeowner: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Reports {
    pub version: u32,
    pub entries: Vec<ReportEntry>,
}

impl Reports {
    /// Parse max numeric id from entries (e.g. CR-000042 -> 42). Returns 0 if none.
    pub fn max_id(&self) -> u32 {
        self.entries
            .iter()
            .filter_map(|e| parse_report_id(&e.id))
            .max()
            .unwrap_or(0)
    }

    pub fn next_id(&self) -> String {
        let n = self.max_id() + 1;
        format!("CR-{:06}", n)
    }

    pub fn add_entry(&mut self, entry: ReportEntry) {
        self.entries.push(entry);
    }

    pub fn delete_by_id(&mut self, id: &str) -> bool {
        if let Some(pos) = self.entries.iter().position(|e| e.id == id) {
            self.entries.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn resolve_by_id(&mut self, id: &str) -> bool {
        if let Some(e) = self.entries.iter_mut().find(|e| e.id == id) {
            e.status = "resolved".to_string();
            true
        } else {
            false
        }
    }

    pub fn by_id(&self, id: &str) -> Option<&ReportEntry> {
        self.entries.iter().find(|e| e.id == id)
    }
}

fn parse_report_id(id: &str) -> Option<u32> {
    id.strip_prefix("CR-")?.parse().ok()
}

pub fn load_reports(repo_root: &Path) -> Result<Reports, String> {
    let path = repo_root.join(".codereports").join(REPORTS_FILENAME);
    if !path.exists() {
        return Ok(Reports {
            version: REPORTS_VERSION,
            entries: vec![],
        });
    }
    let content = std::fs::read_to_string(&path).map_err(|e| format!("read reports: {}", e))?;
    let reports: Reports =
        serde_yaml::from_str(&content).map_err(|e| format!("invalid reports.yaml: {}", e))?;
    if reports.version != REPORTS_VERSION {
        return Err(format!(
            "unsupported reports version: {} (expected {})",
            reports.version, REPORTS_VERSION
        ));
    }
    Ok(reports)
}

/// Atomic write: temp file in .codereports then rename.
pub fn save_reports(repo_root: &Path, reports: &Reports) -> Result<(), String> {
    let dir = repo_root.join(".codereports");
    let dest = dir.join(REPORTS_FILENAME);
    let yaml = serde_yaml::to_string(reports).map_err(|e| format!("serialize reports: {}", e))?;
    let mut temp = dir.join(REPORTS_FILENAME);
    temp.set_extension("yaml.tmp");
    std::fs::write(&temp, yaml).map_err(|e| format!("write reports: {}", e))?;
    std::fs::rename(&temp, &dest).map_err(|e| format!("rename reports: {}", e))?;
    Ok(())
}

/// Build Author for serialization from resolved author.
pub fn author_from_resolved(git: Option<String>, codeowner: Option<String>) -> Author {
    Author { git, codeowner }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_id_monotonic() {
        let mut r = Reports {
            version: 1,
            entries: vec![],
        };
        assert_eq!(r.next_id(), "CR-000001");
        r.entries.push(ReportEntry {
            id: "CR-000001".to_string(),
            path: "x".to_string(),
            range: LineRange { start: 1, end: 2 },
            tag: "todo".to_string(),
            message: "m".to_string(),
            author: Author {
                git: None,
                codeowner: None,
            },
            created_at: "2026-01-01".to_string(),
            expires_at: None,
            status: "open".to_string(),
        });
        assert_eq!(r.next_id(), "CR-000002");
    }
}
