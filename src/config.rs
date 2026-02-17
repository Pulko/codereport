use std::collections::HashMap;
use std::path::Path;
use std::str::FromStr;

const CONFIG_VERSION: u32 = 1;

/// Fixed tag set; unknown tags are rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tag {
    Todo,
    Refactor,
    Buggy,
    Critical,
}

impl Tag {
    pub fn as_str(&self) -> &'static str {
        match self {
            Tag::Todo => "todo",
            Tag::Refactor => "refactor",
            Tag::Buggy => "buggy",
            Tag::Critical => "critical",
        }
    }

    pub fn all() -> &'static [Tag] {
        &[Tag::Todo, Tag::Refactor, Tag::Buggy, Tag::Critical]
    }
}

fn to_ascii_lowercase(s: &str) -> String {
    s.chars().flat_map(|c| c.to_lowercase()).collect()
}

impl FromStr for Tag {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match to_ascii_lowercase(s).as_str() {
            "todo" => Ok(Tag::Todo),
            "refactor" => Ok(Tag::Refactor),
            "buggy" => Ok(Tag::Buggy),
            "critical" => Ok(Tag::Critical),
            _ => Err(format!("unknown tag: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Low,
    Medium,
    High,
    Blocking,
}

impl FromStr for Severity {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match to_ascii_lowercase(s).as_str() {
            "low" => Ok(Severity::Low),
            "medium" => Ok(Severity::Medium),
            "high" => Ok(Severity::High),
            "blocking" => Ok(Severity::Blocking),
            _ => Err(format!("unknown severity: {}", s)),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TagConfig {
    pub enabled: bool,
    pub severity: String,
    #[serde(default)]
    pub expires: Option<u32>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Config {
    pub version: u32,
    pub tags: HashMap<String, TagConfig>,
}

pub fn load_config(repo_root: &Path) -> Result<Config, String> {
    let path = repo_root.join(".codereports").join("config.yaml");
    let content = std::fs::read_to_string(&path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            "config not found. Run 'codereport init' first.".to_string()
        } else {
            format!("failed to read config: {}", e)
        }
    })?;
    let config: Config =
        serde_yaml::from_str(&content).map_err(|e| format!("invalid config.yaml: {}", e))?;
    if config.version != CONFIG_VERSION {
        return Err(format!(
            "unsupported config version: {} (expected {})",
            config.version, CONFIG_VERSION
        ));
    }
    for (name, tc) in &config.tags {
        Severity::from_str(&tc.severity).map_err(|e| format!("tag '{}': {}", name, e))?;
    }
    Ok(config)
}

/// Default config matching the spec (todo, refactor, buggy, critical with expires).
pub fn default_config() -> Config {
    let mut tags = HashMap::new();
    tags.insert(
        "todo".to_string(),
        TagConfig {
            enabled: true,
            severity: "low".to_string(),
            expires: None,
        },
    );
    tags.insert(
        "refactor".to_string(),
        TagConfig {
            enabled: true,
            severity: "medium".to_string(),
            expires: Some(180),
        },
    );
    tags.insert(
        "buggy".to_string(),
        TagConfig {
            enabled: true,
            severity: "high".to_string(),
            expires: Some(90),
        },
    );
    tags.insert(
        "critical".to_string(),
        TagConfig {
            enabled: true,
            severity: "blocking".to_string(),
            expires: Some(14),
        },
    );
    Config {
        version: CONFIG_VERSION,
        tags,
    }
}

/// Validate tag for add: must be known (Tag enum) and present in config with enabled true.
pub fn validate_tag_for_add(config: &Config, tag_str: &str) -> Result<Tag, String> {
    let tag = Tag::from_str(tag_str)?;
    let key = tag.as_str();
    let tc = config
        .tags
        .get(key)
        .ok_or_else(|| format!("tag '{}' is not defined in config", key))?;
    if !tc.enabled {
        return Err(format!("tag '{}' is disabled in config", key));
    }
    Ok(tag)
}

/// Get expiration days for a tag from config.
pub fn expires_days(config: &Config, tag: Tag) -> Option<u32> {
    config.tags.get(tag.as_str()).and_then(|tc| tc.expires)
}

/// Get severity for a tag from config.
pub fn severity(config: &Config, tag: Tag) -> Result<Severity, String> {
    config
        .tags
        .get(tag.as_str())
        .ok_or_else(|| format!("tag '{}' not in config", tag.as_str()))
        .and_then(|tc| Severity::from_str(&tc.severity))
}

pub fn write_default_config(repo_root: &Path) -> Result<(), String> {
    let dir = repo_root.join(".codereports");
    let path = dir.join("config.yaml");
    let config = default_config();
    let yaml = serde_yaml::to_string(&config).map_err(|e| format!("serialize config: {}", e))?;
    std::fs::write(&path, yaml).map_err(|e| format!("write config: {}", e))?;
    Ok(())
}
