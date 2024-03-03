use human_panic::setup_panic;

use artem::config::{self, TargetType};
use clap::Parser;

use rustic_yellow::{Game, KeyboardEvent, PokemonSpecies};
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

mod sixel;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Which Pokemon to start with
    #[arg(long, default_value = "Pikachu")]
    starter: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    setup_panic!();
    initialize_panic_handler();
    println!("Starting");
    env_logger::init();
    let caps = Capabilities::new_from_env()?;
    // println!("Capabilities: {:?}", caps);
    let mut terminal = new_terminal(caps)?;
    terminal.set_raw_mode()?;
    terminal.enter_alternate_screen()?;
    let mut buffered_terminal = BufferedTerminal::new(terminal)?;
    // println!("Starting game");

    buffered_terminal.add_change(Change::ClearScreen(Default::default()));
    buffered_terminal.add_change(Change::CursorVisibility(CursorVisibility::Hidden));
    buffered_terminal.add_change(Change::CursorPosition {
        x: Position::Absolute(0),
        y: Position::Absolute(0),
    });
    buffered_terminal.flush()?;

    // println!("Starting game");
    // buffered_terminal.terminal().set_raw_mode()?;
    // buffered_terminal.terminal().enter_alternate_screen()?;

    let args = Args::parse();
    let starter: PokemonSpecies = args.starter.parse().unwrap();

    let render_delay = Arc::new(AtomicU64::new(16_743));

    let (sender1, receiver1) = mpsc::channel();
    let (sender2, receiver2) = mpsc::sync_channel(1);

    let gamethread = thread::spawn(move || run_game(sender2, receiver1, starter));

    let timer = timer_periodic(render_delay.clone());

    let rnd_delay = render_delay.load(std::sync::atomic::Ordering::Relaxed);

    let mut stop = false;
    // let mut input_stream  = buffered_terminal.terminal().poll_input(None);
    // let surface = termwiz::surface::Surface::new(rustic_yellow::SCREEN_W, rustic_yellow::SCREEN_H);

    let mut cached_sixel = CachedSixel::new(rustic_yellow::SCREEN_W, rustic_yellow::SCREEN_H);

    loop {
        timer.recv().unwrap();
        if stop {
            break;
        }
        let wait_dur = Duration::from_micros(rnd_delay);
        // let mut delay = Delay::new(Duration::from_micros(rnd_delay)).fuse();

        match receiver2.try_recv() {
            Ok(data) => {
                // println!("Received data");
                let seqno = buffered_terminal.current_seqno();
                recalculate_screen(&data, &mut buffered_terminal, &mut cached_sixel);
                if buffered_terminal.has_changes(seqno) {
                    buffered_terminal.flush().unwrap();
                }
            }
            Err(mpsc::TryRecvError::Empty) => (),
            Err(..) => {
                println!("Remote end has hung-up");
                stop = true;
                break;
            }
        }

        match buffered_terminal.terminal().poll_input(Some(wait_dur / 4)) {
            Ok(Some(InputEvent::Resized { rows, cols })) => {
                // FIXME: this is working around a bug where we don't realize
                // that we should redraw everything on resize in BufferedTerminal.
                // buf.add_change(Change::ClearScreen(Default::default()));
                // buf.resize(cols, rows);
            }
            Ok(Some(input)) => match input {
                InputEvent::Key(KeyEvent {
                    key: KeyCode::Escape,
                    ..
                }) => {
                    // Quit the app when escape is pressed
                    stop = true;
                    break;
                }
                input @ _ => {
                    // Feed input into the Ui
                    if let Some(key) = key_to_keyboard(input) {
                        let _ = sender1.send(KeyboardEvent::Down { key, shift: false });
                    }
                }
            },
            Ok(None) => {}
            Err(e) => {
                print!("{:?}\r\n", e);
                stop = true;
                break;
            }
        }

        if stop {
            break;
        }
    }

    println!("Exiting");
    sender1.send(KeyboardEvent::Down {
        key: rustic_yellow::KeyboardKey::Escape,
        shift: false,
    });
    gamethread.join().unwrap();
    println!("Game thread joined");

    buffered_terminal.add_change(Change::ClearScreen(Default::default()));
    buffered_terminal.add_change(Change::CursorVisibility(CursorVisibility::Visible));
    buffered_terminal.repaint()?;
    buffered_terminal.terminal().exit_alternate_screen()?;

    Ok(())
}

use better_panic::Settings;

use crate::sixel::{encode_raw, CachedSixel};

pub fn initialize_panic_handler() {
    std::panic::set_hook(Box::new(|panic_info| {
        Settings::auto()
            .most_recent_first(false)
            .lineno_suffix(true)
            .create_panic_handler()(panic_info);
    }));
}

fn key_to_keyboard(key: InputEvent) -> Option<rustic_yellow::KeyboardKey> {
    match key {
        InputEvent::Key(KeyEvent {
            key: KeyCode::Char('a'),
            ..
        }) => Some(rustic_yellow::KeyboardKey::A),
        InputEvent::Key(KeyEvent {
            key: KeyCode::Char('b'),
            ..
        }) => Some(rustic_yellow::KeyboardKey::B),
        InputEvent::Key(KeyEvent {
            key: KeyCode::Char('c'),
            ..
        }) => Some(rustic_yellow::KeyboardKey::C),
        InputEvent::Key(KeyEvent {
            key: KeyCode::Char('d'),
            ..
        }) => Some(rustic_yellow::KeyboardKey::D),
        InputEvent::Key(KeyEvent {
            key: KeyCode::Char('e'),
            ..
        }) => Some(rustic_yellow::KeyboardKey::E),
        InputEvent::Key(KeyEvent {
            key: KeyCode::Char('f'),
            ..
        }) => Some(rustic_yellow::KeyboardKey::F),
        InputEvent::Key(KeyEvent {
            key: KeyCode::Char('g'),
            ..
        }) => Some(rustic_yellow::KeyboardKey::G),
        InputEvent::Key(KeyEvent {
            key: KeyCode::Char('h'),
            ..
        }) => Some(rustic_yellow::KeyboardKey::H),
        InputEvent::Key(KeyEvent {
            key: KeyCode::Char('i'),
            ..
        }) => Some(rustic_yellow::KeyboardKey::I),
        InputEvent::Key(KeyEvent {
            key: KeyCode::Char('j'),
            ..
        }) => Some(rustic_yellow::KeyboardKey::J),
        InputEvent::Key(KeyEvent {
            key: KeyCode::Char('k'),
            ..
        }) => Some(rustic_yellow::KeyboardKey::K),
        InputEvent::Key(KeyEvent {
            key: KeyCode::Char('l'),
            ..
        }) => Some(rustic_yellow::KeyboardKey::L),
        InputEvent::Key(KeyEvent {
            key: KeyCode::Char('m'),
            ..
        }) => Some(rustic_yellow::KeyboardKey::M),
        InputEvent::Key(KeyEvent {
            key: KeyCode::Char('n'),
            ..
        }) => Some(rustic_yellow::KeyboardKey::N),
        InputEvent::Key(KeyEvent {
            key: KeyCode::Char('o'),
            ..
        }) => Some(rustic_yellow::KeyboardKey::O),
        InputEvent::Key(KeyEvent {
            key: KeyCode::Char('p'),
            ..
        }) => Some(rustic_yellow::KeyboardKey::P),
        InputEvent::Key(KeyEvent {
            key: KeyCode::Char('q'),
            ..
        }) => Some(rustic_yellow::KeyboardKey::Q),
        InputEvent::Key(KeyEvent {
            key: KeyCode::Char('r'),
            ..
        }) => Some(rustic_yellow::KeyboardKey::R),
        InputEvent::Key(KeyEvent {
            key: KeyCode::Char('s'),
            ..
        }) => Some(rustic_yellow::KeyboardKey::S),
        InputEvent::Key(KeyEvent {
            key: KeyCode::Char('t'),
            ..
        }) => Some(rustic_yellow::KeyboardKey::T),
        InputEvent::Key(KeyEvent {
            key: KeyCode::Char('u'),
            ..
        }) => Some(rustic_yellow::KeyboardKey::U),
        InputEvent::Key(KeyEvent {
            key: KeyCode::Char('v'),
            ..
        }) => Some(rustic_yellow::KeyboardKey::V),
        InputEvent::Key(KeyEvent {
            key: KeyCode::Char('w'),
            ..
        }) => Some(rustic_yellow::KeyboardKey::W),
        InputEvent::Key(KeyEvent {
            key: KeyCode::Char('x'),
            ..
        }) => Some(rustic_yellow::KeyboardKey::X),
        InputEvent::Key(KeyEvent {
            key: KeyCode::Char('y'),
            ..
        }) => Some(rustic_yellow::KeyboardKey::Y),
        InputEvent::Key(KeyEvent {
            key: KeyCode::Char('z'),
            ..
        }) => Some(rustic_yellow::KeyboardKey::Z),
        InputEvent::Key(KeyEvent {
            key: KeyCode::Escape,
            ..
        }) => Some(rustic_yellow::KeyboardKey::Escape),
        InputEvent::Key(KeyEvent {
            key: KeyCode::LeftArrow,
            ..
        }) => Some(rustic_yellow::KeyboardKey::Left),
        InputEvent::Key(KeyEvent {
            key: KeyCode::UpArrow,
            ..
        }) => Some(rustic_yellow::KeyboardKey::Up),
        InputEvent::Key(KeyEvent {
            key: KeyCode::RightArrow,
            ..
        }) => Some(rustic_yellow::KeyboardKey::Right),
        InputEvent::Key(KeyEvent {
            key: KeyCode::DownArrow,
            ..
        }) => Some(rustic_yellow::KeyboardKey::Down),
        InputEvent::Key(KeyEvent {
            key: KeyCode::Backspace,
            ..
        }) => Some(rustic_yellow::KeyboardKey::Backspace),
        InputEvent::Key(KeyEvent {
            key: KeyCode::Enter,
            ..
        }) => Some(rustic_yellow::KeyboardKey::Return),
        InputEvent::Key(KeyEvent {
            key: KeyCode::Char(' '),
            ..
        }) => Some(rustic_yellow::KeyboardKey::Space),
        // Continue the pattern for the rest of the alphabet
        InputEvent::Key(KeyEvent {
            key: KeyCode::Char('b'),
            ..
        }) => Some(rustic_yellow::KeyboardKey::B),
        InputEvent::Key(KeyEvent {
            key: KeyCode::Char('c'),
            ..
        }) => Some(rustic_yellow::KeyboardKey::C),
        // ... Add cases for the rest of the alphabet ...
        InputEvent::Key(KeyEvent {
            key: KeyCode::Char('z'),
            ..
        }) => Some(rustic_yellow::KeyboardKey::Z),
        // Default case for unhandled keys
        _ => None,
    }
}

use icy_sixel::{
    sixel_string, DiffusionMethod, MethodForLargest, MethodForRep, PixelFormat, Quality,
};

fn recalculate_screen(
    datavec: &[u8],
    buffered_terminal: &mut BufferedTerminal<impl Terminal>,
    cached_sixel: &mut CachedSixel,
) {
    let res = cached_sixel.tick(datavec);

    if let Some(change) = res {
        buffered_terminal.add_change(Change::ClearScreen(Default::default()));
        buffered_terminal.add_change(change);
    }

    // let six = encode_raw(
    //     datavec,
    //     rustic_yellow::SCREEN_W.try_into().unwrap(),
    //     rustic_yellow::SCREEN_H.try_into().unwrap(),
    //     PixelFormat::RGB888,
    //     DiffusionMethod::None,
    //     MethodForLargest::Norm,
    //     MethodForRep::Auto,
    //     Quality::LOW,
    // )
    // .unwrap();
    // buffered_terminal.add_change(Change::ClearScreen(Default::default()));

    return;

    // let mut rgba_data = Vec::with_capacity(datavec.len() * 4 / 3);

    // for rgb in datavec.chunks(3) {
    //     rgba_data.extend_from_slice(rgb); // Copy the RGB values
    //     rgba_data.push(255); // Add the alpha value, 255 for fully opaque
    // }

    // let rgba_image = glium::texture::RawImage2d {
    //     data: std::borrow::Cow::Borrowed(&rgba_data),
    //     width: rustic_yellow::SCREEN_W as u32,
    //     height: rustic_yellow::SCREEN_H as u32,
    //     format: glium::texture::ClientFormat::U8U8U8U8, // Now using RGBA format
    // };

    // // let rawimage2d = glium::texture::RawImage2d {
    // //     data: std::borrow::Cow::Borrowed(datavec),
    // //     width: rustic_yellow::SCREEN_W as u32,
    // //     height: rustic_yellow::SCREEN_H as u32,
    // //     format: glium::texture::ClientFormat::U8U8U8,
    // // };

    // let raw_data = rgba_image.data.into_owned();
    // // let raw_data = rawimage2d.format
    // let img_buffer: image::ImageBuffer<_, Vec<u8>> =
    //     image::ImageBuffer::from_raw(rgba_image.width, rgba_image.height, raw_data).unwrap();
    // let imgrgba8 = image::DynamicImage::ImageRgba8(img_buffer);
    // let six = encode(imgrgba8).unwrap();

    // buffered_terminal.add_change(Change::Text(six));

    // save the image to a file
    // imgrgba8.save("test.png").unwrap();

    // // Step 3: Convert ImageBuffer to DynamicImage
    // let img_buffer = image::DynamicImage::ImageRgb8(img_buffer);

    // // Now you can convert the DynamicImage to ASCII
    // let mut config_builder = artem::config::ConfigBuilder::new();
    // config_builder.target(TargetType::Shell(true, true));

    // config_builder.dimension(config::ResizingDimension::Width);

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
    // let wsize = buffered_terminal.dimensions();
    // let w_width = wsize.0 as u32;
    // let cols = wsize.1 as u32;
    // config_builder.target_size(NonZeroU32::new(cols).unwrap()); //safe to unwrap, since it is clamped before
    // let target_img_size = 1024; //terminal_size(width).max(2048);

    // //best ratio between height and width is 0.43
    // let guess_scale = |target_size: u32| -> f32 {
    //     let target_size = target_size as f32;
    //     let scale = (target_size * 0.43) / (rustic_yellow::SCREEN_W as f32);
    //     scale.clamp(0.1, 5.0)
    // };

    // let matches_scales = |target_size: u32, scale: f32| -> bool {
    //     let target_size = target_size as f32;
    //     let scale = (target_size * 0.43) / (rustic_yellow::SCREEN_W as f32);
    //     (scale - 0.1..=scale + 0.1).contains(&scale)
    // };

    // let scale = 0.3; //guess_scale(cols);
    //                  // eprintln!("Scale: {}", scale);
    // config_builder.scale(scale);

    // // config_builder.center_x(true);
    // // config_builder.center_y(true);
    // config_builder.hysteresis(true);
    // config_builder.characters(" .:-=+*#%@".to_string());

    // let config = config_builder.build();

    // convert to RGBa u8 data
    // let rgb8 = dynamic_image.to_rgb8().to_owned();
    // let rgba8 = img_buffer.to_rgba8();
    // let wimage = termwiz::image::ImageDataType::new_single_frame(
    //     rgba_image.width,
    //     rgba_image.height,
    //     imgrgba8.to_rgba8().into_vec(),
    // );

    // let imaged = Arc::new(ImageData::with_data(wimage));
    // let imaged = Arc::new(ImageData::with_data(wimage));

    // let (wdim, hdim): (usize, usize) = buffered_terminal.dimensions();
    // let top_l = TextureCoordinate::new_f32(0.0, 0.0);
    // let bottom_r: TextureCoordinate = TextureCoordinate::new_f32(1.0, 1.0);

    // let imagec = ImageCell::new(top_l, bottom_r, imaged);
    // let image = termwiz::surface::Image {
    //     width: wdim,
    //     height: hdim,
    //     bottom_right: bottom_r,
    //     top_left: top_l,
    //     image: imaged,
    // };

    // buffered_terminal.

    // let scaled_image: image::DynamicImage = dynamic_image.resize(
    //     (rustic_yellow::SCREEN_W as f32 * scale) as u32,
    //     (rustic_yellow::SCREEN_H as f32 * scale) as u32,
    //     image::imageops::FilterType::Nearest,
    // );
    // Write the scaled image to a file
    // scaled_image.save("scaled_image.png").unwrap();

    // let ascii_art = artem::convert(dynamic_image, &config);
    // let ascii_lines = ascii_art.lines().map(|line | Line::from_text(s, attrs, seqno, unicode_version)
    // let changes = ascii_to_surface(&ascii_art, surface);

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
    // for line in ascii_art.lines() {
    //     queue!(io::stdout(), style::Print(line)).unwrap();
    // }

    // execute!(io::stdout(), EndSynchronizedUpdate).unwrap();
    // io::stdout().flush().unwrap();
}

// fn ascii_to_surface(ascii: &str) -> Vec<Change> {

//     // for (i, line) in ascii.lines().enumerate() {
//     //     for (j, c) in line.chars().enumerate() {
//     //         let cell = Cell::new(
//     //             c.
//     //             termwiz::surface::CellContents::Chars(&[c]),
//     //         );
//     //         let position = Position::new(i as isize, j as isize);
//     //         let changes = vec![Change::

//     //     }
//     // }

//     // changes
// }

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
