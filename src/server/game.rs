use crossterm::event::KeyCode;
use image::{Rgba, RgbaImage};
use once_cell::sync::Lazy;
use rand::Rng;
use ratatui::{
    layout::{Constraint, Layout, Margin, Position, Rect},
    style::Color,
    text::Line,
    widgets::{Paragraph, Widget},
    Frame,
};
use ratatui_image::{protocol::sixel::Sixel, Image};
use std::{collections::HashMap, time::Instant};

use crate::sixel::render_sixel;

use super::{
    gb::global_sixel,
    types::{AppResult, SshTerminal},
};

const MINIMUM_DELTATIME_MILLISECONDS: f32 = 18.0;
const GAME_DURATION_MILLISECONDS: u128 = 90 * 1000;
const STARTING_DELAY_MILLISECONDS: u128 = 3000;
const AFTER_GOAL_DELAY_MILLISECONDS: u128 = 2000;
const ENDING_DELAY_MILLISECONDS: u128 = 1000;

const MIN_X: f32 = 3.0;
const MAX_X: f32 = 157.0;
const MIN_Y: f32 = 3.0;
const MAX_Y: f32 = 83.0;

#[derive(Clone, Copy, PartialEq)]
enum GameState {
    // TODO: add character selection with different stats
    Starting { time: Instant },
    Running,

    Ending { time: Instant },
}

#[derive(Clone)]
pub struct Player {
    shooting_direction: Option<(f32, f32)>,
    shooting_counter: f32,
    after_shooting_counter: f32,
    after_got_stolen_counter: f32,
}

impl Player {
    pub fn new() -> Self {
        Self {
            shooting_direction: None,
            shooting_counter: 0.0,
            after_shooting_counter: 0.0,
            after_got_stolen_counter: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.shooting_direction = None;
        self.shooting_counter = 0.0;
        self.after_shooting_counter = 0.0;
    }
}

#[derive(Clone)]
pub struct Client {
    id: usize,
    terminal: SshTerminal,
    is_connected: bool,
}

impl Client {
    pub fn new(id: usize, terminal: SshTerminal) -> Self {
        Self {
            id,
            terminal,
            is_connected: true,
        }
    }

    pub fn clear(&mut self) -> AppResult<()> {
        if self.is_connected {
            self.terminal.draw(|f| {
                let mut lines = vec![];
                for _ in 0..f.size().height {
                    lines.push(Line::from(" ".repeat(f.size().width.into())));
                }
                let clear = Paragraph::new(lines).style(Color::White);
                f.render_widget(clear, f.size());
            })?;
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct Game {
    clients: HashMap<usize, Client>,
    pub id: uuid::Uuid,
    timer: u128,
    last_tick: Instant,
    fps: f32,
    state: GameState,
    pub sixel: String,
}

impl Game {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
            id: uuid::Uuid::new_v4(),
            timer: 0,
            last_tick: Instant::now(),
            fps: 0.0,
            state: GameState::Starting {
                time: Instant::now(),
            },
            sixel: "not yet".to_string(),
        }
    }

    pub fn clear_client(&mut self, client_id: usize) {
        if let Some(client) = self.clients.get_mut(&client_id) {
            client.clear().unwrap_or_else(|e| {
                log::error!("Failed to clear client {}: {}", client_id, e);
            });
        }
    }
    pub fn set_sixel(&mut self, sixel: String) {
        self.sixel = sixel;
    }

    pub fn add_client_terminal(&mut self, client_id: usize, terminal: SshTerminal) {
        self.clients
            .insert(client_id, Client::new(client_id, terminal));
    }

    fn reset(&mut self) {
        self.state = GameState::Starting {
            time: Instant::now(),
        };
    }

    fn close(&mut self) {
        for client in self.clients.values_mut() {
            client.is_connected = false;
        }
    }

    pub fn disconnect(&mut self, client_id: usize) {
        if let Some(client) = self.clients.get_mut(&client_id) {
            client.is_connected = false;
        }
    }

    pub fn is_running(&self) -> bool {
        matches!(self.state, GameState::Running)
    }

    pub fn client_ids(&self) -> Vec<usize> {
        self.clients.keys().copied().collect()
    }

    pub fn handle_input(&mut self, client_id: usize, key_code: KeyCode) {
        if key_code == KeyCode::Esc {
            self.disconnect(client_id);
            return;
        }
        println!("Received key code: {:?}", key_code);
    }

    pub fn update(&mut self) -> AppResult<()> {
        let now = Instant::now();
        let deltatime = now.duration_since(self.last_tick).as_millis() as f32;
        if deltatime < MINIMUM_DELTATIME_MILLISECONDS {
            return Ok(());
        }

        match self.state {
            GameState::Starting { time } => {
                if now.duration_since(time).as_millis() >= STARTING_DELAY_MILLISECONDS {
                    self.state = GameState::Running;
                }
            }
            GameState::Running => {
                self.update_running(deltatime)?;
                self.timer += deltatime as u128;
                if self.timer > GAME_DURATION_MILLISECONDS {
                    self.state = GameState::Ending {
                        time: Instant::now(),
                    };
                }
            }
            GameState::Ending { time } => {
                if now.duration_since(time).as_millis() >= ENDING_DELAY_MILLISECONDS {
                    self.close();
                }
            }
        }
        self.fps = 1000.0 / deltatime;
        self.last_tick = now;

        Ok(())
    }

    fn update_running(&mut self, deltatime: f32) -> AppResult<()> {
        Ok(())
    }

    pub fn draw(&mut self) -> AppResult<()> {
        let timer = if self.timer > GAME_DURATION_MILLISECONDS {
            0
        } else {
            (GAME_DURATION_MILLISECONDS - self.timer) / 1000
        };

        for client in self.clients.values_mut() {
            if !client.is_connected {
                continue;
            }
            let _ = client.terminal.draw(|f| {
                let _ = Self::render(f, timer, self.fps, self.state, Some(self.sixel.clone()))
                    .unwrap_or_else(|e| {
                        log::error!("Failed to draw game: {}", e);
                    });
            });
        }

        Ok(())
    }

    fn render(
        frame: &mut Frame,
        timer: u128,
        fps: f32,
        state: GameState,
        sixel: Option<String>,
    ) -> AppResult<()> {
        let info_rect = Rect::new(frame.size().width - 20, frame.size().height - 1, 10, 1);
        let sixel_area = Rect::new(
            0,
            0,
            frame.size().width as u16 - 1,
            frame.size().height as u16 - 1,
        );

        if let Some(sixel) = sixel {
            let sixel_rect = Rect::new(0, 0, 40, 30);
            render_sixel(sixel_rect, &sixel, sixel_area, frame.buffer_mut(), true);
            // let ar = frame.buffer_mut().get_mut(0, 0);
            // ar.set_symbol(&sixel);
            // ar.set_skip(true);
        } else {
            let rect = Rect::new(0, 0, 100, 100);
            let paragraph = Paragraph::new("No sixel").style(Color::White);
            frame.render_widget(paragraph, rect);
        }

        Ok(())
    }

    pub fn connections_state(&self) -> Vec<bool> {
        self.clients.values().map(|c| c.is_connected).collect()
    }
}
