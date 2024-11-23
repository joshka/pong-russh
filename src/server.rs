use std::{
    collections::HashMap,
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use ratatui::Terminal;
use russh::{
    keys::key::{KeyPair, PublicKey},
    server::{Auth, Config, Handler, Msg, Server, Session},
    Channel, ChannelId, Pty,
};
use tokio::{sync::Mutex, time::sleep};
use tracing::{info, instrument};

use crate::{backend::SshBackend, game::Game};

pub type SshTerminal = Terminal<SshBackend>;

#[derive(Debug, Clone)]
pub struct AppServer {
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
            keys: vec![KeyPair::generate_ed25519()],
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
pub struct AppHandler {
    pub client_id: usize,
    pub game: Arc<Mutex<Game>>,
    pub terminals: Arc<Mutex<HashMap<usize, SshTerminal>>>,
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

#[cfg(test)]
pub mod tests {
    use super::*;
    use russh::{keys::key::KeyPair, server::Handler};

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
        let public_key = KeyPair::generate_ed25519().clone_public_key().unwrap();
        let result = handler.auth_publickey("test", &public_key);
        assert_eq!(result.await.unwrap(), Auth::Accept);
    }
}
