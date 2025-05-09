// Mock UI demo for Gossip visualization (no real protocol yet)
// -----------------------------------------------------------
// This standalone CLI shows an animated ring of nodes gradually
// turning red to simulate rumor spread. All network / protocol
// logic is stubbed out — replace the mock generator with real
// `gossip_core` events later.
// -----------------------------------------------------------

use std::collections::HashSet;
use std::io;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use clap::Parser;
use crossterm::event::{self, Event as CEvent, KeyCode};
use rand::{seq::SliceRandom, thread_rng};
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Style};
use ratatui::text::Span;
use ratatui::widgets::canvas::Canvas;
use ratatui::widgets::{Block, Borders, Gauge, Paragraph};
use ratatui::{Terminal, backend::CrosstermBackend};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

// ---- Mock event type ---------------------------------------------------------
#[derive(Debug, Clone)]
enum GossipEvent {
    RumorReceived { node_id: String },
}

#[derive(Parser, Debug)]
struct Opts {
    /// Comma‑separated list of node IDs to simulate
    #[arg(
        long,
        default_value = "A,B,C,D,E,F,G,H,I,J,K,L,M",
        value_delimiter = ','
    )]
    nodes: Vec<String>,
    /// Interval (ms) between infections
    #[arg(long, default_value_t = 500)]
    step_ms: u64,
}

struct UiState {
    nodes: Vec<String>,
    infected: HashSet<String>,
}

impl UiState {
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
    run_tui(opts.nodes, rx)?;
    Ok(())
}

// TODO: replace this with real gossip-core events
fn spawn(mut nodes: Vec<String>, step_ms: u64, tx: UnboundedSender<GossipEvent>) {
    use rand::thread_rng;

    // shuffle before the async block (ThreadRng is !Send)
    nodes.shuffle(&mut thread_rng());

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

            // simple log (shows in VS Code terminal once you exit alt-screen)
            // eprintln!("Infecting {:?}", batch);
        }
    });
}

fn run_tui(node_ids: Vec<String>, mut rx: UnboundedReceiver<GossipEvent>) -> io::Result<()> {
    // Set up terminal
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal: Terminal<CrosstermBackend<io::Stdout>> = Terminal::new(backend)?;

    let mut state = UiState {
        nodes: node_ids,
        infected: HashSet::new(),
    };
    let tick_rate = Duration::from_millis(60);
    let mut last_tick = Instant::now();

    loop {
        // keyboard quit
        if event::poll(Duration::from_millis(30))? {
            if let CEvent::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }

        // apply incoming events
        while let Ok(ev) = rx.try_recv() {
            if let GossipEvent::RumorReceived { node_id } = ev {
                state.infected.insert(node_id);
            }
        }

        if last_tick.elapsed() >= tick_rate {
            terminal.draw(|f: &mut ratatui::Frame| draw_ui(f, &state))?;
            last_tick = Instant::now();
        }
    }

    // cleanup
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;
    Ok(())
}

fn draw_ui(f: &mut ratatui::Frame, st: &UiState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5)])
        .split(f.size());

    // Footer with hint
    let header = Paragraph::new("Press 'q' to quit ")
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(header, chunks[1]);

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
            let n = st.nodes.len().max(1);
            for (i, id) in st.nodes.iter().enumerate() {
                let ang = 2.0 * PI * i as f64 / n as f64;
                let (x, y) = (ang.cos(), ang.sin());
                let infected = st.infected.contains(id);
                let color = if infected { Color::Red } else { Color::Gray };
                let node = "●";
                ctx.print(x, y, Span::styled(node, Style::default().fg(color)));
            }
        });
    f.render_widget(canvas, chunks[1]);
}
