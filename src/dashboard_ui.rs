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
use std::time::{Duration, Instant};

const FALLBACK_DASHBOARD_TITLE_PREFIX: &str = "windows-mtr fallback dashboard";

const EMBEDDED_TRIPPY_ENV: &str = "WINDOWS_MTR_EMBEDDED_TRIPPY";

#[derive(Clone)]
struct HopStat {
    hop: usize,
    host: String,
    loss_pct: Option<f64>,
    best_ms: Option<f64>,
    avg_ms: Option<f64>,
    worst_ms: Option<f64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DashboardAction {
    Quit,
    NextTab,
    PreviousTab,
    ToggleHelp,
}

pub struct DashboardApp {
    target: String,
    tab_index: usize,
    hops: Vec<HopStat>,
    latency_history: Vec<(f64, f64)>,
    loss_history: Vec<(f64, f64)>,
    started_at: Instant,
    last_error: Option<String>,
    consecutive_poll_failures: u32,
    show_help: bool,
}

impl DashboardApp {
    fn new(target: &str) -> Self {
        Self {
            target: target.to_string(),
            tab_index: 0,
            hops: Vec::new(),
            latency_history: Vec::new(),
            loss_history: Vec::new(),
            started_at: Instant::now(),
            last_error: None,
            consecutive_poll_failures: 0,
            show_help: false,
        }
    }

    fn ingest_snapshot(&mut self, hops: Vec<HopStat>) {
        if hops.is_empty() {
            self.last_error = Some("No hop data returned by trippy JSON report".to_string());
            self.consecutive_poll_failures = self.consecutive_poll_failures.saturating_add(1);
            return;
        }

        self.hops = hops;
        self.last_error = None;
        self.consecutive_poll_failures = 0;

        let x = self.latency_history.len().max(self.loss_history.len()) as f64;
        if let Some(latency) = self.hops.last().and_then(|hop| hop.avg_ms) {
            self.latency_history.push((x, latency));
        }
        if let Some(loss) = self.hops.last().and_then(|hop| hop.loss_pct) {
            self.loss_history.push((x, loss));
        }

        if self.latency_history.len() > 120 {
            self.latency_history.remove(0);
        }
        if self.loss_history.len() > 120 {
            self.loss_history.remove(0);
        }
    }

    fn ingest_error(&mut self, err: anyhow::Error) {
        self.last_error = Some(err.to_string());
        self.consecutive_poll_failures = self.consecutive_poll_failures.saturating_add(1);
    }

    fn next_tab(&mut self) {
        self.tab_index = (self.tab_index + 1) % 3;
    }

    fn prev_tab(&mut self) {
        self.tab_index = (self.tab_index + 2) % 3;
    }

    fn apply_action(&mut self, action: DashboardAction) -> bool {
        match action {
            DashboardAction::Quit => true,
            DashboardAction::NextTab => {
                self.next_tab();
                false
            }
            DashboardAction::PreviousTab => {
                self.prev_tab();
                false
            }
            DashboardAction::ToggleHelp => {
                self.show_help = !self.show_help;
                false
            }
        }
    }
}

fn dashboard_action(key: KeyCode) -> Option<DashboardAction> {
    match key {
        KeyCode::Char('q') => Some(DashboardAction::Quit),
        KeyCode::Right | KeyCode::Tab => Some(DashboardAction::NextTab),
        KeyCode::Left | KeyCode::BackTab => Some(DashboardAction::PreviousTab),
        KeyCode::Char('?') | KeyCode::Char('h') => Some(DashboardAction::ToggleHelp),
        _ => None,
    }
}

pub fn run_dashboard_ui(target: &str, snapshot_args: &[String]) -> anyhow::Result<i32> {
    enable_raw_mode().context("failed to enable raw mode for dashboard UI")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).context("failed to enter alternate screen")?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("failed to initialize terminal backend")?;

    let result = run_ui_loop(&mut terminal, target, snapshot_args);

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
    snapshot_args: &[String],
) -> anyhow::Result<i32> {
    let mut app = DashboardApp::new(target);
    let tick_rate = Duration::from_millis(250);
    let poll_rate = Duration::from_millis(900);
    let (snapshot_tx, snapshot_rx) = mpsc::channel::<anyhow::Result<Vec<HopStat>>>();
    let poll_args = snapshot_args.to_vec();
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
                Err(err) => app.ingest_error(err),
            }
        }

        terminal.draw(|f| draw_ui(f, &app))?;

        if event::poll(tick_rate).context("failed to poll terminal events")?
            && let Event::Key(key) = event::read().context("failed to read terminal event")?
        {
            if let Some(action) = dashboard_action(key.code)
                && app.apply_action(action)
            {
                return Ok(0);
            }
        }
    }
}

fn fetch_hops_snapshot(snapshot_args: &[String], target: &str) -> anyhow::Result<Vec<HopStat>> {
    // SAFETY: `current_exe` is only used to re-exec this process for local JSON polling,
    // not for any trust or authorization decision.
    let current_exe =
        // nosemgrep: rust.lang.security.current-exe.current-exe
        env::current_exe().context("failed to locate current executable for dashboard polling")?;

    let output = Command::new(&current_exe)
        .env(EMBEDDED_TRIPPY_ENV, "1")
        .args(snapshot_args.iter().skip(1))
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
        let loss_pct = read_loss_percent(item).map(|value| value.clamp(0.0, 100.0));
        let avg_ms = read_f64(item, &["avg_ms", "avg", "average_ms", "last_ms", "last"]);
        let best_ms = read_f64(item, &["best_ms", "best", "min_ms", "min"]).or(avg_ms);
        let worst_ms = read_f64(item, &["worst_ms", "worst", "max_ms", "max"]).or(avg_ms);

        hops.push(HopStat {
            hop,
            host,
            loss_pct,
            best_ms,
            avg_ms,
            worst_ms,
        });
    }

    hops
}

fn read_loss_percent(item: &Value) -> Option<f64> {
    if let Some(value) = read_f64(item, &["loss_pct", "loss_percentage"]) {
        return Some(value);
    }
    if let Some(value) = read_f64(item, &["loss_ratio"]) {
        return Some(value * 100.0);
    }
    read_f64(item, &["loss"])
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
    read_f64(item, keys).and_then(|value| {
        if value >= 0.0 && value <= usize::MAX as f64 {
            Some(value as usize)
        } else {
            None
        }
    })
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
                if let Some(n) = number.as_f64().filter(|n| n.is_finite()) {
                    return Some(n);
                }
            }
            Value::String(raw) => {
                let trimmed = raw
                    .trim()
                    .trim_end_matches("ms")
                    .trim_end_matches('%')
                    .trim();
                if let Ok(n) = trimmed.parse::<f64>()
                    && n.is_finite()
                {
                    return Some(n);
                }
            }
            _ => {}
        }
    }
    None
}

fn draw_ui(frame: &mut ratatui::Frame<'_>, app: &DashboardApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(12),
            Constraint::Length(2),
        ])
        .split(frame.area());

    let titles = ["Overview", "Hops", "Charts"]
        .iter()
        .map(|t| Line::from(*t))
        .collect::<Vec<_>>();

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .title(format!(
                    "{FALLBACK_DASHBOARD_TITLE_PREFIX} ({})",
                    app.target
                ))
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
        0 => render_overview(frame, app, chunks[1]),
        1 => render_hop_table(frame, app, chunks[1]),
        _ => render_charts(frame, app, chunks[1]),
    }

    let help_text = build_help_text(app);
    let help =
        Paragraph::new(help_text).block(Block::default().borders(Borders::ALL).title("Help"));
    frame.render_widget(help, chunks[2]);
}

fn render_overview(
    frame: &mut ratatui::Frame<'_>,
    app: &DashboardApp,
    area: ratatui::layout::Rect,
) {
    let message = if let Some(error) = &app.last_error {
        format!("Polling error: {error}\nThe dashboard will keep showing the last valid hop data.")
    } else if app.hops.is_empty() {
        format!("Loading probe snapshots for {}...", app.target)
    } else {
        let destination = app.hops.last().expect("non-empty hops checked above");
        format!(
            "{} hops received. Destination: {}\nLatest avg: {} ms   Latest loss: {}%",
            app.hops.len(),
            destination.host,
            format_metric(destination.avg_ms),
            format_metric(destination.loss_pct)
        )
    };
    let overview =
        Paragraph::new(message).block(Block::default().borders(Borders::ALL).title("Overview"));
    frame.render_widget(overview, area);
}

fn build_help_text(app: &DashboardApp) -> String {
    let base = if app.show_help {
        "Help: Tab/Right next tab, Shift+Tab/Left previous tab, h/? toggle this help, q quit. JSON polling has no hidden retries."
    } else {
        "Fallback dashboard: JSON snapshot polling, limited fields. Tab/Right navigate; h/? help; q quit."
    };
    let mut notes = Vec::new();

    if app.hops.is_empty() {
        notes.push(format!(
            "Awaiting hop data for {}s",
            app.started_at.elapsed().as_secs()
        ));
    }

    if let Some(err) = &app.last_error {
        notes.push(format!("Last poll error: {err}"));
    }

    if app.hops.is_empty() && app.consecutive_poll_failures >= 3 {
        notes.push(
            "No hops yet. Try running as Administrator, checking firewall policy, or using report mode (-r)."
                .to_string(),
        );
    }

    if app.hops.is_empty() && app.started_at.elapsed().as_secs() >= 15 {
        notes.push(
            "Still no data after 15s. Press q to quit and retry with report mode (-r) for immediate diagnostics."
                .to_string(),
        );
    }

    if notes.is_empty() {
        base.to_string()
    } else {
        format!("{base} | {}", notes.join(" | "))
    }
}

fn render_hop_table(
    frame: &mut ratatui::Frame<'_>,
    app: &DashboardApp,
    area: ratatui::layout::Rect,
) {
    if app.hops.is_empty() {
        let state = app
            .last_error
            .as_deref()
            .map(|error| format!("Unable to load hop data: {error}"))
            .unwrap_or_else(|| format!("Loading hop data for {}...", app.target));
        let placeholder =
            Paragraph::new(state).block(Block::default().borders(Borders::ALL).title("Hop table"));
        frame.render_widget(placeholder, area);
        return;
    }

    let rows = app.hops.iter().map(|hop| {
        Row::new(vec![
            hop.hop.to_string(),
            hop.host.clone(),
            format_metric(hop.loss_pct),
            format_metric(hop.best_ms),
            format_metric(hop.avg_ms),
            format_metric(hop.worst_ms),
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

fn format_metric(value: Option<f64>) -> String {
    value
        .map(|metric| format!("{metric:.1}"))
        .unwrap_or_else(|| "N/A".to_string())
}

fn render_charts(frame: &mut ratatui::Frame<'_>, app: &DashboardApp, area: ratatui::layout::Rect) {
    let charts = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);
    render_latency_chart(frame, app, charts[0]);
    render_loss_chart(frame, app, charts[1]);
}

fn render_latency_chart(
    frame: &mut ratatui::Frame<'_>,
    app: &DashboardApp,
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
    app: &DashboardApp,
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
        assert_eq!(hops[0].loss_pct, Some(1.0));

        assert_eq!(hops[1].hop, 2);
        assert_eq!(hops[1].host, "target.example");
        assert_eq!(hops[1].loss_pct, Some(0.5));
        assert_eq!(hops[1].best_ms, Some(15.0));
        assert_eq!(hops[1].avg_ms, Some(20.0));
        assert_eq!(hops[1].worst_ms, Some(25.0));
    }

    #[test]
    fn read_loss_percent_handles_percent_and_ratio_fields() {
        let percent = json!({"loss_pct": 0.5});
        let percent_large = json!({"loss_percentage": 5.0});
        let ratio = json!({"loss_ratio": 0.05});

        assert_eq!(read_loss_percent(&percent), Some(0.5));
        assert_eq!(read_loss_percent(&percent_large), Some(5.0));
        assert_eq!(read_loss_percent(&ratio), Some(5.0));
    }

    #[test]
    fn extract_hops_parses_trippy_013_fixture() {
        let fixture = include_str!("../tests/fixtures/trippy_0_13_report.json");
        let payload: Value = serde_json::from_str(fixture).expect("fixture must parse");

        let hops = extract_hops(&payload, "8.8.8.8");
        assert!(!hops.is_empty());
        assert_eq!(hops[0].hop, 1);
        assert!(
            hops.iter()
                .any(|h| h.host.contains("8.8.8.8") || h.host.contains("dns.google"))
        );
    }

    #[test]
    fn build_help_text_includes_live_troubleshooting_when_ui_has_no_data() {
        let mut app = DashboardApp::new("example.com");
        app.started_at = Instant::now() - Duration::from_secs(16);
        app.ingest_error(anyhow!("poll failed"));
        app.ingest_error(anyhow!("poll failed"));
        app.ingest_error(anyhow!("poll failed"));

        let help = build_help_text(&app);
        assert!(help.contains("Awaiting hop data for"));
        assert!(help.contains("Last poll error: poll failed"));
        assert!(help.contains("No hops yet."));
        assert!(help.contains("report mode (-r)"));
        assert!(help.contains("Still no data after 15s."));
    }

    #[test]
    fn build_help_text_is_compact_when_data_stream_is_healthy() {
        let mut app = DashboardApp::new("example.com");
        app.ingest_snapshot(vec![HopStat {
            hop: 1,
            host: "1.1.1.1".to_string(),
            loss_pct: Some(0.0),
            best_ms: Some(1.0),
            avg_ms: Some(2.0),
            worst_ms: Some(3.0),
        }]);

        assert_eq!(
            build_help_text(&app),
            "Fallback dashboard: JSON snapshot polling, limited fields. Tab/Right navigate; h/? help; q quit."
        );
    }

    #[test]
    fn dashboard_keyboard_actions_are_discoverable_and_apply_without_loop_io() {
        assert_eq!(
            dashboard_action(KeyCode::Tab),
            Some(DashboardAction::NextTab)
        );
        assert_eq!(
            dashboard_action(KeyCode::BackTab),
            Some(DashboardAction::PreviousTab)
        );
        assert_eq!(
            dashboard_action(KeyCode::Char('?')),
            Some(DashboardAction::ToggleHelp)
        );
        assert_eq!(
            dashboard_action(KeyCode::Char('q')),
            Some(DashboardAction::Quit)
        );

        let mut app = DashboardApp::new("example.com");
        assert!(!app.apply_action(DashboardAction::NextTab));
        assert_eq!(app.tab_index, 1);
        assert!(!app.apply_action(DashboardAction::ToggleHelp));
        assert!(app.show_help);
        assert!(app.apply_action(DashboardAction::Quit));
    }

    #[test]
    fn partial_or_malformed_metrics_remain_missing_and_do_not_enter_charts() {
        let payload =
            json!({"hops": [{"ttl": 1, "host": "router", "avg": "NaN", "loss": "not-a-number"}]});
        let hops = extract_hops(&payload, "target.example");
        assert_eq!(hops.len(), 1);
        assert_eq!(hops[0].avg_ms, None);
        assert_eq!(hops[0].loss_pct, None);
        assert_eq!(format_metric(hops[0].avg_ms), "N/A");

        let mut app = DashboardApp::new("target.example");
        app.ingest_snapshot(hops);
        assert!(app.latency_history.is_empty());
        assert!(app.loss_history.is_empty());
    }
}
