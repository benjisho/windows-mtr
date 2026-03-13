use anyhow::Context;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{Axis, Block, Borders, Cell, Chart, Dataset, Paragraph, Row, Table, Tabs},
};
use std::{io, time::Duration};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum UiTab {
    Hops,
    Latency,
    Loss,
}

impl UiTab {
    fn all() -> [Self; 3] {
        [Self::Hops, Self::Latency, Self::Loss]
    }

    fn title(self) -> &'static str {
        match self {
            Self::Hops => "Hop table",
            Self::Latency => "Latency chart",
            Self::Loss => "Loss chart",
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Hops => Self::Latency,
            Self::Latency => Self::Loss,
            Self::Loss => Self::Hops,
        }
    }

    fn previous(self) -> Self {
        match self {
            Self::Hops => Self::Loss,
            Self::Latency => Self::Hops,
            Self::Loss => Self::Latency,
        }
    }
}

#[derive(Clone)]
struct HopRow {
    hop: u8,
    host: String,
    loss_pct: f64,
    sent: u32,
    last_ms: f64,
    avg_ms: f64,
    best_ms: f64,
    worst_ms: f64,
}

pub fn run_native_ui(target: &str) -> anyhow::Result<i32> {
    let mut stdout = io::stdout();
    enable_raw_mode().context("failed to enable terminal raw mode")?;
    execute!(stdout, EnterAlternateScreen).context("failed to enter alternate screen")?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("failed to initialize terminal backend")?;

    let app_result = run_event_loop(&mut terminal, target);

    disable_raw_mode().ok();
    execute!(terminal.backend_mut(), LeaveAlternateScreen).ok();
    terminal.show_cursor().ok();

    app_result
}

fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    target: &str,
) -> anyhow::Result<i32> {
    let hops = demo_hops(target);
    let latency_points = chart_points(&hops, |h| h.avg_ms);
    let loss_points = chart_points(&hops, |h| h.loss_pct);
    let mut active_tab = UiTab::Hops;

    loop {
        terminal
            .draw(|frame| {
                let area = frame.area();
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3),
                        Constraint::Min(6),
                        Constraint::Length(2),
                    ])
                    .split(area);

                render_tabs(frame, chunks[0], active_tab);
                match active_tab {
                    UiTab::Hops => render_hop_table(frame, chunks[1], &hops),
                    UiTab::Latency => render_chart(
                        frame,
                        chunks[1],
                        "Avg RTT (ms)",
                        "ms",
                        &latency_points,
                        Color::Cyan,
                    ),
                    UiTab::Loss => render_chart(
                        frame,
                        chunks[1],
                        "Packet loss (%)",
                        "%",
                        &loss_points,
                        Color::Yellow,
                    ),
                }
                render_help(frame, chunks[2]);
            })
            .context("failed to render native ratatui frame")?;

        if !event::poll(Duration::from_millis(200)).context("failed to poll keyboard events")? {
            continue;
        }

        if let Event::Key(key) = event::read().context("failed to read keyboard event")? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(0),
                KeyCode::Right | KeyCode::Tab => {
                    active_tab = active_tab.next();
                }
                KeyCode::Left | KeyCode::BackTab => {
                    active_tab = active_tab.previous();
                }
                _ => {}
            }
        }
    }
}

fn render_tabs(frame: &mut ratatui::Frame<'_>, area: Rect, active_tab: UiTab) {
    let tabs = UiTab::all()
        .iter()
        .map(|tab| Line::from(Span::raw(tab.title())))
        .collect::<Vec<_>>();
    let selected = UiTab::all()
        .iter()
        .position(|tab| *tab == active_tab)
        .unwrap_or(0);

    let widget = Tabs::new(tabs)
        .block(
            Block::default()
                .title("Native Ratatui UI (Roadmap Preview)")
                .borders(Borders::ALL),
        )
        .select(selected)
        .highlight_style(
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(widget, area);
}

fn render_hop_table(frame: &mut ratatui::Frame<'_>, area: Rect, hops: &[HopRow]) {
    let header = Row::new(vec![
        Cell::from("Hop"),
        Cell::from("Host"),
        Cell::from("Loss%"),
        Cell::from("Snt"),
        Cell::from("Last"),
        Cell::from("Avg"),
        Cell::from("Best"),
        Cell::from("Wrst"),
    ])
    .style(
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    );

    let rows = hops.iter().map(|hop| {
        Row::new(vec![
            Cell::from(hop.hop.to_string()),
            Cell::from(hop.host.clone()),
            Cell::from(format!("{:.1}", hop.loss_pct)),
            Cell::from(hop.sent.to_string()),
            Cell::from(format!("{:.1}", hop.last_ms)),
            Cell::from(format!("{:.1}", hop.avg_ms)),
            Cell::from(format!("{:.1}", hop.best_ms)),
            Cell::from(format!("{:.1}", hop.worst_ms)),
        ])
    });

    let table = Table::new(
        rows,
        [
            Constraint::Length(4),
            Constraint::Length(30),
            Constraint::Length(8),
            Constraint::Length(6),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(8),
        ],
    )
    .header(header)
    .block(Block::default().title("Hop table").borders(Borders::ALL));

    frame.render_widget(table, area);
}

fn render_chart(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    title: &str,
    unit: &str,
    points: &[(f64, f64)],
    color: Color,
) {
    let max_y = points.iter().map(|(_, y)| *y).fold(1.0f64, f64::max).ceil();

    let dataset = Dataset::default()
        .name(title)
        .marker(symbols::Marker::Braille)
        .style(Style::default().fg(color))
        .graph_type(ratatui::widgets::GraphType::Line)
        .data(points);

    let chart = Chart::new(vec![dataset])
        .block(Block::default().title(title).borders(Borders::ALL))
        .x_axis(
            Axis::default()
                .title("Hop")
                .bounds([1.0, points.len() as f64])
                .labels(vec![Line::from("1"), Line::from(points.len().to_string())]),
        )
        .y_axis(
            Axis::default()
                .title(unit)
                .bounds([0.0, max_y])
                .labels(vec![Line::from("0"), Line::from(format!("{max_y:.0}"))]),
        );

    frame.render_widget(chart, area);
}

fn render_help(frame: &mut ratatui::Frame<'_>, area: Rect) {
    let help = Paragraph::new("Tab/→/← switch views • q/Esc quit")
        .block(Block::default().borders(Borders::ALL).title("Controls"));
    frame.render_widget(help, area);
}

fn chart_points(hops: &[HopRow], value: impl Fn(&HopRow) -> f64) -> Vec<(f64, f64)> {
    hops.iter().map(|h| (f64::from(h.hop), value(h))).collect()
}

fn demo_hops(target: &str) -> Vec<HopRow> {
    vec![
        HopRow {
            hop: 1,
            host: "gateway.local".to_string(),
            loss_pct: 0.0,
            sent: 20,
            last_ms: 1.2,
            avg_ms: 1.5,
            best_ms: 0.9,
            worst_ms: 4.0,
        },
        HopRow {
            hop: 2,
            host: "10.10.0.1".to_string(),
            loss_pct: 0.0,
            sent: 20,
            last_ms: 6.8,
            avg_ms: 7.3,
            best_ms: 5.9,
            worst_ms: 11.2,
        },
        HopRow {
            hop: 3,
            host: "core1.isp.net".to_string(),
            loss_pct: 0.5,
            sent: 20,
            last_ms: 14.1,
            avg_ms: 13.8,
            best_ms: 12.2,
            worst_ms: 20.0,
        },
        HopRow {
            hop: 4,
            host: "edge5.isp.net".to_string(),
            loss_pct: 1.0,
            sent: 20,
            last_ms: 22.4,
            avg_ms: 21.7,
            best_ms: 19.8,
            worst_ms: 30.5,
        },
        HopRow {
            hop: 5,
            host: target.to_string(),
            loss_pct: 0.0,
            sent: 20,
            last_ms: 28.6,
            avg_ms: 29.1,
            best_ms: 26.3,
            worst_ms: 36.8,
        },
    ]
}
