use std::collections::HashSet;
use std::io;
use std::time::{Duration, Instant};

use clap::Parser;
use crossterm::event::{self, Event as CEvent, KeyCode};
use rand::seq::SliceRandom;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Style};
use ratatui::text::Span;
use ratatui::widgets::canvas::Canvas;
use ratatui::widgets::{Block, Borders, Gauge};
use ratatui::{Terminal, backend::CrosstermBackend};
use tokio::sync::broadcast;

#[derive(Debug, Clone)]
enum GossipEvent {
    RumorReceived { node_id: String },
}

#[derive(Debug, Clone)]
enum Event {
    Step,
    Gossip(GossipEvent),
}

#[derive(Parser, Debug)]
struct Opts {
    #[arg(
        long,
        default_value = "A,B,C,D,E,F,G,H,I,J,K,L,M,N,O",
        value_delimiter = ','
    )]
    nodes: Vec<String>,

    #[arg(long = "break", default_value_t = false)]
    break_mode: bool,

    /// Interval (ms) between infections
    #[arg(long, default_value_t = 1000)]
    step_ms: u64,
}

struct State {
    nodes: Vec<String>,
    infected: HashSet<String>,
}

impl State {
    fn percent(&self) -> f64 {
        if self.nodes.is_empty() {
            0.0
        } else {
            self.infected.len() as f64 / self.nodes.len() as f64
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opts = Opts::parse();
    let (tx, _rx) = broadcast::channel(16);

    spawn(
        opts.nodes.clone(),
        opts.step_ms,
        tx.clone(),
        opts.break_mode,
    );

    run(opts.nodes, tx.clone(), opts.break_mode)?;
    Ok(())
}

// TODO: replace this with real gossip-core events
fn spawn(mut nodes: Vec<String>, step_ms: u64, tx: broadcast::Sender<Event>, break_mode: bool) {
    use rand::rng;

    // shuffle before the async block (ThreadRng is !Send)
    nodes.shuffle(&mut rng());
    let mut rx = tx.subscribe();

    tokio::spawn(async move {
        while !nodes.is_empty() {
            if break_mode {
                loop {
                    match rx.recv().await {
                        Ok(Event::Step) => break,
                        Ok(_) => continue,
                        Err(_) => {
                            return;
                        }
                    }
                }
            } else {
                tokio::time::sleep(Duration::from_millis(step_ms)).await;
            }
            // take up to 3 nodes each round
            let batch: Vec<String> = (0..3).filter_map(|_| nodes.pop()).collect();

            // send an event per node
            for id in &batch {
                let _ = tx.send(Event::Gossip(GossipEvent::RumorReceived {
                    node_id: id.to_string(),
                }));
            }
        }
    });
}

fn run(node_ids: Vec<String>, tx: broadcast::Sender<Event>, break_mode: bool) -> io::Result<()> {
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal: Terminal<CrosstermBackend<io::Stdout>> = Terminal::new(backend)?;

    let mut state = State {
        nodes: node_ids,
        infected: HashSet::new(),
    };
    let tick_rate = Duration::from_millis(60);
    let mut last_tick = Instant::now();

    let mut rx = tx.subscribe();

    state.infected.insert(state.nodes[0].clone());

    loop {
        if event::poll(Duration::from_millis(30))? {
            if let CEvent::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('p') | KeyCode::Char(' ') if break_mode => {
                        let _ = tx.send(Event::Step);
                    }
                    _ => {}
                }
            }
        }

        // apply incoming events
        while let Ok(ev) = rx.try_recv() {
            if let Event::Gossip(GossipEvent::RumorReceived { node_id }) = ev {
                state.infected.insert(node_id);
            }
        }

        if last_tick.elapsed() >= tick_rate {
            terminal.draw(|f: &mut ratatui::Frame| draw_ui(f, &state))?;
            last_tick = Instant::now();
        }
    }

    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;
    Ok(())
}

fn draw_ui(f: &mut ratatui::Frame, st: &State) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(5)])
        .split(f.area());

    let gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Convergence "),
        )
        .gauge_style(Style::default().fg(Color::Green))
        .ratio(st.percent());
    f.render_widget(gauge, chunks[0]);

    use std::f64::consts::PI;

    let canvas = Canvas::default()
        .block(Block::default().borders(Borders::ALL).title(" Nodes "))
        .x_bounds([-1.2, 1.2])
        .y_bounds([-1.2, 1.2])
        .paint(|ctx| {
            let n = st.nodes.len().max(1) as f64;
            let label_offset = 0.20;

            for (i, id) in st.nodes.iter().enumerate() {
                let ang = PI + (2.0 * PI) * (i as f64 / n);
                let (x, y) = (ang.cos(), ang.sin());
                let infected = st.infected.contains(id);
                let color = if infected || id == "A" {
                    Color::Green
                } else {
                    Color::DarkGray
                };

                let node = "⬤";
                ctx.print(x, y, Span::styled(node, Style::default().fg(color)));

                ctx.print(
                    x,
                    y + label_offset,
                    Span::styled(id.clone(), Style::default().fg(Color::White)),
                );
            }
        });
    f.render_widget(canvas, chunks[1]);
}
