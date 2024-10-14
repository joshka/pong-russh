use ratatui::{layout::Position, prelude::*};

/// A coordinate point in the range [0.0, 1.0] representing a position in the game world.
///
/// (0.5, 0.5) is the center of the screen.
#[derive(Debug, Default, Clone, Copy)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const CENTER: Self = Self::new(0.5, 0.5);

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Convert a point in the game world to a point on the screen.
    ///
    /// Screen coordinates are in the range [0, width] and [0, height].
    pub fn to_screen(self, area: Rect) -> Position {
        // ensure that the resulting position is within the screen bounds
        let max_width = (area.width.saturating_sub(1)) as f32;
        let max_height = (area.height.saturating_sub(1)) as f32;
        Position {
            x: (self.x * max_width) as u16 + area.x,
            y: (self.y * max_height) as u16 + area.y,
        }
    }
}

/// A velocity vector in the range [-1.0, 1.0] representing a direction and speed.
///
/// The x component is the horizontal velocity, with negative values moving left and positive
/// values moving right. The y component is the vertical velocity, with negative values moving
/// up and positive values moving down.
///
/// The unit of the velocity is the fraction of the screen width or height moved per second.
/// Velocity is independent of the screen size, so the same velocity will move the same distance
/// regardless of the screen size.
///
/// There are only a few valid values for the velocity components to align with the original
/// Pong game. (See <https://www.pong-story.com/LAWN_TENNIS.pdf> for more details.) These have
/// been scaled to coordinates in the range [-1.0, 1.0] to make them independent of the screen
/// size and rounded slightly to make them easier to work with.
///
/// - Vertical velocity: -0.69, -0.46, -0.23, 0.0, 0.23, 0.46, 0.69
/// - Horizontal velocity: -0.53, -0.39. -0.26, 0.26, 0.39, 0.53
#[derive(Debug, Default)]
pub struct Velocity {
    pub x: f32,
    pub y: f32,
}

impl Velocity {
    // pub const VALID_X: [f32; 6] = [-0.53, -0.39, -0.26, 0.26, 0.39, 0.53];
    pub const VALID_Y: [f32; 7] = [-0.69, -0.46, -0.23, 0.0, 0.23, 0.46, 0.69];

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}
