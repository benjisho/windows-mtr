use anyhow::Context;
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
use std::io::{self, Stdout};
use std::time::{Duration, Instant};

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
    started: Instant,
}

impl NativeUiApp {
    fn new(target: &str) -> Self {
        Self {
            target: target.to_string(),
            tab_index: 0,
            hops: Vec::new(),
            latency_history: Vec::new(),
            loss_history: Vec::new(),
            started: Instant::now(),
        }
    }

    fn update(&mut self) {
        let t = self.started.elapsed().as_secs_f64();
        let mut hops = Vec::with_capacity(8);

        for hop in 1..=8 {
            let wave = (t + hop as f64 * 0.6).sin().abs();
            let base = 8.0 + hop as f64 * 4.0;
            let avg_ms = base + (wave * 30.0);
            let best_ms = (avg_ms * 0.7).max(1.0);
            let worst_ms = avg_ms * 1.4;
            let loss_pct = ((t / 3.0 + hop as f64).sin().abs() * 4.0).min(100.0);
            let host = if hop < 8 {
                format!("hop-{hop}.local")
            } else {
                self.target.clone()
            };

            hops.push(HopStat {
                hop,
                host,
                loss_pct,
                best_ms,
                avg_ms,
                worst_ms,
            });
        }

        self.hops = hops;

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

pub fn run_native_ui(target: &str) -> anyhow::Result<i32> {
    enable_raw_mode().context("failed to enable raw mode for native UI")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).context("failed to enter alternate screen")?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("failed to initialize terminal backend")?;

    let result = run_ui_loop(&mut terminal, target);

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
) -> anyhow::Result<i32> {
    let mut app = NativeUiApp::new(target);
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(700);

    loop {
        if last_tick.elapsed() >= tick_rate {
            app.update();
            last_tick = Instant::now();
        }

        terminal.draw(|f| draw_ui(f, &app))?;

        if event::poll(Duration::from_millis(100)).context("failed to poll terminal events")? {
            if let Event::Key(key) = event::read().context("failed to read terminal event")? {
                match key.code {
                    KeyCode::Char('q') => return Ok(0),
                    KeyCode::Right | KeyCode::Tab => app.next_tab(),
                    KeyCode::Left => app.prev_tab(),
                    _ => {}
                }
            }
        }
    }
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
                .title(format!("windows-mtr native UI preview ({})", app.target))
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

    let help = Paragraph::new("Controls: ←/→ or Tab switch tabs • q quits")
        .block(Block::default().borders(Borders::ALL).title("Help"));
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
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Hop table (native ratatui)"),
    );

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
