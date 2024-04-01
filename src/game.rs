use std::time::{Duration, Instant};

use color_eyre::eyre::bail;
use ratatui::{
    prelude::*,
    widgets::{Block, Clear, WidgetRef},
};
use tracing::info;

use crate::{ball::Ball, paddle::Paddle, SshTerminal};

#[derive(Debug)]
pub struct Game {
    ball: Ball,
    left_paddle: Paddle,
    right_paddle: Paddle,
    score: (u32, u32),
    serve_time: Option<Instant>,
    last_update: Option<Instant>,
    clients: [Option<usize>; 2],
}

impl Game {
    // Wait for a fixed duration before serving the ball
    const SERVE_DURATION: Duration = Duration::from_millis(1500);

    pub fn new() -> Self {
        Self {
            ball: Ball::new(),
            left_paddle: Paddle::new(0.0, 0.5),
            right_paddle: Paddle::new(1.0, 0.5),
            score: (0, 0),
            serve_time: None,
            last_update: None,
            clients: [None, None],
        }
    }

    pub fn connect_player(&mut self, client_id: usize) -> color_eyre::Result<()> {
        if self.clients[0].is_none() {
            info!("Player 1 connected");
            self.clients[0] = Some(client_id);
        } else if self.clients[1].is_none() {
            info!("Player 2 connected");
            self.clients[1] = Some(client_id);
        } else {
            bail!("Game is full");
        }
        if self.clients.iter().all(Option::is_some) {
            info!("Both players connected, starting game");
            self.score = (0, 0);
            self.serve();
        }
        Ok(())
    }

    pub fn disconnect_player(&mut self, client_id: usize) {
        if let Some(id) = self.clients.iter_mut().find(|id| **id == Some(client_id)) {
            info!("Player disconnected");
            *id = None;
        }
    }

    pub fn draw(&mut self, terminal: &mut SshTerminal) -> color_eyre::Result<()> {
        terminal.draw(|frame| frame.render_widget_ref(self, frame.size()))?;
        Ok(())
    }

    pub fn move_up(&mut self, client_id: usize) {
        if self.clients[0]
            .as_ref()
            .map_or(false, |id| *id == client_id)
        {
            self.left_paddle.move_up();
        } else if self.clients[1]
            .as_ref()
            .map_or(false, |id| *id == client_id)
        {
            self.right_paddle.move_up();
        }
    }

    pub fn move_down(&mut self, client_id: usize) {
        if self.clients[0]
            .as_ref()
            .map_or(false, |id| *id == client_id)
        {
            self.left_paddle.move_down();
        } else if self.clients[1]
            .as_ref()
            .map_or(false, |id| *id == client_id)
        {
            self.right_paddle.move_down();
        }
    }

    pub fn update(&mut self) {
        if self
            .serve_time
            .map_or(true, |t| t.elapsed() < Self::SERVE_DURATION)
        {
            return;
        }
        let duration = self.last_update.map_or(Duration::ZERO, |t| t.elapsed());
        self.last_update = Some(Instant::now());
        self.ball
            .update(duration, &self.left_paddle, &self.right_paddle);

        if self.ball.pos.x < 0.0 {
            self.score.1 += 1;
            self.serve();
        } else if self.ball.pos.x > 1.0 {
            self.score.0 += 1;
            self.serve();
        }
    }

    pub fn serve(&mut self) {
        info!("Serving ball");
        self.ball.serve();
        self.serve_time = Some(Instant::now());
        self.last_update = None;
    }
}

impl WidgetRef for &mut Game {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);
        let block = Block::bordered()
            .title("Pong")
            .title_alignment(Alignment::Center)
            .style((Color::White, Color::DarkGray));
        (&block).render(area, buf);
        let area = block.inner(area);
        Line::from(format!("Score: {} - {}", self.score.0, self.score.1))
            .centered()
            .render(area, buf);
        self.ball.render(area, buf);
        self.left_paddle.render(area, buf);
        self.right_paddle.render(area, buf);
    }
}
