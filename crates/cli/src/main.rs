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
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

#[derive(Debug, Clone)]
enum GossipEvent {
    RumorReceived { node_id: String },
}

#[derive(Parser, Debug)]
struct Opts {
    #[arg(
        long,
        default_value = "A,B,C,D,E,F,G,H,I,J,K,L,M,N,O",
        value_delimiter = ','
    )]
    nodes: Vec<String>,
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
    let (tx, rx) = unbounded_channel();

    // Spawn mock infection generator
    spawn(opts.nodes.clone(), opts.step_ms, tx);

    // Run the TUI (blocks until 'q')
    run(opts.nodes, rx)?;
    Ok(())
}

// TODO: replace this with real gossip-core events
fn spawn(mut nodes: Vec<String>, step_ms: u64, tx: UnboundedSender<GossipEvent>) {
    use rand::rng;

    // shuffle before the async block (ThreadRng is !Send)
    nodes.shuffle(&mut rng());

    tokio::spawn(async move {
        while !nodes.is_empty() {
            tokio::time::sleep(Duration::from_millis(step_ms)).await;

            // take up to 3 nodes each round
            let batch: Vec<String> = (0..3).filter_map(|_| nodes.pop()).collect();

            // send an event per node
            for id in &batch {
                let _ = tx.send(GossipEvent::RumorReceived {
                    node_id: id.clone(),
                });
            }
        }
    });
}

fn run(node_ids: Vec<String>, mut rx: UnboundedReceiver<GossipEvent>) -> io::Result<()> {
    // Set up terminal
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

    state.infected.insert(state.nodes[0].clone());

    loop {
        if event::poll(Duration::from_millis(30))? {
            if let CEvent::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }

        // apply incoming events
        while let Ok(ev) = rx.try_recv() {
            let GossipEvent::RumorReceived { node_id } = ev;
            state.infected.insert(node_id);
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

    // progress bar
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
