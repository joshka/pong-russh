use std::time::{Duration, Instant};

use crossterm::event::KeyCode;
use ratatui::prelude::*;

use crate::{ball::Ball, events, paddle::Paddle, tui::Tui};

#[derive(Debug)]
pub struct Game {
    ball: Ball,
    player1: Paddle,
    player2: Paddle,
    exit: bool,
    score: (u32, u32),
    serve_time: Option<Instant>,
    last_update: Option<Instant>,
}

impl Game {
    // Wait for a fixed duration before serving the ball
    const SERVE_DURATION: Duration = Duration::from_millis(1500);

    pub const fn new() -> Self {
        Self {
            ball: Ball::new(),
            player1: Paddle::new(0.0, 0.5),
            player2: Paddle::new(1.0, 0.5),
            exit: false,
            score: (0, 0),
            serve_time: None,
            last_update: None,
        }
    }

    pub fn run(&mut self, tui: &mut Tui) -> color_eyre::Result<()> {
        self.serve();
        while !self.exit {
            self.draw(tui)?;
            self.handle_input()?;
            self.update();
        }
        Ok(())
    }

    fn draw(&mut self, tui: &mut Tui) -> color_eyre::Result<()> {
        tui.draw(|frame| frame.render_widget(&*self, frame.size()))?;
        Ok(())
    }

    fn handle_input(&mut self) -> color_eyre::Result<()> {
        if let Some(key) = events::poll()? {
            if events::is_quit_key(key) {
                self.exit = true
            }
            match key.code {
                KeyCode::Char('w') => {
                    self.player1.move_up();
                }
                KeyCode::Char('s') => {
                    self.player1.move_down();
                }
                KeyCode::Up => {
                    self.player2.move_up();
                }
                KeyCode::Down => {
                    self.player2.move_down();
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn update(&mut self) {
        if self
            .serve_time
            .map_or(true, |t| t.elapsed() < Self::SERVE_DURATION)
        {
            return;
        }
        let duration = self.last_update.map_or(Duration::ZERO, |t| t.elapsed());
        self.last_update = Some(Instant::now());
        self.ball.update(duration, &self.player1, &self.player2);

        if self.ball.pos.x < 0.0 {
            self.score.1 += 1;
            self.serve();
        } else if self.ball.pos.x > 1.0 {
            self.score.0 += 1;
            self.serve();
        }
    }

    fn serve(&mut self) {
        self.ball.serve();
        self.serve_time = Some(Instant::now());
        self.last_update = None;
    }
}

impl Widget for &Game {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Line::from(format!("Score: {} - {}", self.score.0, self.score.1))
            .centered()
            .render(area, buf);
        self.ball.render(area, buf);
        self.player1.render(area, buf);
        self.player2.render(area, buf);
    }
}
