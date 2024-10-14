use std::{
    collections::HashMap,
    io::{self, Write},
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use delegate::delegate;
use ratatui::{
    backend::{Backend, CrosstermBackend, WindowSize},
    layout::Size,
    Terminal,
};
use russh::{
    server::{Auth, Config, Handle, Handler, Msg, Server, Session},
    Channel, ChannelId, Pty,
};
use russh_keys::key::{KeyPair, PublicKey};

use game::Game;
use tokio::{sync::Mutex, time::sleep};
use tracing::{debug, info, instrument, level_filters::LevelFilter};
use tracing_subscriber::EnvFilter;
mod ball;
mod game;
mod paddle;
mod physics;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    init_tracing()?;
    let mut server = AppServer::new();
    server.run().await?;
    Ok(())
}

fn init_tracing() -> color_eyre::Result<()> {
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env()?
        .add_directive("pong_russh=debug".parse()?);
    tracing_subscriber::fmt()
        .compact()
        .with_env_filter(filter)
        .init();
    debug!("Tracing initialized");
    Ok(())
}

pub type SshTerminal = Terminal<SshBackend>;

/// A backend that writes to an SSH terminal.
///
/// This backend is a wrapper around the crossterm backend that writes to a terminal handle. It
/// delegates most of the methods to the inner crossterm backend, but overrides the methods related
/// to the terminal size and window size.
#[derive(Debug)]
pub struct SshBackend {
    inner: CrosstermBackend<TerminalHandle>,
    size: Size,
    window_size: WindowSize,
}

impl SshBackend {
    pub fn new(
        channel_id: ChannelId,
        session_handle: Handle,
        col_width: u32,
        row_height: u32,
        pix_width: u32,
        pix_height: u32,
    ) -> Self {
        let terminal_handle = TerminalHandle::new(channel_id, session_handle);
        let size = Size::new(col_width as u16, row_height as u16);
        let window_size = WindowSize {
            columns_rows: Size::new(col_width as u16, row_height as u16),
            pixels: Size::new(pix_width as u16, pix_height as u16),
        };
        Self {
            inner: CrosstermBackend::new(terminal_handle),
            size,
            window_size,
        }
    }
}

impl Backend for SshBackend {
    delegate! {
        to self.inner {
            #[allow(late_bound_lifetime_arguments)]
            fn draw<'a, I>(&mut self, content: I) -> std::io::Result<()>
            where
                I: Iterator<Item = (u16, u16, &'a ratatui::prelude::buffer::Cell)>;

            fn hide_cursor(&mut self) -> std::io::Result<()>;
            fn show_cursor(&mut self) -> std::io::Result<()>;
            #[allow(deprecated)]
            fn get_cursor(&mut self) -> std::io::Result<(u16, u16)>;
            #[allow(deprecated)]
            fn set_cursor(&mut self, x: u16, y: u16) -> std::io::Result<()>;
            fn get_cursor_position(&mut self) -> io::Result<ratatui::prelude::Position> ;
            fn set_cursor_position<P: Into<ratatui::prelude::Position>>(&mut self, position: P) -> io::Result<()> ;
            fn clear(&mut self) -> std::io::Result<()>;

        }
    }
    // can't delegate as there is a conflict with the `Write` trait
    fn flush(&mut self) -> io::Result<()> {
        Backend::flush(&mut self.inner)
    }
    fn size(&self) -> io::Result<Size> {
        Ok(self.size)
    }
    fn window_size(&mut self) -> io::Result<WindowSize> {
        Ok(self.window_size)
    }
}

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

        let addr = Ipv4Addr::LOCALHOST;
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

    #[instrument(skip(self, _public_key), err)]
    async fn auth_publickey(
        &mut self,
        _user: &str,
        _public_key: &PublicKey,
    ) -> Result<Auth, Self::Error> {
        info!(client_id = ?self.client_id, "Authenticating client");
        Ok(Auth::Accept)
    }

    #[instrument(skip(self, _session), err)]
    async fn channel_open_session(
        &mut self,
        _channel: Channel<Msg>,
        _session: &mut Session,
    ) -> Result<bool, Self::Error> {
        info!(client_id = ?self.client_id, "Opening session");
        let mut game = self.game.lock().await;
        game.connect_player(self.client_id)?;
        Ok(true)
    }

    #[instrument(skip(self, _session), err)]
    async fn channel_close(
        &mut self,
        _channel: ChannelId,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        info!(client_id = ?self.client_id, "Closing session");
        self.game.lock().await.disconnect_player(self.client_id);
        self.terminals.lock().await.remove(&self.client_id);
        Ok(())
    }

    async fn data(
        &mut self,
        channel_id: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        match data {
            // Pressing 'q' closes the connection.
            b"q" => {
                session.close(channel_id);
            }
            // Pressing 'c' resets the counter for the app.
            // Every client sees the counter reset.
            b"w" => self.game.lock().await.move_up(self.client_id),
            b"s" => self.game.lock().await.move_down(self.client_id),
            _ => {}
        }

        Ok(())
    }

    #[instrument(skip(self, _modes, session), err)]
    async fn pty_request(
        &mut self,
        channel_id: ChannelId,
        term: &str,
        col_width: u32,
        row_height: u32,
        pix_width: u32,
        pix_height: u32,
        _modes: &[(Pty, u32)],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        info!(client_id = ?self.client_id, "Creating terminal");
        let terminal = Terminal::new(SshBackend::new(
            channel_id,
            session.handle(),
            col_width,
            row_height,
            pix_width,
            pix_height,
        ))?;
        let mut terminals = self.terminals.lock().await;
        terminals.insert(self.client_id, terminal);

        Ok(())
    }

    /// The client's pseudo-terminal window size has changed.
    #[instrument(skip(self, session), err)]
    async fn window_change_request(
        &mut self,
        channel_id: ChannelId,
        col_width: u32,
        row_height: u32,
        pix_width: u32,
        pix_height: u32,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        info!(client_id = ?self.client_id, "Resizing terminal");
        let terminal = Terminal::new(SshBackend::new(
            channel_id,
            session.handle(),
            col_width,
            row_height,
            pix_width,
            pix_height,
        ))?;
        let mut terminals = self.terminals.lock().await;
        terminals.insert(self.client_id, terminal);

        Ok(())
    }
}

#[derive(Clone)]
pub struct TerminalHandle {
    handle: Handle,
    channel_id: ChannelId,
    // The sink collects the data which is finally flushed to the handle.
    sink: Vec<u8>,
}

impl TerminalHandle {
    pub fn new(channel_id: ChannelId, handle: Handle) -> Self {
        Self {
            handle,
            channel_id,
            sink: Vec::new(),
        }
    }
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
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.sink.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use russh::server::Handler;
    use russh_keys::key::KeyPair;

    #[tokio::test]
    async fn test_auth() {
        // JM 2024-10-14 I don't recall what this part of this test was supposed to do.
        // let config = Arc::new(Config {
        //     inactivity_timeout: Some(Duration::from_secs(3600)),
        //     auth_rejection_time: Duration::from_secs(3),
        //     auth_rejection_time_initial: Some(Duration::from_secs(0)),
        //     keys: vec![KeyPair::generate_ed25519().unwrap()],
        //     ..Default::default()
        // });

        // let addr = Ipv4Addr::UNSPECIFIED;
        // let port = 2222;
        let mut server = AppServer::new();
        let mut handler = server.new_client(None);
        let public_key = KeyPair::generate_ed25519()
            .unwrap()
            .clone_public_key()
            .unwrap();
        let result = handler.auth_publickey("test", &public_key);
        assert_eq!(result.await.unwrap(), Auth::Accept);
    }
}
