use async_trait::async_trait;
use crossterm::event::KeyCode;
use ratatui::{backend::CrosstermBackend, layout::Rect, Terminal};
use russh::{client, server::*, Channel, ChannelId};
use russh_keys::key::{KeyPair, PublicKey};
use std::{
    collections::HashMap,
    fs::File,
    future::IntoFuture,
    io::{Read, Write},
    pin::Pin,
    sync::Arc,
    thread::{self, spawn},
    time::Instant,
};
use tokio::sync::Mutex;

use crate::server::gb::{global_sixel, start_gb};

use super::{
    game::Game,
    types::{AppResult, TerminalHandle},
};

const GAME_NAME: &str = "ssHattrick";
const TERMINAL_WIDTH: u16 = 40;
const TERMINAL_HEIGHT: u16 = 30;
const INACTIVITY_TIMEOUT: u64 = 100;

pub fn save_keys(signing_key: &ed25519_dalek::SigningKey) -> AppResult<()> {
    let file = File::create::<&str>("./keys".into())?;
    assert!(file.metadata()?.is_file());
    let mut buffer = std::io::BufWriter::new(file);
    buffer.write(&signing_key.to_bytes())?;
    Ok(())
}

pub fn load_keys() -> AppResult<ed25519_dalek::SigningKey> {
    let file = File::open::<&str>("./keys".into())?;
    let mut buffer = std::io::BufReader::new(file);
    let mut buf: [u8; 32] = [0; 32];
    buffer.read(&mut buf)?;
    Ok(ed25519_dalek::SigningKey::from_bytes(&buf))
}

fn convert_data_to_key_code(data: &[u8]) -> crossterm::event::KeyCode {
    match data {
        b"\x1b[A" => crossterm::event::KeyCode::Up,
        b"\x1b[B" => crossterm::event::KeyCode::Down,
        b"\x1b[C" => crossterm::event::KeyCode::Right,
        b"\x1b[D" => crossterm::event::KeyCode::Left,
        // ctrl+c is also converted to esc
        b"\x03" => crossterm::event::KeyCode::Esc,
        b"\x1b" => crossterm::event::KeyCode::Esc,
        b"\x0d" => crossterm::event::KeyCode::Enter,
        b"\x7f" => crossterm::event::KeyCode::Backspace,
        b"\x1b[3~" => crossterm::event::KeyCode::Delete,
        b"\x09" => crossterm::event::KeyCode::Tab,
        _ => crossterm::event::KeyCode::Char(data[0] as char),
    }
}

#[derive(Clone)]
pub struct GameServer {
    clients: Arc<Mutex<HashMap<usize, TerminalHandle>>>,
    clients_to_game: Arc<Mutex<HashMap<usize, uuid::Uuid>>>,
    client_id: usize,
    game: Arc<Mutex<Game>>,
    pending_client: Arc<Mutex<Option<(usize, Instant)>>>,
}

impl GameServer {
    pub fn new() -> Self {
        log::info!("Creating new server");

        Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
            game: Arc::new(Mutex::new(Game::new())),
            clients_to_game: Arc::new(Mutex::new(HashMap::new())),
            client_id: 0,

            pending_client: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn run(
        &mut self,
        port: u16,
    ) -> Pin<Box<dyn futures::Future<Output = Result<(), std::io::Error>> + std::marker::Send + '_>>
    {
        log::info!("Starting game loop");
        // TODO (maybe): spawn a new loop for each game. Not sure it's a good idea actually
        // To close the loop, check if both are disconnected or the game is over.
        let game = self.game.clone();
        let clients = self.clients.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;

                let sixel = global_sixel().await.read().await.get_sixel();

                game.lock().await.set_sixel(sixel);

                game.lock().await.update().unwrap_or_else(|e| {
                    log::error!("Failed to update game: {:?}", e);
                });

                game.lock().await.draw().unwrap_or_else(|e| {
                    log::error!("Failed to draw game: {:?}", e);
                });
            }
        });

        let signing_key = load_keys().unwrap_or_else(|_| {
            let key_pair = russh_keys::key::KeyPair::generate_ed25519().unwrap();
            let signing_key = match key_pair {
                KeyPair::Ed25519(key) => key,
            };
            let _ = save_keys(&signing_key);
            signing_key
        });

        let key_pair = KeyPair::Ed25519(signing_key);

        let config = Config {
            inactivity_timeout: Some(std::time::Duration::from_secs(INACTIVITY_TIMEOUT)),
            auth_rejection_time: std::time::Duration::from_secs(3),
            auth_rejection_time_initial: Some(std::time::Duration::from_secs(0)),
            keys: vec![key_pair],
            ..Default::default()
        };

        log::info!("Starting server on port {}", port);

        let s = self.run_on_address(Arc::new(config), ("0.0.0.0", port));

        return s.into_future();
    }

    async fn close_session(
        &mut self,
        session: &mut Session,
        channel: ChannelId,
    ) -> Result<(), anyhow::Error> {
        self.clients.lock().await.remove(&self.client_id);
        self.clients_to_game.lock().await.remove(&self.client_id);

        session.eof(channel);
        session.disconnect(russh::Disconnect::ByApplication, "Quit", "");
        session.close(channel);

        let mut pending_client = self.pending_client.lock().await;
        if pending_client.is_some() && pending_client.unwrap().0 == self.client_id {
            *pending_client = None;
            log::info!("Removed player from pending list");
        }
        Ok(())
    }
}

impl Server for GameServer {
    type Handler = Self;
    fn new_client(&mut self, _: Option<std::net::SocketAddr>) -> Self {
        let s = self.clone();
        self.client_id += 1;
        s
    }
}

#[async_trait]
impl Handler for GameServer {
    type Error = anyhow::Error;

    async fn channel_close(
        &mut self,
        channel: ChannelId,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        self.close_session(session, channel).await
    }

    async fn channel_eof(
        &mut self,
        channel: ChannelId,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        self.close_session(session, channel).await
    }

    async fn channel_open_session(
        &mut self,
        channel: Channel<Msg>,
        session: &mut Session,
    ) -> Result<bool, Self::Error> {
        {
            log::info!("Opening new session");
            let mut terminal_handle = TerminalHandle::new(session.handle(), channel.id());
            let client_id = self.client_id;

            let mut clients = self.clients.lock().await;

            clients.insert(self.client_id, terminal_handle.clone());

            let backend = CrosstermBackend::new(terminal_handle.clone());
            let terminal = Terminal::with_options(
                backend,
                ratatui::TerminalOptions {
                    viewport: ratatui::Viewport::Fixed(Rect {
                        x: 0,
                        y: 0,
                        width: TERMINAL_WIDTH,
                        height: TERMINAL_HEIGHT,
                    }),
                },
            )?;

            let mut game = self.game.lock().await;
            game.add_client_terminal(client_id, terminal);
            self.clients_to_game.lock().await.insert(client_id, game.id);
            self.clients_to_game
                .lock()
                .await
                .insert(self.client_id, game.id);
            log::info!("Added player to new game. Game id: {}", game.id);
        }

        Ok(true)
    }

    async fn auth_none(&mut self, _: &str) -> Result<Auth, Self::Error> {
        Ok(Auth::Accept)
    }

    async fn auth_password(&mut self, _: &str, _: &str) -> Result<Auth, Self::Error> {
        Ok(Auth::Accept)
    }

    async fn auth_publickey(&mut self, _: &str, _: &PublicKey) -> Result<Auth, Self::Error> {
        Ok(Auth::Accept)
    }

    async fn auth_keyboard_interactive(
        &mut self,
        _: &str,
        _: &str,
        _: Option<Response<'async_trait>>,
    ) -> Result<Auth, Self::Error> {
        Ok(Auth::Accept)
    }

    async fn window_change_request(
        &mut self,
        _: ChannelId,
        _: u32,
        _: u32,
        _: u32,
        _: u32,
        _: &mut Session,
    ) -> Result<(), Self::Error> {
        if let Some(game_id) = &mut self.clients_to_game.lock().await.get_mut(&self.client_id) {
            if let mut game = self.game.lock().await {
                game.clear_client(self.client_id);
            }
        }
        Ok(())
    }

    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        let key_code = convert_data_to_key_code(data);

        if key_code == KeyCode::Esc {
            self.close_session(session, channel)
                .await
                .unwrap_or_else(|e| log::error!("Failed to close session: {:?}", e));
            return Ok(());
        }

        let pending_client = self.pending_client.lock().await;
        if pending_client.is_some() && pending_client.unwrap().0 == self.client_id {
            return Ok(());
        }

        if let Some(game_id) = &mut self.clients_to_game.lock().await.get_mut(&self.client_id) {
            let mut game = self.game.lock().await;
            game.handle_input(self.client_id, key_code);
            return Ok(());
        }

        self.clients.lock().await.remove(&self.client_id);
        self.clients_to_game.lock().await.remove(&self.client_id);
        self.game.lock().await.disconnect(self.client_id);
        session.eof(channel);
        session.disconnect(russh::Disconnect::ByApplication, "Quit", "");
        session.close(channel);

        Ok(())
    }
}
