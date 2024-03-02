use artem::config::{self, TargetType};
use clap::Parser;

use crossterm::terminal::{
    BeginSynchronizedUpdate, ClearType, EndSynchronizedUpdate, EnterAlternateScreen,
    LeaveAlternateScreen, SetSize,
};
use crossterm::{cursor, queue, style, terminal};
use rustic_yellow::{Game, KeyboardEvent, PokemonSpecies};
use std::io::{self, stdout, Write};
use std::num::NonZeroU32;
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::{atomic::AtomicU64, Arc};
use std::thread;
use std::time::Duration;

use crossterm::{
    cursor::position,
    event::{DisableMouseCapture, EnableMouseCapture, Event, EventStream, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode},
};
use futures::{future::FutureExt, select, StreamExt};
use futures_timer::Delay;

use crossterm::event::{poll, read};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Which Pokemon to start with
    #[arg(long, default_value = "Pikachu")]
    starter: String,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    enable_raw_mode().unwrap();
    execute!(std::io::stdout(), EnterAlternateScreen).unwrap();

    env_logger::init();

    let args = Args::parse();
    let starter: PokemonSpecies = args.starter.parse().unwrap();

    let render_delay = Arc::new(AtomicU64::new(16_743));

    let (sender1, receiver1) = mpsc::channel();
    let (sender2, receiver2) = mpsc::sync_channel(1);

    let gamethread = thread::spawn(move || run_game(sender2, receiver1, starter));

    // let periodic = timer_periodic(render_delay.clone());

    execute!(
        std::io::stdout(),
        style::ResetColor,
        terminal::SetSize(
            rustic_yellow::SCREEN_W as u16,
            rustic_yellow::SCREEN_H as u16
        ),
        terminal::Clear(ClearType::All),
        cursor::Hide,
        cursor::MoveTo(0, 0),
    )?;

    let mut reader = EventStream::new();
    let rnd_delay = render_delay.load(std::sync::atomic::Ordering::Relaxed);

    let mut stop = false;
    loop {
        if stop {
            break;
        }
        let mut delay = Delay::new(Duration::from_micros(rnd_delay)).fuse();
        let mut event = reader.next().fuse();

        select! {
            _ = delay => {
                if stop {
                    break;
                }

                match receiver2.try_recv() {
                    Ok(data) => {

                        recalculate_screen(&data);
                    }
                    Err(mpsc::TryRecvError::Empty) => (),
                    Err(..) => {
                        // println!("Remote end has hung-up");
                        stop = true;
                    }
                }
         },

            maybe_event = event => {
                match maybe_event {
                    Some(Ok(event)) => {

                    match event {
                        Event::Key(event) => match event.code {
                            KeyCode::Esc => stop = true,
                            _ => {
                                if let Some(key) = crossterm_to_keyboard(event.code) {
                                    let _ = sender1.send(KeyboardEvent::Down {
                                        key,
                                        shift: event
                                            .modifiers
                                            .contains(crossterm::event::KeyModifiers::SHIFT),
                                    });
                                }
                            }
                        },
                        Event::Mouse(event) => {
                            // println!("{:?}", event);
                        }
                        _ => (),
                    }
                },
                Some(Err(e)) => {}
                None => break,
            }
            }
        }

        if stop {
            break;
        }
    }

    execute!(std::io::stdout(), LeaveAlternateScreen).unwrap();
    disable_raw_mode().unwrap();
    let _ = gamethread.join();

    Ok(())
}

fn crossterm_to_keyboard(key: crossterm::event::KeyCode) -> Option<rustic_yellow::KeyboardKey> {
    match key {
        crossterm::event::KeyCode::Esc => Some(rustic_yellow::KeyboardKey::Escape),
        crossterm::event::KeyCode::Left => Some(rustic_yellow::KeyboardKey::Left),
        crossterm::event::KeyCode::Up => Some(rustic_yellow::KeyboardKey::Up),
        crossterm::event::KeyCode::Right => Some(rustic_yellow::KeyboardKey::Right),
        crossterm::event::KeyCode::Down => Some(rustic_yellow::KeyboardKey::Down),
        crossterm::event::KeyCode::Backspace => Some(rustic_yellow::KeyboardKey::Backspace),
        crossterm::event::KeyCode::Enter => Some(rustic_yellow::KeyboardKey::Return),
        crossterm::event::KeyCode::Char(' ') => Some(rustic_yellow::KeyboardKey::Space),
        crossterm::event::KeyCode::Char('a') => Some(rustic_yellow::KeyboardKey::A),
        crossterm::event::KeyCode::Char('b') => Some(rustic_yellow::KeyboardKey::B),
        crossterm::event::KeyCode::Char('c') => Some(rustic_yellow::KeyboardKey::C),
        crossterm::event::KeyCode::Char('d') => Some(rustic_yellow::KeyboardKey::D),
        crossterm::event::KeyCode::Char('e') => Some(rustic_yellow::KeyboardKey::E),
        crossterm::event::KeyCode::Char('f') => Some(rustic_yellow::KeyboardKey::F),
        crossterm::event::KeyCode::Char('g') => Some(rustic_yellow::KeyboardKey::G),
        crossterm::event::KeyCode::Char('h') => Some(rustic_yellow::KeyboardKey::H),
        crossterm::event::KeyCode::Char('i') => Some(rustic_yellow::KeyboardKey::I),
        crossterm::event::KeyCode::Char('j') => Some(rustic_yellow::KeyboardKey::J),
        crossterm::event::KeyCode::Char('k') => Some(rustic_yellow::KeyboardKey::K),
        crossterm::event::KeyCode::Char('l') => Some(rustic_yellow::KeyboardKey::L),
        crossterm::event::KeyCode::Char('m') => Some(rustic_yellow::KeyboardKey::M),
        crossterm::event::KeyCode::Char('n') => Some(rustic_yellow::KeyboardKey::N),
        crossterm::event::KeyCode::Char('o') => Some(rustic_yellow::KeyboardKey::O),
        crossterm::event::KeyCode::Char('p') => Some(rustic_yellow::KeyboardKey::P),
        crossterm::event::KeyCode::Char('q') => Some(rustic_yellow::KeyboardKey::Q),
        crossterm::event::KeyCode::Char('r') => Some(rustic_yellow::KeyboardKey::R),
        crossterm::event::KeyCode::Char('s') => Some(rustic_yellow::KeyboardKey::S),
        crossterm::event::KeyCode::Char('t') => Some(rustic_yellow::KeyboardKey::T),
        crossterm::event::KeyCode::Char('u') => Some(rustic_yellow::KeyboardKey::U),
        crossterm::event::KeyCode::Char('v') => Some(rustic_yellow::KeyboardKey::V),
        crossterm::event::KeyCode::Char('w') => Some(rustic_yellow::KeyboardKey::W),
        crossterm::event::KeyCode::Char('x') => Some(rustic_yellow::KeyboardKey::X),
        crossterm::event::KeyCode::Char('y') => Some(rustic_yellow::KeyboardKey::Y),
        crossterm::event::KeyCode::Char('z') => Some(rustic_yellow::KeyboardKey::Z),

        _ => None,
    }
}

fn recalculate_screen(datavec: &[u8]) {
    execute!(io::stdout(), BeginSynchronizedUpdate).unwrap();

    queue!(
        stdout(),
        // terminal::Clear(ClearType::All),
        // terminal::SetSize(
        //     rustic_yellow::SCREEN_W as u16,
        //     rustic_yellow::SCREEN_H as u16
        // ),
        cursor::MoveTo(0, 0),
    )
    .unwrap();

    let rawimage2d = glium::texture::RawImage2d {
        data: std::borrow::Cow::Borrowed(datavec),
        width: rustic_yellow::SCREEN_W as u32,
        height: rustic_yellow::SCREEN_H as u32,
        format: glium::texture::ClientFormat::U8U8U8,
    };

    let raw_data = rawimage2d.data.into_owned();

    // Step 2: Convert Vec<u8> to ImageBuffer
    let img_buffer =
        image::ImageBuffer::from_raw(rawimage2d.width, rawimage2d.height, raw_data).unwrap();

    // Step 3: Convert ImageBuffer to DynamicImage
    let dynamic_image = image::DynamicImage::ImageRgb8(img_buffer);

    // Now you can convert the DynamicImage to ASCII
    let mut config_builder = artem::config::ConfigBuilder::new();
    config_builder.target(TargetType::Shell(true, true));

    config_builder.dimension(config::ResizingDimension::Width);

    // let target_size = if width || height {
    //     if height {
    //         config_builder.dimension(config::ResizingDimension::Height);
    //     }
    //     terminal_size(height)
    // } else {
    //     //use given input size
    //     log::trace!("Using user input size as target size");
    //     0
    // }
    // .max(20); //min should be 20 to ensure a somewhat visible picture

    // log::debug!("Target Size: {target_size}");
    let wsize = crossterm::terminal::window_size().unwrap();
    let w_width = wsize.width as u32;
    let cols = wsize.columns as u32;
    config_builder.target_size(NonZeroU32::new(cols).unwrap()); //safe to unwrap, since it is clamped before
    let target_img_size = 1024; //terminal_size(width).max(2048);

    //best ratio between height and width is 0.43
    let guess_scale = |target_size: u32| -> f32 {
        let target_size = target_size as f32;
        let scale = (target_size * 0.43) / (rustic_yellow::SCREEN_W as f32);
        scale.clamp(0.1, 5.0)
    };

    let matches_scales = |target_size: u32, scale: f32| -> bool {
        let target_size = target_size as f32;
        let scale = (target_size * 0.43) / (rustic_yellow::SCREEN_W as f32);
        (scale - 0.1..=scale + 0.1).contains(&scale)
    };

    let scale = 0.3; //guess_scale(cols);
                     // eprintln!("Scale: {}", scale);
    config_builder.scale(scale);

    // config_builder.center_x(true);
    // config_builder.center_y(true);
    config_builder.hysteresis(true);
    config_builder.characters(" .:-=+*#%@".to_string());

    let config = config_builder.build();

    // let scaled_image: image::DynamicImage = dynamic_image.resize(
    //     (rustic_yellow::SCREEN_W as f32 * scale) as u32,
    //     (rustic_yellow::SCREEN_H as f32 * scale) as u32,
    //     image::imageops::FilterType::Nearest,
    // );
    // Write the scaled image to a file
    // scaled_image.save("scaled_image.png").unwrap();

    let ascii_art = artem::convert(dynamic_image, &config);

    // texture.write(
    //     glium::Rect {
    //         left: 0,
    //         bottom: 0,
    //         width: rustic_yellow::SCREEN_W as u32,
    //         height: rustic_yellow::SCREEN_H as u32,
    //     },
    //     rawimage2d,
    // );

    // // We use a custom BlitTarget to transform OpenGL coordinates to row-column coordinates
    // let target = display.draw();
    // let (target_w, target_h) = target.get_dimensions();
    // texture.as_surface().blit_whole_color_to(
    //     &target,
    //     &glium::BlitTarget {
    //         left: 0,
    //         bottom: target_h,
    //         width: target_w as i32,
    //         height: -(target_h as i32),
    //     },
    //     glium::uniforms::MagnifySamplerFilter::Nearest,
    // );
    // target.finish().unwrap();
    for line in ascii_art.lines() {
        queue!(io::stdout(), style::Print(line)).unwrap();
    }

    execute!(io::stdout(), EndSynchronizedUpdate).unwrap();
    io::stdout().flush().unwrap();
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

// fn set_window_size(window: &glium::glutin::window::Window) {
//     use glium::glutin::dpi::{LogicalSize, PhysicalSize};

//     let dpi = window.scale_factor();

//     let physical_size = PhysicalSize::<u32>::from((
//         rustic_yellow::SCREEN_W as u32,
//         rustic_yellow::SCREEN_H as u32,
//     ));
//     let logical_size = LogicalSize::<u32>::from_physical(physical_size, dpi);

//     window.set_inner_size(logical_size);
// }
