use crate::physics::Point;
use ratatui::prelude::*;

/// Represents a paddle in the game.
///
/// The x coordinate of the paddle is fixed, so it only moves up and down.
#[derive(Debug, Default)]
pub struct Paddle {
    pub pos: Point,
}

impl Paddle {
    // const WIDTH: f32 = 0.01;
    pub const HEIGHT: f32 = 0.15;
    const MOVE_DELTA: f32 = 0.025;

    pub const fn new(x: f32, y: f32) -> Self {
        Self {
            pos: Point { x, y },
        }
    }

    /// Move the paddle up by a small amount
    pub fn move_up(&mut self) {
        self.pos.y = f32::max(self.pos.y - Self::MOVE_DELTA, Self::HEIGHT / 2.0);
    }

    /// Move the paddle down by a small amount
    pub fn move_down(&mut self) {
        self.pos.y = f32::min(self.pos.y + Self::MOVE_DELTA, 1.0 - Self::HEIGHT / 2.0);
    }
}

impl Widget for &Paddle {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // use block characters that represent 1/8th of a cell to draw the paddles
        const TOP_BARS: [&str; 9] = ["█", "▇", "▆", "▅", "▄", "▃", "▂", "▁", " "];
        const BOTTOM_BARS: [&str; 9] = [" ", "▔", "🮂", "🮃", "▀", "🮄", "🮅", "🮆", "█"];
        let x = (self.pos.x * (area.width.saturating_sub(1)) as f32) as u16 + area.x;
        let top = (self.pos.y - Paddle::HEIGHT / 2.0) * area.height as f32;
        let bottom = (self.pos.y + Paddle::HEIGHT / 2.0) * area.height as f32;
        // draw the top character of the paddle by taking the fractional part of the top position
        let index = (top.fract() * 8.0).round() as usize;
        let top_char = TOP_BARS[index];
        let top = top as u16 + area.y;
        buf.set_string(x, top, top_char, Style::default());
        // draw the bottom character of the paddle by taking the fractional part of the bottom position
        let index = (bottom.fract() * 8.0).round() as usize;
        let bottom_char = BOTTOM_BARS[index];
        let bottom = bottom as u16 + area.y;
        buf.set_string(x, bottom, bottom_char, Style::default());

        // fill in the middle of the paddle with block characters
        for y in top + 1..bottom {
            buf.set_string(x, y, "█", Style::default());
        }
    }
}
