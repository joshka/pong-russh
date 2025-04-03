use color_eyre::Result;
use tracing::{debug, level_filters::LevelFilter};
use tracing_subscriber::EnvFilter;

mod backend;
mod ball;
mod game;
mod paddle;
mod physics;
mod server;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    init_tracing()?;
    let mut server = server::AppServer::new()?;
    server.run().await?;
    Ok(())
}

fn init_tracing() -> Result<()> {
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
