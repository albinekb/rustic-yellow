#![allow(clippy::bool_to_int_with_if, clippy::identity_op)]

pub use crate::gpu::{SCREEN_H, SCREEN_W};
pub use crate::keypad::KeypadKey;
pub use crate::sound::AudioPlayer;

pub mod cpu;

mod gpu;
mod keypad;
mod mbc5;
mod mmu;
mod serial;
mod sound;
mod timer;
