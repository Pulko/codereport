use crate::reports::Reports;
use std::collections::HashMap;
use std::path::Path;

pub fn generate_html(repo_root: &Path, reports: &Reports) -> Result<std::path::PathBuf, String> {
    let json = serde_json::to_string(&reports.entries).map_err(|e| e.to_string())?;
    let json_escaped = escape_json_for_script(&json);

    let (tag_counts, file_counts, heatmap) = compute_chart_data(reports);

    let max_tag_count = tag_counts.iter().map(|(_, c)| *c).max().unwrap_or(1) as f64;
    let tag_bars: String = tag_counts
        .iter()
        .map(|(tag, count)| {
            let pct = (*count as f64 / max_tag_count * 100.0).min(100.0);
            format!(
                r#"<div class="bar-row"><span class="bar-label">{}</span><div class="bar-wrap"><div class="bar" style="width:{}%"></div><span class="bar-value">{}</span></div></div>"#,
                escape_html(tag), pct, count
            )
        })
        .collect();

    let file_rows: String = file_counts
        .iter()
        .take(15)
        .map(|(path, count)| format!("<tr><td>{}</td><td>{}</td></tr>", escape_html(path), count))
        .collect();

    let tags: Vec<&String> = tag_counts.iter().map(|(t, _)| t).collect();
    let files: Vec<&String> = file_counts.iter().take(20).map(|(p, _)| p).collect();
    let heatmap_rows: String = {
        let mut rows = String::new();
        for path in &files {
            rows.push_str("<tr>");
            rows.push_str(&format!(
                "<td class=\"path-cell\">{}</td>",
                escape_html(path)
            ));
            for tag in &tags {
                let count = heatmap
                    .get(*path)
                    .and_then(|m| m.get(*tag))
                    .copied()
                    .unwrap_or(0);
                let class = if count > 0 {
                    if count >= 3 {
                        "heat hi"
                    } else if count >= 2 {
                        "heat mid"
                    } else {
                        "heat lo"
                    }
                } else {
                    ""
                };
                rows.push_str(&format!("<td class=\"{}\">{}</td>", class, count));
            }
            rows.push_str("</tr>");
        }
        rows
    };

    let tag_headers: String = tags
        .iter()
        .map(|t| format!("<th>{}</th>", escape_html(t)))
        .collect();

    let html = format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>Code Reports</title>
<style>
* {{ box-sizing: border-box; }}
body {{ font-family: system-ui, sans-serif; margin: 1rem 2rem; background: #1a1a1a; color: #e0e0e0; }}
h1 {{ margin-bottom: 0.5rem; }}
h2 {{ margin-top: 1.5rem; color: #b0b0b0; font-size: 1rem; }}
.section {{ margin-bottom: 2rem; }}
.bar-row {{ display: flex; align-items: center; margin-bottom: 0.4rem; }}
.bar-label {{ width: 100px; flex-shrink: 0; }}
.bar-wrap {{ flex: 1; display: flex; align-items: center; background: #2a2a2a; border-radius: 4px; overflow: hidden; }}
.bar {{ height: 20px; background: #4a9; min-width: 2px; }}
.bar-value {{ margin-left: 8px; font-variant-numeric: tabular-nums; }}
table {{ border-collapse: collapse; font-size: 0.9rem; }}
th, td {{ padding: 0.35rem 0.6rem; text-align: left; border: 1px solid #333; }}
th {{ background: #2a2a2a; }}
.path-cell {{ max-width: 280px; overflow: hidden; text-overflow: ellipsis; }}
.heat {{ text-align: center; }}
.heat.lo {{ background: #2d4a2d; }}
.heat.mid {{ background: #3d6a3d; }}
.heat.hi {{ background: #4a904a; }}
#report-table {{ width: 100%; }}
#report-table th {{ cursor: pointer; }}
</style>
</head>
<body>
<h1>Code Reports</h1>
<p>Generated from .codereports/reports.yaml</p>

<div class="section">
<h2>By tag</h2>
<div class="bar-rows">
{}
</div>
</div>

<div class="section">
<h2>Files with most reports (top 15)</h2>
<table>
<thead><tr><th>Path</th><th>Count</th></tr></thead>
<tbody>
{}
</tbody>
</table>
</div>

<div class="section">
<h2>Tag × file heatmap (top 20 files)</h2>
<table class="heatmap">
<thead><tr><th>Path</th>{}</tr></thead>
<tbody>
{}
</tbody>
</table>
</div>

<div class="section">
<h2>All reports</h2>
<table id="report-table">
<thead><tr><th>ID</th><th>Path</th><th>Range</th><th>Tag</th><th>Status</th><th>Message</th><th>Created</th><th>Expires</th></tr></thead>
<tbody id="report-tbody"></tbody>
</table>
</div>

<script type="application/json" id="report-data">{}</script>
<script>
(function() {{
  var data = JSON.parse(document.getElementById('report-data').textContent);
  var tbody = document.getElementById('report-tbody');
  data.forEach(function(e) {{
    var tr = document.createElement('tr');
    tr.innerHTML = '<td>' + e.id + '</td><td>' + e.path + '</td><td>' + e.range.start + '-' + e.range.end + '</td><td>' + e.tag + '</td><td>' + e.status + '</td><td>' + e.message + '</td><td>' + (e.created_at || '') + '</td><td>' + (e.expires_at || '') + '</td>';
    tbody.appendChild(tr);
  }});
}})();
</script>
</body>
</html>
"##,
        tag_bars, file_rows, tag_headers, heatmap_rows, json_escaped
    );

    let out_dir = repo_root.join(".codereports").join("html");
    std::fs::create_dir_all(&out_dir).map_err(|e| format!("create html dir: {}", e))?;
    let index_path = out_dir.join("index.html");
    std::fs::write(&index_path, html).map_err(|e| format!("write index.html: {}", e))?;
    Ok(index_path)
}

fn escape_json_for_script(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026")
}

fn escape_html(s: &str) -> String {
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
