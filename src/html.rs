use crate::reports::Reports;
use chrono::Utc;
use std::collections::HashMap;
use std::path::Path;

struct DashboardStats {
    total: usize,
    open: usize,
    resolved: usize,
    critical: usize,
    expired: usize,
    expiring_soon: usize,
}

pub fn generate_html(repo_root: &Path, reports: &Reports) -> Result<std::path::PathBuf, String> {
    let today = Utc::now().format("%Y-%m-%d").to_string();
    let stats = compute_stats(reports, &today);
    let (tag_counts, file_counts, heatmap) = compute_chart_data(reports);

    let max_tag_count = tag_counts.iter().map(|(_, c)| *c).max().unwrap_or(1) as f64;
    let tag_bars: String = tag_counts
        .iter()
        .map(|(tag, count)| {
            let pct = (*count as f64 / max_tag_count * 100.0).min(100.0);
            let tag_class = tag_slug(tag);
            format!(
                r#"<div class="bar-row"><span class="bar-label tag-dot {}">{}</span><div class="bar-wrap" title="{}"><div class="bar {}" style="width:{}%"></div></div><span class="bar-value">{}</span></div>"#,
                tag_class, escape_html(tag), count, tag_class, pct, count
            )
        })
        .collect();

    let tags: Vec<&String> = tag_counts.iter().map(|(t, _)| t).collect();
    let files: Vec<&String> = file_counts.iter().take(30).map(|(p, _)| p).collect();
    let heatmap_rows: String = {
        let mut rows = String::new();
        for path in &files {
            rows.push_str("<tr>");
            rows.push_str(&format!(
                "<td class=\"path-cell\" title=\"{}\">{}</td>",
                escape_attr(path),
                escape_html(path)
            ));
            for tag in &tags {
                let count = heatmap
                    .get(*path)
                    .and_then(|m| m.get(*tag))
                    .copied()
                    .unwrap_or(0);
                let tag_class = tag_slug(tag);
                let (heat_class, title) = if count > 0 {
                    let level = if count >= 3 { "hi" } else if count >= 2 { "mid" } else { "lo" };
                    (format!("heat {} {}", level, tag_class), format!("{}: {}", tag, count))
                } else {
                    ("".to_string(), "".to_string())
                };
                rows.push_str(&format!(
                    "<td class=\"{}\" title=\"{}\">{}</td>",
                    heat_class, title, if count > 0 { count.to_string() } else { "—".to_string() }
                ));
            }
            rows.push_str("</tr>");
        }
        rows
    };

    let tag_headers: String = tags
        .iter()
        .map(|t| {
            let slug = tag_slug(t);
            format!("<th class=\"tag-th {}\">{}</th>", slug, escape_html(t))
        })
        .collect();

    let html = format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Code Reports</title>
<style>
:root {{
  --bg: #0b0c0e;
  --surface: #16181c;
  --border: #2a2d33;
  --muted: #6b7280;
  --text: #e5e7eb;
  --text-strong: #f9fafb;
  --accent: #3b82f6;
  --accent-dim: #1e3a5f;
  --success: #10b981;
  --success-dim: #064e3b;
  --warn: #f59e0b;
  --warn-dim: #451a03;
  --danger: #ef4444;
  --danger-dim: #450a0a;
  --radius: 8px;
  --font: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
}}
* {{ box-sizing: border-box; }}
body {{ font-family: var(--font); margin: 0; background: var(--bg); color: var(--text); font-size: 14px; line-height: 1.5; }}
.page {{ max-width: 1200px; margin: 0 auto; padding: 24px; }}

.header {{ margin-bottom: 24px; }}
.header h1 {{ font-size: 1.5rem; font-weight: 600; color: var(--text-strong); margin: 0 0 4px 0; }}
.header p {{ color: var(--muted); margin: 0; font-size: 13px; }}

.stats {{ display: grid; grid-template-columns: repeat(auto-fill, minmax(120px, 1fr)); gap: 12px; margin-bottom: 24px; }}
.stat {{ background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius); padding: 14px 16px; }}
.stat-value {{ font-size: 1.5rem; font-weight: 700; color: var(--text-strong); font-variant-numeric: tabular-nums; }}
.stat-label {{ font-size: 11px; text-transform: uppercase; letter-spacing: 0.04em; color: var(--muted); margin-top: 2px; }}
.stat.danger .stat-value {{ color: var(--danger); }}
.stat.warn .stat-value {{ color: var(--warn); }}
.stat.success .stat-value {{ color: var(--success); }}

.section {{ margin-bottom: 24px; }}
.section-title {{ font-size: 11px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.06em; color: var(--muted); margin-bottom: 12px; }}

.bar-rows {{ display: flex; flex-direction: column; gap: 8px; }}
.bar-row {{ display: flex; align-items: center; gap: 12px; }}
.bar-label {{ width: 86px; flex-shrink: 0; font-size: 13px; color: var(--text); }}
.bar-label.tag-dot::before {{ content: ''; display: inline-block; width: 6px; height: 6px; border-radius: 50%; margin-right: 6px; vertical-align: 0.15em; }}
.bar-label.tag-dot.critical::before {{ background: var(--danger); }}
.bar-label.tag-dot.buggy::before {{ background: var(--warn); }}
.bar-label.tag-dot.refactor::before {{ background: #8b5cf6; }}
.bar-label.tag-dot.todo::before {{ background: var(--muted); }}
.bar-wrap {{ width: 160px; flex-shrink: 0; height: 8px; background: var(--border); border-radius: 4px; overflow: hidden; }}
.bar {{ height: 100%; border-radius: 4px; min-width: 2px; transition: width 0.2s ease; }}
.bar.critical {{ background: var(--danger); }}
.bar.buggy {{ background: var(--warn); }}
.bar.refactor {{ background: #8b5cf6; }}
.bar.todo {{ background: var(--muted); }}
.bar-value {{ width: 2.2em; text-align: right; font-variant-numeric: tabular-nums; font-size: 13px; color: var(--muted); }}

.heatmap-wrap {{ background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius); overflow: auto; }}
.heatmap {{ border-collapse: collapse; width: 100%; font-size: 13px; }}
.heatmap th, .heatmap td {{ padding: 8px 10px; border-bottom: 1px solid var(--border); }}
.heatmap thead th {{ text-align: left; font-weight: 600; color: var(--muted); font-size: 11px; text-transform: uppercase; letter-spacing: 0.04em; background: var(--surface); position: sticky; top: 0; z-index: 1; }}
.heatmap thead th.tag-th {{ text-align: center; min-width: 44px; }}
.heatmap .path-cell {{ max-width: 280px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--text); }}
.heatmap tbody tr:hover {{ background: rgba(59, 130, 246, 0.06); }}
.heatmap tbody td {{ text-align: center; color: var(--muted); font-variant-numeric: tabular-nums; }}
.heatmap .heat {{ font-weight: 600; color: var(--text-strong); }}
.heatmap .heat.lo.critical {{ background: rgba(239, 68, 68, 0.2); color: #fca5a5; }}
.heatmap .heat.mid.critical {{ background: rgba(239, 68, 68, 0.35); color: #fecaca; }}
.heatmap .heat.hi.critical {{ background: rgba(239, 68, 68, 0.5); color: #fee2e2; }}
.heatmap .heat.lo.buggy {{ background: rgba(245, 158, 11, 0.2); color: #fcd34d; }}
.heatmap .heat.mid.buggy {{ background: rgba(245, 158, 11, 0.35); color: #fde68a; }}
.heatmap .heat.hi.buggy {{ background: rgba(245, 158, 11, 0.5); color: #fef3c7; }}
.heatmap .heat.lo.refactor {{ background: rgba(139, 92, 246, 0.2); color: #c4b5fd; }}
.heatmap .heat.mid.refactor {{ background: rgba(139, 92, 246, 0.35); color: #ddd6fe; }}
.heatmap .heat.hi.refactor {{ background: rgba(139, 92, 246, 0.5); color: #ede9fe; }}
.heatmap .heat.lo.todo {{ background: rgba(107, 114, 128, 0.25); color: #9ca3af; }}
.heatmap .heat.mid.todo {{ background: rgba(107, 114, 128, 0.4); color: #d1d5db; }}
.heatmap .heat.hi.todo {{ background: rgba(107, 114, 128, 0.55); color: #e5e7eb; }}
</style>
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap" rel="stylesheet">
</head>
<body>
<div class="page">
<header class="header">
<h1>Code Reports</h1>
<p>Generated from .codereports/reports.yaml · {}</p>
</header>

<div class="stats">
<div class="stat"><div class="stat-value">{}</div><div class="stat-label">Total</div></div>
<div class="stat success"><div class="stat-value">{}</div><div class="stat-label">Open</div></div>
<div class="stat"><div class="stat-value">{}</div><div class="stat-label">Resolved</div></div>
<div class="stat danger"><div class="stat-value">{}</div><div class="stat-label">Critical</div></div>
<div class="stat danger"><div class="stat-value">{}</div><div class="stat-label">Expired</div></div>
<div class="stat warn"><div class="stat-value">{}</div><div class="stat-label">Expiring soon</div></div>
</div>

<div class="section">
<div class="section-title">By tag</div>
<div class="bar-rows">
{}
</div>
</div>

<div class="section">
<div class="section-title">File × tag heatmap (top 30 files)</div>
<div class="heatmap-wrap">
<table class="heatmap">
<thead><tr><th>File</th>{}</tr></thead>
<tbody>
{}
</tbody>
</table>
</div>
</div>
</div>
</body>
</html>
"##,
        escape_html(&today),
        stats.total,
        stats.open,
        stats.resolved,
        stats.critical,
        stats.expired,
        stats.expiring_soon,
        tag_bars,
        tag_headers,
        heatmap_rows
    );

    let out_dir = repo_root.join(".codereports").join("html");
    std::fs::create_dir_all(&out_dir).map_err(|e| format!("create html dir: {}", e))?;
    let index_path = out_dir.join("index.html");
    std::fs::write(&index_path, html).map_err(|e| format!("write index.html: {}", e))?;
    Ok(index_path)
}

fn tag_slug(tag: &str) -> &'static str {
    let t = tag.to_lowercase();
    match t.as_str() {
        "critical" => "critical",
        "buggy" => "buggy",
        "refactor" => "refactor",
        "todo" => "todo",
        _ => "todo",
    }
}

fn compute_stats(reports: &Reports, today: &str) -> DashboardStats {
    let mut open = 0usize;
    let mut resolved = 0usize;
    let mut critical = 0usize;
    let mut expired = 0usize;
    let mut expiring_soon = 0usize;

    for e in &reports.entries {
        if e.status.eq_ignore_ascii_case("open") {
            open += 1;
        } else {
            resolved += 1;
        }
        if e.tag.eq_ignore_ascii_case("critical") {
            critical += 1;
        }
        if let Some(ref exp) = e.expires_at {
            let exp_str = exp.trim();
            if !exp_str.is_empty() {
                if exp_str < today {
                    expired += 1;
                } else if days_between(today, exp_str) <= 7 {
                    expiring_soon += 1;
                }
            }
        }
    }

    DashboardStats {
        total: reports.entries.len(),
        open,
        resolved,
        critical,
        expired,
        expiring_soon,
    }
}

/// Days between two YYYY-MM-DD strings (order-insensitive absolute difference).
fn days_between(a: &str, b: &str) -> i64 {
    let parse = |s: &str| {
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() != 3 {
            return None;
        }
        let y: i32 = parts[0].parse().ok()?;
        let m: u32 = parts[1].parse().ok()?;
        let d: u32 = parts[2].parse().ok()?;
        chrono::NaiveDate::from_ymd_opt(y, m, d)
    };
    match (parse(a), parse(b)) {
        (Some(d1), Some(d2)) => (d2 - d1).num_days().abs(),
        _ => 999,
    }
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn compute_chart_data(
    reports: &Reports,
) -> (
    Vec<(String, u32)>,
    Vec<(String, u32)>,
    HashMap<String, HashMap<String, u32>>,
) {
    let mut tag_counts: HashMap<String, u32> = HashMap::new();
    let mut file_counts: HashMap<String, u32> = HashMap::new();
    let mut heatmap: HashMap<String, HashMap<String, u32>> = HashMap::new();

    for e in &reports.entries {
        *tag_counts.entry(e.tag.clone()).or_insert(0) += 1;
        *file_counts.entry(e.path.clone()).or_insert(0) += 1;
        heatmap
            .entry(e.path.clone())
            .or_default()
            .entry(e.tag.clone())
            .and_modify(|n| *n += 1)
            .or_insert(1);
    }

    let mut tag_vec: Vec<(String, u32)> = tag_counts.into_iter().collect();
    tag_vec.sort_by(|a, b| b.1.cmp(&a.1));

    let mut file_vec: Vec<(String, u32)> = file_counts.into_iter().collect();
    file_vec.sort_by(|a, b| b.1.cmp(&a.1));

    (tag_vec, file_vec, heatmap)
}
