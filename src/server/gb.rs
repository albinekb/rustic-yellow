use futures::lock::Mutex;
use human_panic::setup_panic;

use artem::config::{self, TargetType};
use clap::Parser;

use async_once_cell::OnceCell;
use rustic_yellow::{Game, KeyboardEvent, PokemonSpecies};
use tokio::sync::RwLock;

use std::io::{self, stdout};

use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::{atomic::AtomicU64, Arc};
use std::time::Duration;
use std::{thread, vec};

use termwiz::image::{ImageCell, ImageData, TextureCoordinate};
use termwiz::input::{InputEvent, KeyCode, KeyEvent};
use termwiz::surface::{Image, Line};
use termwiz::terminal::{self, new_terminal, UnixTerminal};
use termwiz::{
    caps::Capabilities,
    cell::{AttributeChange, Blink, CellAttributes, Intensity, Underline},
    color::{AnsiColor, ColorAttribute, ColorSpec, LinearRgba, RgbColor, SrgbaTuple},
    surface::{Change, CursorVisibility, Position, SequenceNo, Surface},
    terminal::{buffered::BufferedTerminal, ScreenSize, SystemTerminal, Terminal},
};

use crate::sixel::CachedSixel;

pub async fn global_sixel() -> &'static RwLock<CachedSixel> {
    static INSTANCE: OnceCell<RwLock<CachedSixel>> = OnceCell::new();
    INSTANCE
        .get_or_init(async {
            let m = CachedSixel::new(rustic_yellow::SCREEN_W, rustic_yellow::SCREEN_H);

            RwLock::new(m)
        })
        .await
}

pub async fn start_gb() {
    let render_delay = Arc::new(AtomicU64::new(16_743));
    print!("Starting gb...");
    let (sender1, receiver1) = mpsc::channel();
    let (sender2, receiver2) = mpsc::sync_channel(1);
    let starter = PokemonSpecies::Charmander;

    let gamethread = thread::spawn(move || run_game(sender2, receiver1, starter));

    let rnd_delay = render_delay.load(std::sync::atomic::Ordering::Relaxed);

    let mut stop = false;
    // let mut input_stream  = buffered_terminal.terminal().poll_input(None);
    // let surface = termwiz::surface::Surface::new(rustic_yellow::SCREEN_W, rustic_yellow::SCREEN_H);

    // let mut cached_sixel = CachedSixel::new(rustic_yellow::SCREEN_W, rustic_yellow::SCREEN_H);
    let timer = timer_periodic(render_delay.clone());

    loop {
        if stop {
            break;
        }

        timer.recv().unwrap();
        // let mut delay = Delay::new(Duration::from_micros(rnd_delay)).fuse();

        match receiver2.try_recv() {
            Ok(data) => {
                // println!("Received data");
                global_sixel().await.write().await.tick(&data);
            }
            Err(mpsc::TryRecvError::Empty) => (),
            Err(..) => {
                println!("Remote end has hung-up");
                stop = true;
                break;
            }
        }
    }

    let _ = gamethread.join().unwrap();
}

fn run_game(
    sender: SyncSender<Vec<u8>>,
    receiver: Receiver<KeyboardEvent>,
    starter: PokemonSpecies,
) {
    Game::new(sender, receiver, starter).boot();
}

fn timer_periodic(delay: Arc<AtomicU64>) -> Receiver<()> {
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || loop {
        let micros = delay.load(std::sync::atomic::Ordering::Relaxed);
        std::thread::sleep(std::time::Duration::from_micros(micros));
        if tx.send(()).is_err() {
            break;
        }
    });
    rx
}
