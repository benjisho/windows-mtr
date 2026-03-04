use anyhow::Context;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols;
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Axis, Block, Borders, Cell, Chart, Dataset, Gauge, Paragraph, Row, Sparkline, Table, Tabs, Wrap,
};
use std::f64::consts::PI;
use std::io;
use std::time::{Duration, Instant};

#[derive(Clone)]
struct HopStat {
    hop: u8,
    host: String,
    loss_pct: f64,
    last_ms: f64,
    avg_ms: f64,
    best_ms: f64,
    worst_ms: f64,
}

#[derive(Clone, Copy)]
enum Tab {
    Overview,
    Hops,
    Stats,
    Help,
}

impl Tab {
    fn next(self) -> Self {
        match self {
            Self::Overview => Self::Hops,
            Self::Hops => Self::Stats,
            Self::Stats => Self::Help,
            Self::Help => Self::Overview,
        }
    }

    fn previous(self) -> Self {
        match self {
            Self::Overview => Self::Help,
            Self::Hops => Self::Overview,
            Self::Stats => Self::Hops,
            Self::Help => Self::Stats,
        }
    }

    fn index(self) -> usize {
        match self {
            Self::Overview => 0,
            Self::Hops => 1,
            Self::Stats => 2,
            Self::Help => 3,
        }
    }
}

#[derive(Clone, Copy)]
enum SortMode {
    Hop,
    Loss,
    AvgLatency,
}

impl SortMode {
    fn next(self) -> Self {
        match self {
            Self::Hop => Self::Loss,
            Self::Loss => Self::AvgLatency,
            Self::AvgLatency => Self::Hop,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Hop => "hop",
            Self::Loss => "loss",
            Self::AvgLatency => "avg-latency",
        }
    }
}

struct NativeUiApp {
    target: String,
    active_tab: Tab,
    selected_hop: usize,
    cycles: u64,
    paused: bool,
    latency_history: Vec<(f64, f64)>,
    hops: Vec<HopStat>,
    started_at: Instant,
    sort_mode: SortMode,
}

impl NativeUiApp {
    fn new(target: &str) -> Self {
        let hops = vec![
            HopStat {
                hop: 1,
                host: "gateway.local".to_string(),
                loss_pct: 0.0,
                last_ms: 1.2,
                avg_ms: 1.1,
                best_ms: 0.9,
                worst_ms: 1.8,
            },
            HopStat {
                hop: 2,
                host: "metro-edge".to_string(),
                loss_pct: 0.2,
                last_ms: 5.8,
                avg_ms: 6.0,
                best_ms: 5.2,
                worst_ms: 7.3,
            },
            HopStat {
                hop: 3,
                host: "core-pop".to_string(),
                loss_pct: 0.3,
                last_ms: 9.5,
                avg_ms: 9.1,
                best_ms: 8.4,
                worst_ms: 12.1,
            },
            HopStat {
                hop: 4,
                host: target.to_string(),
                loss_pct: 0.7,
                last_ms: 18.6,
                avg_ms: 17.9,
                best_ms: 16.1,
                worst_ms: 23.0,
            },
        ];

        let mut app = Self {
            target: target.to_string(),
            active_tab: Tab::Overview,
            selected_hop: 0,
            cycles: 0,
            paused: false,
            latency_history: Vec::new(),
            hops,
            started_at: Instant::now(),
            sort_mode: SortMode::Hop,
        };
        app.seed_history();
        app
    }

    fn seed_history(&mut self) {
        for idx in 0..100 {
            let x = idx as f64;
            let y = 18.0 + (x / 8.0 * PI / 2.0).sin() * 4.0;
            self.latency_history.push((x, y));
        }
    }

    fn tick(&mut self) {
        if self.paused {
            return;
        }

        self.cycles += 1;
        let x = self.latency_history.last().map_or(0.0, |(v, _)| v + 1.0);
        let y = 17.5 + (x / 11.0).sin() * 4.5 + (x / 6.5).cos() * 0.8;
        self.latency_history.push((x, y));
        if self.latency_history.len() > 180 {
            self.latency_history.remove(0);
        }

        for (idx, hop) in self.hops.iter_mut().enumerate() {
            let phase = x / 16.0 + idx as f64;
            let baseline = (idx as f64 + 1.0) * 4.5;
            let updated = (baseline + phase.sin() * 1.4 + phase.cos() * 0.6).max(0.5);

            hop.last_ms = updated;
            hop.avg_ms = ((hop.avg_ms * 7.0) + updated) / 8.0;
            hop.best_ms = hop.best_ms.min(updated);
            hop.worst_ms = hop.worst_ms.max(updated);

            if self.cycles.is_multiple_of((9 + idx as u64).max(1)) {
                hop.loss_pct = (hop.loss_pct + 0.1).min(6.0);
            }
            if self.cycles.is_multiple_of((13 + idx as u64).max(1)) {
                hop.loss_pct = (hop.loss_pct - 0.1).max(0.0);
            }
        }

        self.apply_sort();
    }

    fn apply_sort(&mut self) {
        let selected_hop_number = self.selected_hop().map(|h| h.hop).unwrap_or(1);
        match self.sort_mode {
            SortMode::Hop => self.hops.sort_by_key(|h| h.hop),
            SortMode::Loss => self
                .hops
                .sort_by(|a, b| b.loss_pct.total_cmp(&a.loss_pct).then(a.hop.cmp(&b.hop))),
            SortMode::AvgLatency => self
                .hops
                .sort_by(|a, b| b.avg_ms.total_cmp(&a.avg_ms).then(a.hop.cmp(&b.hop))),
        }

        self.selected_hop = self
            .hops
            .iter()
            .position(|h| h.hop == selected_hop_number)
            .unwrap_or(0);
    }

    fn avg_latency(&self) -> f64 {
        if self.latency_history.is_empty() {
            return 0.0;
        }
        let total: f64 = self.latency_history.iter().map(|(_, y)| y).sum();
        total / self.latency_history.len() as f64
    }

    fn latency_jitter(&self) -> f64 {
        if self.latency_history.len() < 2 {
            return 0.0;
        }

        let mut delta_sum = 0.0;
        for window in self.latency_history.windows(2) {
            delta_sum += (window[1].1 - window[0].1).abs();
        }
        delta_sum / (self.latency_history.len() as f64 - 1.0)
    }

    fn global_loss_pct(&self) -> f64 {
        if self.hops.is_empty() {
            return 0.0;
        }
        self.hops.iter().map(|h| h.loss_pct).sum::<f64>() / self.hops.len() as f64
    }

    fn quality_score(&self) -> f64 {
        let loss_penalty = self.global_loss_pct() * 6.0;
        let latency_penalty = self.avg_latency() * 1.2;
        (100.0 - loss_penalty - latency_penalty).clamp(0.0, 100.0)
    }

    fn sparkline_data(&self) -> Vec<u64> {
        self.latency_history
            .iter()
            .map(|(_, y)| (y * 2.2).round().max(0.0) as u64)
            .collect()
    }

    fn status_badge(&self) -> (&'static str, Color) {
        let score = self.quality_score();
        if score >= 80.0 {
            ("Excellent", Color::Rgb(110, 255, 180))
        } else if score >= 60.0 {
            ("Good", Color::Rgb(255, 215, 100))
        } else {
            ("Degraded", Color::Rgb(255, 130, 130))
        }
    }

    fn active_alerts(&self) -> Vec<String> {
        let mut alerts = Vec::new();
        if self.global_loss_pct() > 2.0 {
            alerts.push(format!(
                "Packet loss elevated ({:.1}%)",
                self.global_loss_pct()
            ));
        }
        if self.latency_jitter() > 2.5 {
            alerts.push(format!(
                "Jitter is unstable ({:.2}ms)",
                self.latency_jitter()
            ));
        }
        if let Some(h) = self.selected_hop() {
            if h.avg_ms > 25.0 {
                alerts.push(format!(
                    "Hop #{} latency high ({:.1}ms avg)",
                    h.hop, h.avg_ms
                ));
            }
        }
        if alerts.is_empty() {
            alerts.push("No active alerts".to_string());
        }
        alerts
    }
    fn selected_hop(&self) -> Option<&HopStat> {
        self.hops.get(self.selected_hop)
    }
}

pub fn run(target: &str) -> anyhow::Result<()> {
    enable_raw_mode().context("failed to enable terminal raw mode")?;
    let mut stdout = io::stdout();
    crossterm::execute!(
        stdout,
        crossterm::terminal::EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )
    .context("failed to configure terminal for native UI")?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("failed to initialize terminal backend")?;
    let tick_rate = Duration::from_millis(220);

    let mut app = NativeUiApp::new(target);

    let result = run_event_loop(&mut terminal, &mut app, tick_rate);

    disable_raw_mode().ok();
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::event::DisableMouseCapture,
        crossterm::terminal::LeaveAlternateScreen
    )
    .ok();
    terminal.show_cursor().ok();

    result
}

fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut NativeUiApp,
    tick_rate: Duration,
) -> anyhow::Result<()> {
    loop {
        terminal
            .draw(|frame| render(frame, app))
            .context("failed to render native UI frame")?;

        if event::poll(tick_rate).context("failed polling terminal events")? {
            if let Event::Key(key) = event::read().context("failed reading terminal events")? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Tab => app.active_tab = app.active_tab.next(),
                    KeyCode::BackTab => app.active_tab = app.active_tab.previous(),
                    KeyCode::Left => app.active_tab = app.active_tab.previous(),
                    KeyCode::Right => app.active_tab = app.active_tab.next(),
                    KeyCode::Char('j') | KeyCode::Down => {
                        app.selected_hop =
                            (app.selected_hop + 1).min(app.hops.len().saturating_sub(1));
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        app.selected_hop = app.selected_hop.saturating_sub(1);
                    }
                    KeyCode::Char(' ') => app.paused = !app.paused,
                    KeyCode::Char('s') => {
                        app.sort_mode = app.sort_mode.next();
                        app.apply_sort();
                    }
                    _ => {}
                }
            }
        } else {
            app.tick();
        }
    }

    Ok(())
}

fn render(frame: &mut ratatui::Frame, app: &NativeUiApp) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .split(frame.area());

    let titles = ["Overview", "Hops", "Stats", "Help"]
        .into_iter()
        .map(|title| {
            Line::from(Span::styled(
                title,
                Style::default().fg(Color::Rgb(100, 220, 255)),
            ))
        })
        .collect::<Vec<_>>();

    let status = if app.paused { "PAUSED" } else { "LIVE" };
    let header_title = format!(
        " windows-mtr native UI • target={} • sort={} • {} ",
        app.target,
        app.sort_mode.label(),
        status
    );

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(header_title)
                .border_style(Style::default().fg(Color::Rgb(80, 140, 255))),
        )
        .select(app.active_tab.index())
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Rgb(120, 220, 255))
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(tabs, outer[0]);

    match app.active_tab {
        Tab::Overview => render_overview(frame, app, outer[1]),
        Tab::Hops => render_hops(frame, app, outer[1]),
        Tab::Stats => render_stats(frame, app, outer[1]),
        Tab::Help => render_help(frame, app, outer[1]),
    }

    let footer = Paragraph::new(
        "q quit • tab switch tab • ↑/↓ or j/k select hop • s sort • space pause/resume",
    )
    .style(Style::default().fg(Color::Rgb(210, 210, 210)))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(footer, outer[2]);
}

fn render_overview(frame: &mut ratatui::Frame, app: &NativeUiApp, area: Rect) {
    let top_bottom = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Min(8)])
        .split(area);

    let metric_row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(top_bottom[0]);

    render_metric_gauge(
        frame,
        metric_row[0],
        "Packet Loss",
        app.global_loss_pct().clamp(0.0, 100.0),
        Color::Rgb(250, 120, 170),
        "%",
    );
    render_metric_gauge(
        frame,
        metric_row[1],
        "Quality Score",
        app.quality_score(),
        Color::Rgb(80, 240, 180),
        "",
    );
    render_metric_gauge(
        frame,
        metric_row[2],
        "Avg Latency",
        (app.avg_latency() * 2.0).clamp(0.0, 100.0),
        Color::Rgb(120, 200, 255),
        "ms",
    );

    let lower = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(top_bottom[1]);

    let left_stack = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(6), Constraint::Min(8)])
        .split(lower[0]);

    let spark = Sparkline::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Latency sparkline")
                .border_style(Style::default().fg(Color::Rgb(80, 140, 255))),
        )
        .data(app.sparkline_data())
        .style(Style::default().fg(Color::Rgb(80, 240, 180)));
    frame.render_widget(spark, left_stack[0]);

    let data = vec![
        Dataset::default()
            .name("latency")
            .marker(symbols::Marker::Braille)
            .style(Style::default().fg(Color::Rgb(120, 220, 255)))
            .data(&app.latency_history),
    ];

    let x_max = app.latency_history.last().map_or(80.0, |(x, _)| *x);
    let x_min = (x_max - 100.0).max(0.0);
    let chart = Chart::new(data)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Latency trend (ms)")
                .border_style(Style::default().fg(Color::Rgb(80, 140, 255))),
        )
        .x_axis(Axis::default().bounds([x_min, x_max]).title("samples"))
        .y_axis(Axis::default().bounds([0.0, 40.0]).title("ms"));
    frame.render_widget(chart, left_stack[1]);

    let selected = app.selected_hop();
    let (badge, badge_color) = app.status_badge();
    let mut right_panel_lines = vec![
        Line::from(format!("Target: {}", app.target)),
        Line::from(format!("Cycles: {}", app.cycles)),
        Line::from(format!("Uptime: {}s", app.started_at.elapsed().as_secs())),
        Line::from(format!("Jitter: {:.2} ms", app.latency_jitter())),
        Line::from(Span::styled(
            format!("Health: {badge}"),
            Style::default()
                .fg(badge_color)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Selected hop",
            Style::default()
                .fg(Color::Rgb(255, 215, 100))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(format!(
            "#{} {}",
            selected.map(|h| h.hop).unwrap_or(0),
            selected.map(|h| h.host.as_str()).unwrap_or("n/a")
        )),
        Line::from(format!(
            "loss={:.1}% last={:.1}ms avg={:.1}ms",
            selected.map(|h| h.loss_pct).unwrap_or(0.0),
            selected.map(|h| h.last_ms).unwrap_or(0.0),
            selected.map(|h| h.avg_ms).unwrap_or(0.0),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Active alerts",
            Style::default()
                .fg(Color::Rgb(255, 215, 100))
                .add_modifier(Modifier::BOLD),
        )),
    ];
    for alert in app.active_alerts().into_iter().take(2) {
        right_panel_lines.push(Line::from(format!("• {alert}")));
    }

    let right = Paragraph::new(right_panel_lines)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Session snapshot")
                .border_style(Style::default().fg(Color::Rgb(80, 140, 255))),
        );
    frame.render_widget(right, lower[1]);
}

fn render_metric_gauge(
    frame: &mut ratatui::Frame,
    area: Rect,
    title: &str,
    value: f64,
    color: Color,
    unit: &str,
) {
    let display_value = value.clamp(0.0, 100.0);
    let label = if unit.is_empty() {
        format!("{display_value:.1}")
    } else {
        format!("{display_value:.1}{unit}")
    };

    let gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(Color::Rgb(80, 140, 255))),
        )
        .gauge_style(Style::default().fg(color))
        .percent(display_value.round() as u16)
        .label(label);
    frame.render_widget(gauge, area);
}

fn render_hops(frame: &mut ratatui::Frame, app: &NativeUiApp, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(8), Constraint::Length(5)])
        .split(area);

    let header = Row::new(vec![
        "Hop", "Host", "Loss%", "Last", "Avg", "Best", "Worst", "Trend",
    ])
    .style(
        Style::default()
            .fg(Color::Rgb(255, 220, 140))
            .add_modifier(Modifier::BOLD),
    );

    let rows = app.hops.iter().enumerate().map(|(idx, hop)| {
        let selected_row = idx == app.selected_hop;
        let base = if selected_row {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Rgb(120, 220, 255))
        } else {
            Style::default().fg(Color::Rgb(220, 220, 220))
        };

        Row::new(vec![
            Cell::from(hop.hop.to_string()),
            Cell::from(hop.host.clone()),
            Cell::from(format!("{:.1}", hop.loss_pct))
                .style(loss_cell_style(hop.loss_pct, selected_row)),
            Cell::from(format!("{:.1}ms", hop.last_ms))
                .style(latency_cell_style(hop.last_ms, selected_row)),
            Cell::from(format!("{:.1}ms", hop.avg_ms))
                .style(latency_cell_style(hop.avg_ms, selected_row)),
            Cell::from(format!("{:.1}ms", hop.best_ms)).style(base),
            Cell::from(format!("{:.1}ms", hop.worst_ms))
                .style(latency_cell_style(hop.worst_ms, selected_row)),
            Cell::from(latency_bar(hop.avg_ms)).style(base),
        ])
        .style(base)
    });

    let table = Table::new(
        rows,
        [
            Constraint::Length(5),
            Constraint::Percentage(32),
            Constraint::Length(8),
            Constraint::Length(9),
            Constraint::Length(9),
            Constraint::Length(9),
            Constraint::Length(9),
            Constraint::Length(10),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Hop table")
            .border_style(Style::default().fg(Color::Rgb(80, 140, 255))),
    )
    .row_highlight_style(Style::default().add_modifier(Modifier::BOLD));

    frame.render_widget(table, chunks[0]);

    let detail = app.selected_hop().map_or_else(
        || vec![Line::from("No hop selected")],
        |hop| {
            vec![
                Line::from(Span::styled(
                    format!("Hop #{} • {}", hop.hop, hop.host),
                    Style::default()
                        .fg(Color::Rgb(255, 215, 100))
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(format!(
                    "loss={:.1}% last={:.1}ms avg={:.1}ms best={:.1}ms worst={:.1}ms",
                    hop.loss_pct, hop.last_ms, hop.avg_ms, hop.best_ms, hop.worst_ms
                )),
                Line::from(format!("sort mode: {}", app.sort_mode.label())),
            ]
        },
    );

    let detail_widget = Paragraph::new(detail)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Selected hop details")
                .border_style(Style::default().fg(Color::Rgb(80, 140, 255))),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(detail_widget, chunks[1]);
}

fn latency_cell_style(value_ms: f64, selected: bool) -> Style {
    if selected {
        return Style::default()
            .fg(Color::Black)
            .bg(Color::Rgb(120, 220, 255));
    }

    let fg = if value_ms < 15.0 {
        Color::Rgb(110, 255, 180)
    } else if value_ms < 30.0 {
        Color::Rgb(255, 215, 100)
    } else {
        Color::Rgb(255, 130, 130)
    };
    Style::default().fg(fg)
}

fn loss_cell_style(value_pct: f64, selected: bool) -> Style {
    if selected {
        return Style::default()
            .fg(Color::Black)
            .bg(Color::Rgb(120, 220, 255));
    }

    let fg = if value_pct < 1.0 {
        Color::Rgb(110, 255, 180)
    } else if value_pct < 3.0 {
        Color::Rgb(255, 215, 100)
    } else {
        Color::Rgb(255, 130, 130)
    };
    Style::default().fg(fg)
}

fn latency_bar(value_ms: f64) -> String {
    let level = (value_ms / 5.0).round().clamp(0.0, 8.0) as usize;
    let filled = "█".repeat(level);
    let empty = "░".repeat(8usize.saturating_sub(level));
    format!("{filled}{empty}")
}

fn render_stats(frame: &mut ratatui::Frame, app: &NativeUiApp, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(area);

    let left = vec![
        Line::from(format!("Average latency: {:.2} ms", app.avg_latency())),
        Line::from(format!("Latency jitter: {:.2} ms", app.latency_jitter())),
        Line::from(format!("Global loss: {:.2}%", app.global_loss_pct())),
        Line::from(format!("Quality score: {:.1}/100", app.quality_score())),
        Line::from(format!("Hop count: {}", app.hops.len())),
        Line::from(format!("Cycles completed: {}", app.cycles)),
        Line::from(""),
        Line::from("Quality score formula:"),
        Line::from("100 - (loss% * 6) - (avg_latency_ms * 1.2)"),
        Line::from(""),
        Line::from("UI cues inspired by modern Rust TUIs (compact cards + color semantics)."),
    ];

    let stats_panel = Paragraph::new(left)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Statistics")
                .border_style(Style::default().fg(Color::Rgb(80, 140, 255))),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(stats_panel, chunks[0]);

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(6),
        ])
        .split(chunks[1]);

    render_metric_gauge(
        frame,
        right_chunks[0],
        "Quality",
        app.quality_score(),
        Color::Rgb(80, 240, 180),
        "",
    );
    render_metric_gauge(
        frame,
        right_chunks[1],
        "Loss",
        app.global_loss_pct(),
        Color::Rgb(250, 120, 170),
        "%",
    );

    let history_spark = Sparkline::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Latency pulse")
                .border_style(Style::default().fg(Color::Rgb(80, 140, 255))),
        )
        .data(app.sparkline_data())
        .style(Style::default().fg(Color::Rgb(120, 220, 255)));
    frame.render_widget(history_spark, right_chunks[2]);
}

fn render_help(frame: &mut ratatui::Frame, app: &NativeUiApp, area: Rect) {
    let help = Paragraph::new(vec![
        Line::from(Span::styled(
            "Native UI key bindings",
            Style::default()
                .fg(Color::Rgb(255, 215, 100))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  q               Quit"),
        Line::from("  tab/shift+tab   Switch tabs"),
        Line::from("  ← / →           Switch tabs"),
        Line::from("  j/k or ↑/↓      Move selection in hop table"),
        Line::from("  s               Cycle sort mode (hop/loss/avg-latency)"),
        Line::from("  space           Pause/resume live updates"),
        Line::from(""),
        Line::from(format!("Current target: {}", app.target)),
        Line::from(format!("Current sort mode: {}", app.sort_mode.label())),
        Line::from(""),
        Line::from("This native UI is now designed as a richer dashboard scaffold."),
        Line::from("Next step is wiring these widgets to live traceroute probe streams."),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Help")
            .border_style(Style::default().fg(Color::Rgb(80, 140, 255))),
    )
    .wrap(Wrap { trim: true });

    frame.render_widget(help, area);
}
