use crate::output::json::{JsonOutput, Latency, Summary};
use crate::types::{LoadConfig, StatsSnapshot};
use std::fs::File;
use std::io::{self, BufWriter, Write};

pub fn write_html(snapshot: &StatsSnapshot, config: &LoadConfig, path: &str) -> io::Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    render_html(&mut writer, snapshot, config)
}

pub fn print_html(snapshot: &StatsSnapshot, config: &LoadConfig) -> io::Result<()> {
    let stdout = io::stdout();
    let mut writer = stdout.lock();
    render_html(&mut writer, snapshot, config)
}

fn render_html<W: Write>(w: &mut W, snapshot: &StatsSnapshot, config: &LoadConfig) -> io::Result<()> {
    let summary = Summary {
        total_requests: snapshot.total_requests,
        successful: snapshot.successful,
        failed: snapshot.failed,
        error_rate: snapshot.error_rate,
        requests_per_sec: snapshot.requests_per_sec,
        bytes_received: snapshot.bytes_received,
    };

    let latency = Latency {
        min: snapshot.latency_min_us,
        max: snapshot.latency_max_us,
        mean: snapshot.latency_mean_us,
        stddev: snapshot.latency_stddev_us,
        p50: snapshot.latency_p50_us,
        p75: snapshot.latency_p75_us,
        p90: snapshot.latency_p90_us,
        p95: snapshot.latency_p95_us,
        p99: snapshot.latency_p99_us,
        p999: snapshot.latency_p999_us,
    };

    let status_codes_html = snapshot
        .status_codes
        .iter()
        .map(|(code, count)| {
            let color = if *code < 300 {
                "#22c55e"
            } else if *code < 400 {
                "#eab308"
            } else {
                "#ef4444"
            };
            format!(
                r#"<div class="stat-item"><span class="stat-label" style="color: {}">{}</span><span class="stat-value">{}</span></div>"#,
                color, code, count
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let errors_html = snapshot
        .errors
        .iter()
        .map(|(kind, count)| {
            format!(
                r#"<div class="stat-item"><span class="stat-label">{}</span><span class="stat-value">{}</span></div>"#,
                kind.as_str(), count
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let timeline_data: Vec<String> = snapshot
        .timeline
        .iter()
        .map(|t| format!("{}", t.requests))
        .collect();
    let timeline_json = format!("[{}]", timeline_data.join(","));

    write!(w, r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Kaioken Load Test Report</title>
    <style>
        :root {{
            --bg-primary: #0f172a;
            --bg-secondary: #1e293b;
            --bg-tertiary: #334155;
            --text-primary: #f8fafc;
            --text-secondary: #94a3b8;
            --accent-cyan: #22d3ee;
            --accent-yellow: #facc15;
            --accent-green: #22c55e;
            --accent-red: #ef4444;
            --accent-orange: #f97316;
        }}
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, monospace;
            background: var(--bg-primary);
            color: var(--text-primary);
            line-height: 1.6;
            padding: 2rem;
        }}
        .container {{ max-width: 1200px; margin: 0 auto; }}
        .header {{
            text-align: center;
            margin-bottom: 2rem;
            padding: 2rem;
            background: linear-gradient(135deg, var(--bg-secondary), var(--bg-tertiary));
            border-radius: 12px;
            border: 1px solid var(--bg-tertiary);
        }}
        .header h1 {{
            font-size: 2.5rem;
            background: linear-gradient(90deg, var(--accent-cyan), var(--accent-yellow));
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
            background-clip: text;
            margin-bottom: 0.5rem;
        }}
        .header .subtitle {{ color: var(--text-secondary); font-size: 1.1rem; }}
        .header .url {{ color: var(--accent-cyan); font-family: monospace; margin-top: 1rem; }}
        .grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 1.5rem; margin-bottom: 1.5rem; }}
        .card {{
            background: var(--bg-secondary);
            border-radius: 12px;
            padding: 1.5rem;
            border: 1px solid var(--bg-tertiary);
        }}
        .card h2 {{
            font-size: 1rem;
            color: var(--text-secondary);
            text-transform: uppercase;
            letter-spacing: 0.1em;
            margin-bottom: 1rem;
            padding-bottom: 0.5rem;
            border-bottom: 1px solid var(--bg-tertiary);
        }}
        .big-stat {{
            font-size: 3rem;
            font-weight: bold;
            color: var(--accent-cyan);
            line-height: 1;
        }}
        .big-stat.success {{ color: var(--accent-green); }}
        .big-stat.error {{ color: var(--accent-red); }}
        .big-stat-label {{ color: var(--text-secondary); font-size: 0.9rem; margin-top: 0.5rem; }}
        .stat-item {{
            display: flex;
            justify-content: space-between;
            padding: 0.5rem 0;
            border-bottom: 1px solid var(--bg-tertiary);
        }}
        .stat-item:last-child {{ border-bottom: none; }}
        .stat-label {{ color: var(--text-secondary); }}
        .stat-value {{ font-weight: 600; font-family: monospace; }}
        .latency-bar {{
            display: flex;
            align-items: center;
            margin: 0.5rem 0;
        }}
        .latency-label {{ width: 60px; color: var(--text-secondary); font-size: 0.9rem; }}
        .latency-track {{
            flex: 1;
            height: 24px;
            background: var(--bg-tertiary);
            border-radius: 4px;
            overflow: hidden;
            margin: 0 1rem;
        }}
        .latency-fill {{
            height: 100%;
            background: linear-gradient(90deg, var(--accent-cyan), var(--accent-yellow));
            border-radius: 4px;
        }}
        .latency-value {{ width: 80px; text-align: right; font-family: monospace; }}
        .timeline {{
            height: 100px;
            display: flex;
            align-items: flex-end;
            gap: 2px;
            padding: 1rem 0;
        }}
        .timeline-bar {{
            flex: 1;
            background: var(--accent-cyan);
            border-radius: 2px 2px 0 0;
            min-height: 2px;
        }}
        .footer {{
            text-align: center;
            margin-top: 2rem;
            padding: 1rem;
            color: var(--text-secondary);
            font-size: 0.9rem;
        }}
        .footer a {{ color: var(--accent-cyan); text-decoration: none; }}
        @media (max-width: 768px) {{
            body {{ padding: 1rem; }}
            .header h1 {{ font-size: 1.8rem; }}
            .big-stat {{ font-size: 2rem; }}
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>KAIOKEN</h1>
            <p class="subtitle">Load Test Report</p>
            <p class="url">{method} {url}</p>
        </div>

        <div class="grid">
            <div class="card">
                <h2>Throughput</h2>
                <div class="big-stat">{rps:.0}</div>
                <div class="big-stat-label">requests/sec</div>
                <div style="margin-top: 1rem;">
                    <div class="stat-item">
                        <span class="stat-label">Total Requests</span>
                        <span class="stat-value">{total}</span>
                    </div>
                    <div class="stat-item">
                        <span class="stat-label">Successful</span>
                        <span class="stat-value" style="color: var(--accent-green)">{successful}</span>
                    </div>
                    <div class="stat-item">
                        <span class="stat-label">Failed</span>
                        <span class="stat-value" style="color: var(--accent-red)">{failed}</span>
                    </div>
                    <div class="stat-item">
                        <span class="stat-label">Error Rate</span>
                        <span class="stat-value">{error_rate:.2}%</span>
                    </div>
                </div>
            </div>

            <div class="card">
                <h2>Latency</h2>
                {latency_bars}
            </div>
        </div>

        <div class="grid">
            <div class="card">
                <h2>Status Codes</h2>
                {status_codes}
            </div>

            <div class="card">
                <h2>Errors</h2>
                {errors}
            </div>
        </div>

        <div class="card">
            <h2>Timeline (requests/sec)</h2>
            <div class="timeline" id="timeline"></div>
        </div>

        <div class="card">
            <h2>Configuration</h2>
            <div class="stat-item">
                <span class="stat-label">Concurrency</span>
                <span class="stat-value">{concurrency}</span>
            </div>
            <div class="stat-item">
                <span class="stat-label">Duration</span>
                <span class="stat-value">{duration}s</span>
            </div>
            <div class="stat-item">
                <span class="stat-label">Timeout</span>
                <span class="stat-value">{timeout}ms</span>
            </div>
        </div>

        <div class="footer">
            Generated by <a href="https://github.com/lance0/kaioken">kaioken</a> v{version}
        </div>
    </div>

    <script>
        const data = {timeline_data};
        const max = Math.max(...data, 1);
        const timeline = document.getElementById('timeline');
        data.forEach(val => {{
            const bar = document.createElement('div');
            bar.className = 'timeline-bar';
            bar.style.height = (val / max * 100) + '%';
            timeline.appendChild(bar);
        }});
    </script>
</body>
</html>
"##,
        method = config.method,
        url = config.url,
        rps = summary.requests_per_sec,
        total = summary.total_requests,
        successful = summary.successful,
        failed = summary.failed,
        error_rate = summary.error_rate * 100.0,
        latency_bars = render_latency_bars(&latency),
        status_codes = if status_codes_html.is_empty() { "<p style=\"color: var(--text-secondary)\">No data</p>".to_string() } else { status_codes_html },
        errors = if errors_html.is_empty() { "<p style=\"color: var(--text-secondary)\">None</p>".to_string() } else { errors_html },
        concurrency = config.concurrency,
        duration = config.duration.as_secs(),
        timeout = config.timeout.as_millis(),
        version = env!("CARGO_PKG_VERSION"),
        timeline_data = timeline_json,
    )
}

fn render_latency_bars(latency: &Latency) -> String {
    let max_latency = latency.p999 as f64;
    let percentiles = [
        ("p50", latency.p50),
        ("p90", latency.p90),
        ("p95", latency.p95),
        ("p99", latency.p99),
        ("p999", latency.p999),
    ];

    percentiles
        .iter()
        .map(|(label, value)| {
            let pct = if max_latency > 0.0 {
                (*value as f64 / max_latency * 100.0).min(100.0)
            } else {
                0.0
            };
            let ms = *value as f64 / 1000.0;
            format!(
                r#"<div class="latency-bar">
                    <span class="latency-label">{}</span>
                    <div class="latency-track"><div class="latency-fill" style="width: {}%"></div></div>
                    <span class="latency-value">{:.2}ms</span>
                </div>"#,
                label, pct, ms
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}
