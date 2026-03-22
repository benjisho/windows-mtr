use anyhow::{Context, anyhow};
use crossterm::event::{self, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols;
use ratatui::text::Line;
use ratatui::widgets::{
    Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph, Row, Sparkline, Table, Tabs,
};
use serde_json::Value;
use std::env;
use std::io::{self, Stdout};
use std::process::Command;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const EMBEDDED_TRIPPY_ENV: &str = "WINDOWS_MTR_EMBEDDED_TRIPPY";

#[derive(Clone)]
struct HopStat {
    hop: usize,
    host: String,
    loss_pct: f64,
    best_ms: f64,
    avg_ms: f64,
    worst_ms: f64,
}

pub struct NativeUiApp {
    target: String,
    tab_index: usize,
    hops: Vec<HopStat>,
    latency_history: Vec<(f64, f64)>,
    loss_history: Vec<(f64, f64)>,
    last_error: Option<String>,
}

impl NativeUiApp {
    fn new(target: &str) -> Self {
        Self {
            target: target.to_string(),
            tab_index: 0,
            hops: Vec::new(),
            latency_history: Vec::new(),
            loss_history: Vec::new(),
            last_error: None,
        }
    }

    fn ingest_snapshot(&mut self, hops: Vec<HopStat>) {
        if hops.is_empty() {
            self.last_error = Some("No hop data returned by trippy JSON report".to_string());
            return;
        }

        self.hops = hops;
        self.last_error = None;

        let latest_latency = self.hops.last().map(|h| h.avg_ms).unwrap_or_default();
        let latest_loss = self.hops.last().map(|h| h.loss_pct).unwrap_or_default();
        let x = self.latency_history.len() as f64;

        self.latency_history.push((x, latest_latency));
        self.loss_history.push((x, latest_loss));

        if self.latency_history.len() > 120 {
            self.latency_history.remove(0);
        }
        if self.loss_history.len() > 120 {
            self.loss_history.remove(0);
        }
    }

    fn next_tab(&mut self) {
        self.tab_index = (self.tab_index + 1) % 3;
    }

    fn prev_tab(&mut self) {
        self.tab_index = (self.tab_index + 2) % 3;
    }
}

pub fn run_native_ui(target: &str, trippy_args: &[String]) -> anyhow::Result<i32> {
    enable_raw_mode().context("failed to enable raw mode for native UI")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).context("failed to enter alternate screen")?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("failed to initialize terminal backend")?;

    let result = run_ui_loop(&mut terminal, target, trippy_args);

    let mut restore_error: Option<anyhow::Error> = None;

    if let Err(err) = disable_raw_mode() {
        restore_error = Some(anyhow::Error::new(err).context("failed to disable raw mode"));
    }

    if let Err(err) = execute!(terminal.backend_mut(), LeaveAlternateScreen) {
        let leave_err = anyhow::Error::new(err).context("failed to leave alternate screen");
        restore_error = Some(match restore_error {
            Some(existing) => existing.context(leave_err.to_string()),
            None => leave_err,
        });
    }

    terminal
        .show_cursor()
        .context("failed to restore terminal cursor")?;

    if let Some(err) = restore_error {
        return Err(err);
    }

    result
}

fn run_ui_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    target: &str,
    trippy_args: &[String],
) -> anyhow::Result<i32> {
    let mut app = NativeUiApp::new(target);
    let tick_rate = Duration::from_millis(250);
    let poll_rate = Duration::from_millis(900);
    let (snapshot_tx, snapshot_rx) = mpsc::channel::<anyhow::Result<Vec<HopStat>>>();
    let poll_args = trippy_args.to_vec();
    let poll_target = target.to_string();

    thread::spawn(move || {
        loop {
            let result = fetch_hops_snapshot(&poll_args, &poll_target);
            if snapshot_tx.send(result).is_err() {
                break;
            }
            thread::sleep(poll_rate);
        }
    });

    loop {
        while let Ok(snapshot) = snapshot_rx.try_recv() {
            match snapshot {
                Ok(hops) => app.ingest_snapshot(hops),
                Err(err) => app.last_error = Some(err.to_string()),
            }
        }

        terminal.draw(|f| draw_ui(f, &app))?;

        if event::poll(tick_rate).context("failed to poll terminal events")?
            && let Event::Key(key) = event::read().context("failed to read terminal event")?
        {
            match key.code {
                KeyCode::Char('q') => return Ok(0),
                KeyCode::Right | KeyCode::Tab => app.next_tab(),
                KeyCode::Left => app.prev_tab(),
                _ => {}
            }
        }
    }
}

fn fetch_hops_snapshot(base_args: &[String], target: &str) -> anyhow::Result<Vec<HopStat>> {
    // SAFETY: `current_exe` is only used to re-exec this process for local JSON polling,
    // not for any trust or authorization decision.
    let current_exe =
        // nosemgrep: rust.lang.security.current-exe.current-exe
        env::current_exe().context("failed to locate current executable for native UI polling")?;

    let mut args = sanitize_args_for_json_snapshot(base_args);
    args.extend([
        "--mode".to_string(),
        "json".to_string(),
        "--report-cycles".to_string(),
        "1".to_string(),
    ]);

    let output = Command::new(&current_exe)
        .env(EMBEDDED_TRIPPY_ENV, "1")
        .args(args.iter().skip(1))
        .output()
        .context("failed to run embedded trippy JSON poll")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("trippy poll failed: {}", stderr.trim()));
    }

    let value: Value = serde_json::from_slice(&output.stdout)
        .context("failed to parse trippy JSON poll output")?;

    Ok(extract_hops(&value, target))
}

fn sanitize_args_for_json_snapshot(base_args: &[String]) -> Vec<String> {
    let mut cleaned = Vec::with_capacity(base_args.len());
    let mut skip_next = false;
    let pair_flags = ["--mode", "--report-cycles"];

    for token in base_args {
        if skip_next {
            skip_next = false;
            continue;
        }

        if pair_flags.contains(&token.as_str()) {
            skip_next = true;
            continue;
        }

        cleaned.push(token.clone());
    }

    cleaned
}

fn extract_hops(value: &Value, target: &str) -> Vec<HopStat> {
    let Some(array) = find_hop_array(value) else {
        return Vec::new();
    };

    let mut hops = Vec::with_capacity(array.len());
    for (index, item) in array.iter().enumerate() {
        if !item.is_object() {
            continue;
        }

        let hop = read_usize(item, &["ttl", "hop", "hop_index"]).unwrap_or(index + 1);
        let host = read_string(item, &["host", "hostname", "ip", "addr"]).unwrap_or_else(|| {
            if index + 1 == array.len() {
                target.to_string()
            } else {
                format!("hop-{hop}")
            }
        });
        let loss_pct = read_f64(item, &["loss_pct", "loss", "loss_percentage"]).unwrap_or(0.0);
        let avg_ms =
            read_f64(item, &["avg_ms", "avg", "average_ms", "last_ms", "last"]).unwrap_or(0.0);
        let best_ms = read_f64(item, &["best_ms", "best", "min_ms", "min"]).unwrap_or(avg_ms);
        let worst_ms = read_f64(item, &["worst_ms", "worst", "max_ms", "max"]).unwrap_or(avg_ms);

        hops.push(HopStat {
            hop,
            host,
            loss_pct: normalize_percent(loss_pct),
            best_ms,
            avg_ms,
            worst_ms,
        });
    }

    hops
}

fn normalize_percent(value: f64) -> f64 {
    if value < 1.0 { value * 100.0 } else { value }
}

fn find_hop_array(value: &Value) -> Option<&Vec<Value>> {
    match value {
        Value::Array(array) => {
            if looks_like_hop_array(array) {
                return Some(array);
            }
            for item in array {
                if let Some(found) = find_hop_array(item) {
                    return Some(found);
                }
            }
            None
        }
        Value::Object(map) => {
            for candidate in ["hops", "hosts", "report", "result", "results", "data"] {
                if let Some(next) = map.get(candidate)
                    && let Some(found) = find_hop_array(next)
                {
                    return Some(found);
                }
            }
            for next in map.values() {
                if let Some(found) = find_hop_array(next) {
                    return Some(found);
                }
            }
            None
        }
        _ => None,
    }
}

fn looks_like_hop_array(array: &[Value]) -> bool {
    !array.is_empty()
        && array.iter().all(|item| {
            item.get("ttl").is_some()
                || item.get("hop").is_some()
                || item.get("avg_ms").is_some()
                || item.get("avg").is_some()
                || item.get("host").is_some()
        })
}

fn read_usize(item: &Value, keys: &[&str]) -> Option<usize> {
    read_f64(item, keys).map(|v| v.max(0.0) as usize)
}

fn read_string(item: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        let Some(value) = item.get(*key) else {
            continue;
        };
        if let Some(s) = value.as_str() {
            return Some(s.to_string());
        }
    }
    None
}

fn read_f64(item: &Value, keys: &[&str]) -> Option<f64> {
    for key in keys {
        let Some(value) = item.get(*key) else {
            continue;
        };
        match value {
            Value::Number(number) => {
                if let Some(n) = number.as_f64() {
                    return Some(n);
                }
            }
            Value::String(raw) => {
                let trimmed = raw
                    .trim()
                    .trim_end_matches("ms")
                    .trim_end_matches('%')
                    .trim();
                if let Ok(n) = trimmed.parse::<f64>() {
                    return Some(n);
                }
            }
            _ => {}
        }
    }
    None
}

fn draw_ui(frame: &mut ratatui::Frame<'_>, app: &NativeUiApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(12),
            Constraint::Length(2),
        ])
        .split(frame.area());

    let titles = ["Hops", "Latency", "Loss"]
        .iter()
        .map(|t| Line::from(*t))
        .collect::<Vec<_>>();

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .title(format!("windows-mtr native UI ({})", app.target))
                .borders(Borders::ALL),
        )
        .select(app.tab_index)
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_widget(tabs, chunks[0]);

    match app.tab_index {
        0 => render_hop_table(frame, app, chunks[1]),
        1 => render_latency_chart(frame, app, chunks[1]),
        _ => render_loss_chart(frame, app, chunks[1]),
    }

    let help_text = match &app.last_error {
        Some(err) => format!("Controls: ←/→ or Tab switch tabs • q quits • Last poll error: {err}"),
        None => "Controls: ←/→ or Tab switch tabs • q quits".to_string(),
    };
    let help =
        Paragraph::new(help_text).block(Block::default().borders(Borders::ALL).title("Help"));
    frame.render_widget(help, chunks[2]);
}

fn render_hop_table(
    frame: &mut ratatui::Frame<'_>,
    app: &NativeUiApp,
    area: ratatui::layout::Rect,
) {
    let rows = app.hops.iter().map(|hop| {
        Row::new(vec![
            hop.hop.to_string(),
            hop.host.clone(),
            format!("{:.1}", hop.loss_pct),
            format!("{:.1}", hop.best_ms),
            format!("{:.1}", hop.avg_ms),
            format!("{:.1}", hop.worst_ms),
        ])
    });

    let table = Table::new(
        rows,
        [
            Constraint::Length(5),
            Constraint::Percentage(35),
            Constraint::Length(9),
            Constraint::Length(9),
            Constraint::Length(9),
            Constraint::Length(9),
        ],
    )
    .header(
        Row::new(vec!["Hop", "Host", "Loss%", "Best", "Avg", "Worst"]).style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(Block::default().borders(Borders::ALL).title("Hop table"));

    frame.render_widget(table, area);
}

fn render_latency_chart(
    frame: &mut ratatui::Frame<'_>,
    app: &NativeUiApp,
    area: ratatui::layout::Rect,
) {
    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(8), Constraint::Length(6)])
        .split(area);

    let dataset = Dataset::default()
        .name("RTT avg (ms)")
        .graph_type(GraphType::Line)
        .marker(symbols::Marker::Braille)
        .style(Style::default().fg(Color::Green))
        .data(&app.latency_history);

    let x_max = app
        .latency_history
        .last()
        .map(|(x, _)| *x)
        .unwrap_or(20.0)
        .max(20.0);

    let y_max = app
        .latency_history
        .iter()
        .map(|(_, y)| *y)
        .reduce(f64::max)
        .unwrap_or(100.0)
        .max(50.0);

    let chart = Chart::new(vec![dataset])
        .block(
            Block::default()
                .title("Latency chart")
                .borders(Borders::ALL),
        )
        .x_axis(
            Axis::default()
                .title("Samples")
                .bounds([0.0, x_max])
                .labels(vec![Line::from("0"), Line::from(format!("{x_max:.0}"))]),
        )
        .y_axis(
            Axis::default()
                .title("ms")
                .bounds([0.0, y_max])
                .labels(vec![Line::from("0"), Line::from(format!("{y_max:.0}"))]),
        );

    frame.render_widget(chart, inner[0]);

    let spark_data = app
        .latency_history
        .iter()
        .map(|(_, y)| (*y as u64).min(300))
        .collect::<Vec<_>>();
    let spark = Sparkline::default()
        .block(
            Block::default()
                .title("Latency sparkline")
                .borders(Borders::ALL),
        )
        .data(&spark_data)
        .style(Style::default().fg(Color::LightGreen));
    frame.render_widget(spark, inner[1]);
}

fn render_loss_chart(
    frame: &mut ratatui::Frame<'_>,
    app: &NativeUiApp,
    area: ratatui::layout::Rect,
) {
    let dataset = Dataset::default()
        .name("Loss %")
        .graph_type(GraphType::Line)
        .marker(symbols::Marker::Dot)
        .style(Style::default().fg(Color::Red))
        .data(&app.loss_history);

    let x_max = app
        .loss_history
        .last()
        .map(|(x, _)| *x)
        .unwrap_or(20.0)
        .max(20.0);

    let chart = Chart::new(vec![dataset])
        .block(Block::default().title("Loss chart").borders(Borders::ALL))
        .x_axis(
            Axis::default()
                .title("Samples")
                .bounds([0.0, x_max])
                .labels(vec![Line::from("0"), Line::from(format!("{x_max:.0}"))]),
        )
        .y_axis(Axis::default().title("%").bounds([0.0, 100.0]).labels(vec![
            Line::from("0"),
            Line::from("50"),
            Line::from("100"),
        ]));

    frame.render_widget(chart, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sanitize_args_for_json_snapshot_strips_mode_and_cycles_pairs() {
        let args = vec![
            "mtr".to_string(),
            "--mode".to_string(),
            "tui".to_string(),
            "--report-cycles".to_string(),
            "10".to_string(),
            "--udp".to_string(),
            "example.com".to_string(),
        ];

        let cleaned = sanitize_args_for_json_snapshot(&args);
        assert_eq!(cleaned, vec!["mtr", "--udp", "example.com"]);
    }

    #[test]
    fn read_helpers_scan_all_candidate_keys() {
        let value = json!({
            "avg": "12.5ms",
            "hostname": "example.com"
        });

        assert_eq!(read_f64(&value, &["avg_ms", "avg"]), Some(12.5));
        assert_eq!(
            read_string(&value, &["host", "hostname"]),
            Some("example.com".to_string())
        );
    }

    #[test]
    fn extract_hops_supports_nested_report_shapes_and_defaults_last_hop_host() {
        let payload = json!({
            "result": {
                "hops": [
                    {"ttl": 1, "host": "10.0.0.1", "loss": "1%", "avg_ms": 10.0},
                    {"ttl": 2, "loss_pct": 0.5, "avg": "20ms", "min": 15.0, "max": 25.0}
                ]
            }
        });

        let hops = extract_hops(&payload, "target.example");
        assert_eq!(hops.len(), 2);

        assert_eq!(hops[0].hop, 1);
        assert_eq!(hops[0].host, "10.0.0.1");
        assert_eq!(hops[0].loss_pct, 1.0);

        assert_eq!(hops[1].hop, 2);
        assert_eq!(hops[1].host, "target.example");
        assert_eq!(hops[1].loss_pct, 50.0);
        assert_eq!(hops[1].best_ms, 15.0);
        assert_eq!(hops[1].avg_ms, 20.0);
        assert_eq!(hops[1].worst_ms, 25.0);
    }
}
