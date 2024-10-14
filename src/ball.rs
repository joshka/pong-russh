use std::time::Duration;

use ratatui::prelude::*;

use crate::{
    paddle::Paddle,
    physics::{Point, Velocity},
};

#[derive(Debug)]
pub struct Ball {
    pub pos: Point,
    pub vel: Velocity,
}

impl Default for Ball {
    fn default() -> Self {
        Self::new()
    }
}

impl Ball {
    const DEFAULT_INITIAL_VELOCITY: Velocity = Velocity::new(0.26, -0.23);

    /// Crete a new ball at the center of the screen with the default initial velocity.
    pub const fn new() -> Self {
        Self {
            pos: Point::CENTER,
            vel: Self::DEFAULT_INITIAL_VELOCITY,
        }
    }

    /// Serve the ball from the center of the screen with the existing velocity.
    pub fn serve(&mut self) {
        self.pos = Point::CENTER;
    }

    /// Move the ball by its current velocity.
    ///
    /// The ball will bounce off the top and bottom edges of the screen, reversing the vertical
    /// velocity component.
    ///
    /// The ball will move by the velocity components scaled by the time since the last update.
    /// This ensures that the ball moves at the same speed regardless of the screen size or
    /// refresh rate.
    pub fn update(&mut self, duration: Duration, player1: &Paddle, player2: &Paddle) {
        let dt = duration.as_secs_f32();
        self.pos.x += self.vel.x * dt;
        self.pos.y += self.vel.y * dt;

        // bounce off the top and bottom edges
        if self.pos.y < 0.0 {
            self.pos.y = -self.pos.y;
            self.vel.y = -self.vel.y;
        } else if self.pos.y > 1.0 {
            self.pos.y = 2.0 - self.pos.y;
            self.vel.y = -self.vel.y;
        }

        // bounce off the paddles
        // todo: change direction based on where the ball hits the paddle
        // todo: increase horizontal speed based on number of hits
        // todo: calculate the intersection point of the ball and the paddle rather than just
        // checking if the ball is within the paddle's height
        if self.pos.x < 0.0 {
            if (player1.pos.y - Paddle::HEIGHT / 2.0 < self.pos.y)
                && (self.pos.y < player1.pos.y + Paddle::HEIGHT / 2.0)
            {
                self.pos.x = -self.pos.x;
                self.vel.x = -self.vel.x;

                let distance = self.pos.y - player1.pos.y;
                let angle = distance / (Paddle::HEIGHT / 2.0);
                // map onto the range of valid vertical velocities
                let index = ((angle * 3.0).round() as i32 + 3) as usize;
                self.vel.y = Velocity::VALID_Y[index];
            }
        } else if self.pos.x > 1.0
            && (player2.pos.y - Paddle::HEIGHT / 2.0 < self.pos.y)
            && (self.pos.y < player2.pos.y + Paddle::HEIGHT / 2.0)
        {
            self.pos.x = 2.0 - self.pos.x;
            self.vel.x = -self.vel.x;

            let distance = self.pos.y - player2.pos.y;
            let angle = distance / (Paddle::HEIGHT / 2.0);
            // map onto the range of valid vertical velocities
            let index = ((angle * 3.0).round() as i32 + 3) as usize;
            self.vel.y = Velocity::VALID_Y[index];
        }
    }
}

impl Widget for &Ball {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if !(0.0..=1.0).contains(&self.pos.x) || !(0.0..=1.0).contains(&self.pos.y) {
            return;
        }
        // use block characters that represent 1/8th of a cell to draw the paddles
        const TOP_BARS: [&str; 9] = ["█", "▇", "▆", "▅", "▄", "▃", "▂", "▁", " "];
        const BOTTOM_BARS: [&str; 9] = [" ", "▔", "🮂", "🮃", "▀", "🮄", "🮅", "🮆", "█"];
        let _x = self.pos.x * (area.width.saturating_sub(1)) as f32;
        let y = self.pos.y * (area.height.saturating_sub(1)) as f32;
        // draw the top character of the paddle by taking the fractional part of the top position
        let top_char = TOP_BARS[(y.fract() * 8.0).round() as usize];
        let bottom_char = BOTTOM_BARS[(y.fract() * 8.0).round() as usize];
        let pos = self.pos.to_screen(area);
        let ball_area = Rect::new(pos.x, pos.y, 1, 1);
        Span::raw(top_char).render(ball_area, buf);
        let ball_area = Rect::new(pos.x, pos.y + 1, 1, 1);
        Span::raw(bottom_char).render(ball_area, buf);

        // let debug = format!("{:3}, {:3}, {:5.2}, {:5.2}", pos.x, pos.y, x, y);
        // let last_row = area.rows().last().unwrap();
        // Line::from(debug).centered().render(last_row, buf);
    }
}
