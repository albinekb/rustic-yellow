#![feature(async_closure)]
mod server;
pub mod sixel;

use std::{future::IntoFuture, thread};

use clap::{ArgAction, Parser};
use server::{gb::start_gb, server::GameServer};
use tokio::{select, spawn, task::spawn_blocking};

#[derive(Parser, Debug)]
#[clap(name="ssHattrick", about = "Hockey in the terminal via ssh", author, version, long_about = None)]
struct Args {
    #[clap(long, short = 'p', action=ArgAction::Set, help = "Set port to listen on")]
    port: Option<u16>,
}

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let gb = start_gb();
    let mut game_server = GameServer::new();

    let port = Args::parse().port.unwrap_or(2020);

    // Start the Game Boy emulator in a separate asynchronous task
    let gb_thread = spawn(async move {
        start_gb().await; // Make sure start_gb is an async function
        log::error!("Gameboy thread exited");
    });

    // Start the server in another asynchronous task
    let server_thread = spawn(async move {
        let mut game_server = GameServer::new();

        game_server.run(port).await.await;
    });

    // Wait for both threads to finish

    select! {
        _ = gb_thread => {},
        _ = server_thread => {},
    }
}
