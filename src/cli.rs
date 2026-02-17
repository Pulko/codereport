use crate::author;
use crate::config;
use crate::html;
use crate::repo;
use crate::reports;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;
use std::str::FromStr;

#[derive(Parser, Debug)]
#[command(name = "codereport", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Initialize .codereports/ with config and schema
    Init,
    /// Add a new report
    Add {
        /// Location as path:start-end (e.g. src/foo.rs:42-88)
        location: String,
        #[arg(long)]
        tag: String,
        #[arg(long)]
        message: String,
    },
    /// List reports with optional filters
    List {
        #[arg(long)]
        tag: Option<String>,
        #[arg(long)]
        status: Option<String>,
    },
    /// Delete a report by ID
    Delete { id: String },
    /// Mark a report as resolved
    Resolve { id: String },
    /// CI check: fail if blocking or expired open reports
    Check,
    /// Generate HTML dashboard
    Html {
        #[arg(long)]
        no_open: bool,
    },
}

pub fn run() -> ExitCode {
    let cli = Cli::parse();
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let repo_root = match repo::find_repo_root(&cwd) {
        Some(r) => r,
        None => {
            eprintln!("error: not inside a git repository (no .git found)");
            return ExitCode::from(1);
        }
    };

    match cli.command {
        Command::Init => cmd_init(&repo_root),
        Command::Add {
            location,
            tag,
            message,
        } => cmd_add(&repo_root, &location, &tag, &message),
        Command::List { tag, status } => cmd_list(&repo_root, tag.as_deref(), status.as_deref()),
        Command::Delete { id } => cmd_delete(&repo_root, &id),
        Command::Resolve { id } => cmd_resolve(&repo_root, &id),
        Command::Check => cmd_check(&repo_root),
        Command::Html { no_open } => cmd_html(&repo_root, no_open),
    }
}

const GITIGNORE_BLOCK: &str = "\n# codereport (generated dashboard and local blame cache)\n.codereports/html/\n.codereports/.blame-cache\n";

fn ensure_root_gitignore(repo_root: &std::path::Path) -> Result<(), String> {
    let root_gitignore = repo_root.join(".gitignore");
    let content = if root_gitignore.exists() {
        std::fs::read_to_string(&root_gitignore).map_err(|e| e.to_string())?
    } else {
        String::new()
    };
    let already_has = content.contains(".codereports/html/") || content.contains("# codereport");
    if already_has {
        return Ok(());
    }
    let block = GITIGNORE_BLOCK.trim_start_matches('\n');
    let new_content = if content.trim().is_empty() {
        block.to_string()
    } else {
        format!("{}\n{}", content.trim_end_matches('\n'), block)
    };
    std::fs::write(&root_gitignore, new_content).map_err(|e| e.to_string())?;
    Ok(())
}

fn cmd_init(repo_root: &std::path::Path) -> ExitCode {
    let dir = repo_root.join(".codereports");
    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!("error: failed to create .codereports: {}", e);
        return ExitCode::from(1);
    }
    if let Err(e) = ensure_root_gitignore(repo_root) {
        eprintln!("error: failed to update repo root .gitignore: {}", e);
        return ExitCode::from(1);
    }
    let config_path = dir.join("config.yaml");
    if !config_path.exists() {
        if let Err(e) = config::write_default_config(repo_root) {
            eprintln!("error: failed to write config.yaml: {}", e);
            return ExitCode::from(1);
        }
    }
    let schema_path = dir.join("schema.json");
    if !schema_path.exists() {
        if let Err(e) = std::fs::write(&schema_path, config::default_schema_json()) {
            eprintln!("error: failed to write schema.json: {}", e);
            return ExitCode::from(1);
        }
    }
    println!("Initialized .codereports/ in {}", repo_root.display());
    ExitCode::SUCCESS
}

/// Parse "path:start-end" into (path, start, end).
fn parse_location(location: &str) -> Result<(String, u32, u32), String> {
    let colon = location
        .rfind(':')
        .ok_or_else(|| "expected path:start-end".to_string())?;
    let (path_part, range_part) = location.split_at(colon);
    let range_part = range_part.trim_start_matches(':');
    let path = path_part.trim();
    if path.is_empty() {
        return Err("path is empty".to_string());
    }
    let path = path.replace('\\', "/");
    let dash = range_part
        .find('-')
        .ok_or_else(|| "expected start-end range".to_string())?;
    let (start_s, end_s) = range_part.split_at(dash);
    let start: u32 = start_s
        .trim()
        .parse()
        .map_err(|_| "invalid start line".to_string())?;
    let end: u32 = end_s
        .trim_start_matches('-')
        .trim()
        .parse()
        .map_err(|_| "invalid end line".to_string())?;
    if start == 0 || end < start {
        return Err("invalid range (start >= 1, end >= start)".to_string());
    }
    Ok((path, start, end))
}

fn cmd_add(repo_root: &std::path::Path, location: &str, tag_str: &str, message: &str) -> ExitCode {
    let (path, start, end) = match parse_location(location) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: {}", e);
            return ExitCode::from(1);
        }
    };

    let cfg = match config::load_config(repo_root) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {}", e);
            return ExitCode::from(1);
        }
    };

    let tag = match config::validate_tag_for_add(&cfg, tag_str) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: {}", e);
            return ExitCode::from(1);
        }
    };

    let mut reports_list = match reports::load_reports(repo_root) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {}", e);
            return ExitCode::from(1);
        }
    };

    let author_resolved = author::resolve_author(repo_root, &path, start, end);
    let created_at = chrono::Local::now().format("%Y-%m-%d").to_string();
    let expires_at = config::expires_days(&cfg, tag).map(|days| {
        let d = chrono::Local::now() + chrono::Duration::days(days as i64);
        d.format("%Y-%m-%d").to_string()
    });

    let id = reports_list.next_id();
    let entry = reports::ReportEntry {
        id: id.clone(),
        path: path.clone(),
        range: reports::LineRange { start, end },
        tag: tag.as_str().to_string(),
        message: message.to_string(),
        author: reports::Author {
            git: author_resolved.git,
            codeowner: author_resolved.codeowner,
        },
        created_at,
        expires_at,
        status: "open".to_string(),
    };
    reports_list.add_entry(entry);

    match reports::save_reports(repo_root, &reports_list) {
        Ok(()) => {
            println!("Added {} {}", id, path);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {}", e);
            ExitCode::from(1)
        }
    }
}

fn cmd_list(
    repo_root: &std::path::Path,
    tag_filter: Option<&str>,
    status_filter: Option<&str>,
) -> ExitCode {
    let reports_list = match reports::load_reports(repo_root) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {}", e);
            return ExitCode::from(1);
        }
    };

    let entries = reports_list.entries.iter().filter(|e| {
        let tag_ok = tag_filter
            .map(|t| e.tag.eq_ignore_ascii_case(t))
            .unwrap_or(true);
        let status_ok = status_filter
            .map(|s| e.status.eq_ignore_ascii_case(s))
            .unwrap_or(true);
        tag_ok && status_ok
    });

    for e in entries {
        let range = format!("{}-{}", e.range.start, e.range.end);
        println!(
            "{}  {}  {}  {}  {}  {}",
            e.id, e.path, range, e.tag, e.status, e.message
        );
    }
    ExitCode::SUCCESS
}

fn cmd_delete(repo_root: &std::path::Path, id: &str) -> ExitCode {
    let mut reports_list = match reports::load_reports(repo_root) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {}", e);
            return ExitCode::from(1);
        }
    };

    if !reports_list.delete_by_id(id) {
        eprintln!("error: report not found: {}", id);
        return ExitCode::from(1);
    }

    match reports::save_reports(repo_root, &reports_list) {
        Ok(()) => {
            println!("Deleted {}", id);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {}", e);
            ExitCode::from(1)
        }
    }
}

fn cmd_resolve(repo_root: &std::path::Path, id: &str) -> ExitCode {
    let mut reports_list = match reports::load_reports(repo_root) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {}", e);
            return ExitCode::from(1);
        }
    };

    if !reports_list.resolve_by_id(id) {
        eprintln!("error: report not found: {}", id);
        return ExitCode::from(1);
    }

    match reports::save_reports(repo_root, &reports_list) {
        Ok(()) => {
            println!("Resolved {}", id);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {}", e);
            ExitCode::from(1)
        }
    }
}

fn cmd_check(repo_root: &std::path::Path) -> ExitCode {
    let cfg = match config::load_config(repo_root) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {}", e);
            return ExitCode::from(1);
        }
    };
    let reports_list = match reports::load_reports(repo_root) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {}", e);
            return ExitCode::from(1);
        }
    };

    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let mut violations = Vec::new();
    for e in &reports_list.entries {
        if e.status != "open" {
            continue;
        }
        let tag = match config::Tag::from_str(e.tag.as_str()) {
            Ok(t) => t,
            _ => continue,
        };
        let severity = match config::severity(&cfg, tag) {
            Ok(s) => s,
            _ => continue,
        };
        let blocking = severity == config::Severity::Blocking;
        let expired = e
            .expires_at
            .as_ref()
            .map(|d| d.as_str() < today.as_str())
            .unwrap_or(false);
        if blocking || expired {
            violations.push((
                e.id.as_str(),
                e.path.as_str(),
                e.tag.as_str(),
                e.message.as_str(),
            ));
        }
    }

    if violations.is_empty() {
        return ExitCode::SUCCESS;
    }
    for (id, path, tag, message) in &violations {
        eprintln!("{}  {}  {}  {}", id, path, tag, message);
    }
    ExitCode::from(1)
}

fn cmd_html(repo_root: &std::path::Path, no_open: bool) -> ExitCode {
    let reports_list = match reports::load_reports(repo_root) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {}", e);
            return ExitCode::from(1);
        }
    };
    let index_path = match html::generate_html(repo_root, &reports_list) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {}", e);
            return ExitCode::from(1);
        }
    };
    println!("Generated {}", index_path.display());
    if !no_open {
        if let Err(e) = open::that(&index_path) {
            eprintln!("warning: could not open browser: {}", e);
        }
    }
    ExitCode::SUCCESS
}
