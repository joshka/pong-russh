use scopeguard::defer;

use game::Game;
mod ball;
mod errors;
mod events;
mod game;
mod paddle;
mod physics;
mod tui;

fn main() -> color_eyre::Result<()> {
    errors::install_hooks()?;
    let mut tui = tui::init()?;
    defer! { tui::restore().unwrap() }
    Game::new().run(&mut tui)?;
    Ok(())
}
