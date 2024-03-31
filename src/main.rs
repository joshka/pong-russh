use std::{
    collections::HashMap,
    io::Write,
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use color_eyre::eyre::Context;
use ratatui::{backend::CrosstermBackend, layout::Rect, Terminal, Viewport};
use russh::{
    server::{Auth, Config, Handle, Handler, Msg, Server, Session},
    Channel, ChannelId, Pty,
};
use russh_keys::key::{KeyPair, PublicKey};

use game::Game;
use tokio::{sync::Mutex, time::sleep};
use tracing::{info, instrument};
mod ball;
mod game;
mod paddle;
mod physics;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    tracing_subscriber::fmt::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env().add_directive("info".parse()?),
        )
        .init();
    color_eyre::install()?;
    let mut server = AppServer::new();
    server.run().await?;
    Ok(())
}

type SshTerminal = Terminal<CrosstermBackend<TerminalHandle>>;

#[derive(Debug, Clone)]
struct AppServer {
    client_counter: usize,
    game: Arc<Mutex<Game>>,
    terminals: Arc<Mutex<HashMap<usize, SshTerminal>>>,
}

impl AppServer {
    pub fn new() -> Self {
        Self {
            client_counter: 0,
            game: Arc::new(Mutex::new(Game::new())),
            terminals: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn run(&mut self) -> color_eyre::Result<()> {
        let game = self.game.clone();
        let terminals = self.terminals.clone();
        tokio::spawn(async move {
            loop {
                sleep(tokio::time::Duration::from_millis(16)).await;
                game.lock().await.update();
                for terminal in terminals.lock().await.values_mut() {
                    game.lock().await.draw(terminal).unwrap();
                }
            }
        });

        let config = Arc::new(Config {
            inactivity_timeout: Some(Duration::from_secs(3600)),
            auth_rejection_time: Duration::from_secs(3),
            auth_rejection_time_initial: Some(Duration::from_secs(0)),
            keys: vec![KeyPair::generate_ed25519().unwrap()],
            ..Default::default()
        });

        let addr = Ipv4Addr::UNSPECIFIED;
        let port = 2222;
        info!("Listening on {}:{}", addr, port);
        self.run_on_address(config, (addr, port)).await?;
        Ok(())
    }
}

impl Server for AppServer {
    type Handler = AppHandler;
    fn new_client(&mut self, _peer_addr: Option<SocketAddr>) -> AppHandler {
        self.client_counter += 1;
        info!("New client connected: {}", self.client_counter);
        AppHandler::new(
            self.client_counter,
            self.game.clone(),
            self.terminals.clone(),
        )
    }
}

#[derive(Debug)]
struct AppHandler {
    client_id: usize,
    game: Arc<Mutex<Game>>,
    terminals: Arc<Mutex<HashMap<usize, SshTerminal>>>,
}

impl AppHandler {
    pub fn new(
        id: usize,
        game: Arc<Mutex<Game>>,
        terminals: Arc<Mutex<HashMap<usize, SshTerminal>>>,
    ) -> Self {
        Self {
            client_id: id,
            game,
            terminals,
        }
    }
}

#[async_trait]
impl Handler for AppHandler {
    type Error = color_eyre::Report;

    async fn auth_publickey(
        &mut self,
        _user: &str,
        _public_key: &PublicKey,
    ) -> Result<Auth, Self::Error> {
        Ok(Auth::Accept)
    }

    async fn channel_open_session(
        &mut self,
        channel: Channel<Msg>,
        session: &mut Session,
    ) -> Result<bool, Self::Error> {
        info!("Opening session for client {}", self.client_id);
        let terminal_handle = TerminalHandle {
            handle: session.handle(),
            sink: Vec::new(),
            channel_id: channel.id(),
        };
        let backend = CrosstermBackend::new(terminal_handle);
        let initial_viewport = Rect::new(0, 0, 85, 25);
        let terminal = Terminal::with_options(
            backend,
            ratatui::TerminalOptions {
                viewport: Viewport::Fixed(initial_viewport),
            },
        )?;

        let mut terminals = self.terminals.lock().await;
        terminals.insert(self.client_id, terminal);

        let mut game = self.game.lock().await;
        game.connect_player(self.client_id)?;

        Ok(true)
    }

    async fn channel_close(
        &mut self,
        _channel: ChannelId,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        info!("Closing session for client {}", self.client_id);
        self.game.lock().await.disconnect_player(self.client_id);
        self.terminals.lock().await.remove(&self.client_id);
        Ok(())
    }

    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        match data {
            // Pressing 'q' closes the connection.
            b"q" => {
                session.close(channel);
            }
            // Pressing 'c' resets the counter for the app.
            // Every client sees the counter reset.
            b"w" => self.game.lock().await.move_up(self.client_id),
            b"s" => self.game.lock().await.move_down(self.client_id),
            _ => {}
        }

        Ok(())
    }

    #[instrument(skip_all, err)]
    async fn pty_request(
        &mut self,
        _channel: ChannelId,
        term: &str,
        col_width: u32,
        row_height: u32,
        _pix_width: u32,
        _pix_height: u32,
        _modes: &[(Pty, u32)],
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        info!(client_id = ?self.client_id, ?term, ?col_width, ?row_height, "PTY request");
        let size = Rect::new(0, 0, col_width as u16, row_height as u16);
        self.game.lock().await.resize(self.client_id, size);
        Ok(())
    }

    /// The client's pseudo-terminal window size has changed.
    async fn window_change_request(
        &mut self,
        _: ChannelId,
        col_width: u32,
        row_height: u32,
        _: u32,
        _: u32,
        _: &mut Session,
    ) -> Result<(), Self::Error> {
        info!(
            "Resizing terminal for client {} to {}x{}",
            self.client_id, col_width, row_height
        );
        let area = Rect::new(0, 0, col_width as u16, row_height as u16);
        let mut terminals = self.terminals.lock().await;
        if let Some(terminal) = terminals.get_mut(&self.client_id) {
            terminal
                .resize(area)
                .wrap_err("Failed to resize terminal")?;
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct TerminalHandle {
    handle: Handle,
    // The sink collects the data which is finally flushed to the handle.
    sink: Vec<u8>,
    channel_id: ChannelId,
}

impl std::fmt::Debug for TerminalHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TerminalHandle")
            .field("handle", &"...")
            .field("sink", &self.sink)
            .field("channel_id", &self.channel_id)
            .finish()
    }
}

// The crossterm backend writes to the terminal handle.
impl Write for TerminalHandle {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.sink.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let handle = self.handle.clone();
        let channel_id = self.channel_id;
        let data = self.sink.clone().into();
        futures::executor::block_on(async move {
            let result = handle.data(channel_id, data).await;
            if result.is_err() {
                eprintln!("Failed to send data: {:?}", result);
            }
        });

        self.sink.clear();
        Ok(())
    }
}
