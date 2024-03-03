//! Sixel protocol implementations.
//! Uses [`sixel-bytes`] to draw image pixels, if the terminal [supports] the [Sixel] protocol.
//! Needs the `sixel` feature.
//!
//! [`sixel-bytes`]: https://github.com/benjajaja/sixel-bytes
//! [supports]: https://arewesixelyet.com
//! [Sixel]: https://en.wikipedia.org/wiki/Sixel
use anyhow::Result;
use icy_sixel::{
    self as sizel, sixel_string, DiffusionMethod, MethodForLargest, MethodForRep, PixelFormat,
    Quality,
};
use image::{math::Rect, DynamicImage, Rgb};
use sizel::{dither::sixel_dither, output::sixel_output, tosixel};
use std::cmp::min;
use termwiz::surface::{Change, Position};

// TODO: change E to sixel_rs::status::Error and map when calling
pub fn encode(img: DynamicImage) -> Result<String> {
    let (w, h) = (img.width(), img.height());
    let img_rgba8 = img.to_rgba8();
    let bytes = img_rgba8.as_raw();

    let data = sixel_string(
        bytes,
        w as i32,
        h as i32,
        PixelFormat::RGBA8888,
        DiffusionMethod::Stucki,
        MethodForLargest::Auto,
        MethodForRep::Auto,
        Quality::HIGH,
    )
    .map_err(|e| anyhow::anyhow!("sixel encoding failed: {:?}", e))?;
    Ok(data)
}

pub fn encode_raw(
    bytes: &[u8],
    width: i32,
    height: i32,
    pixelformat: PixelFormat,
    diffusion: DiffusionMethod,
    method_for_largest: MethodForLargest,
    method_for_rep: MethodForRep,
    quality: Quality,
) -> Result<Vec<u8>> {
    let mut sixel_data: Vec<u8> = Vec::new();

    let mut sixel_output = sixel_output::new(&mut sixel_data);
    sixel_output.set_encode_policy(sizel::EncodePolicy::SIZE);
    let mut sixel_dither = sixel_dither::new(256).unwrap();

    sixel_dither
        .initialize(
            bytes,
            width,
            height,
            pixelformat,
            method_for_largest,
            method_for_rep,
            quality,
        )
        .unwrap();

    sixel_dither.set_pixelformat(pixelformat);
    sixel_dither.set_diffusion_type(diffusion);

    let mut bytes = bytes.to_vec();

    sixel_output
        .encode(&mut bytes, width, height, 0, &mut sixel_dither)
        .unwrap();

    Ok(sixel_data)
}

pub struct CachedSixel {
    sixel: String,
    width: i32,
    height: i32,
    sixel_data: Vec<u8>,
}

impl CachedSixel {
    pub fn new(width: usize, height: usize) -> Self {
        CachedSixel {
            sixel: String::new(),
            height: height as i32,
            width: width as i32,
            sixel_data: Vec::new(),
        }
    }

    pub fn get_sixel(&self) -> String {
        self.sixel.clone()
    }

    pub fn tick(&mut self, bytes: &[u8]) -> Option<Change> {
        let data: Vec<u8> = encode_raw(
            bytes,
            self.width,
            self.height,
            PixelFormat::RGB888,
            DiffusionMethod::None,
            MethodForLargest::Norm,
            MethodForRep::Auto,
            Quality::LOW,
        )
        .unwrap();

        if self.sixel_data.is_empty() {
            self.sixel = String::from_utf8_lossy(&data).to_string();
            self.sixel_data = data;
            return Some(Change::Text(self.sixel.clone()));
        }

        // Check if the data has changed
        if self.sixel_data == data {
            return None;
        }

        self.sixel = String::from_utf8_lossy(&data).to_string();
        // let old_data = self.sixel_data.clone();
        self.sixel_data = data;
        // Calculate Changes and update the data
        // if let Some(changes) = diff(
        //     &old_data,
        //     &self.sixel_data,
        //     self.width as usize,
        //     self.height as usize,
        //     6,
        // ) {
        //     return Some(changes);
        // }

        return Some(Change::Text(self.sixel.clone()));
    }
}

fn diff(
    old_data: &[u8],
    new_data: &[u8],
    width: usize,
    height: usize,
    tile_size: usize,
) -> Option<Vec<Change>> {
    if old_data.len() != new_data.len() {
        panic!("Data arrays must be of equal length");
    }

    let mut changes: Vec<Change> = Vec::new();

    let tiles_x = width / tile_size;

    for (index, (old, new)) in old_data
        .chunks(tile_size * 3)
        .zip(new_data.chunks(tile_size * 3))
        .enumerate()
    {
        if old != new {
            // Calculate the position of the changed tile
            let tile_x = index % tiles_x;
            let tile_y = index / tiles_x;

            // Here we re-encode the changed tile into Sixel
            // You'll need to modify `encode_raw` to handle partial encoding
            let encoded_tile = String::from_utf8_lossy(&new);

            changes.push(Change::CursorPosition {
                x: Position::Absolute(tile_x),
                y: Position::Absolute(tile_y),
            });

            changes.push(Change::Text(encoded_tile.to_string()));
        }
    }

    Some(changes)
}

use ratatui::{buffer::Buffer, layout};

pub fn render_sixel(
    rect: layout::Rect,
    data: &str,
    area: layout::Rect,
    buf: &mut Buffer,
    overdraw: bool,
) {
    let render_area = match render_area(rect, area, overdraw) {
        None => {
            // If we render out of area, then the buffer will attempt to write regular text (or
            // possibly other sixels) over the image.
            //
            // On some implementations (e.g. Xterm), this actually works but the image is
            // forever overwritten since we won't write out the same sixel data for the same
            // (col,row) position again (see buffer diffing).
            // Thus, when the area grows, the newly available cells will skip rendering and
            // leave artifacts instead of the image data.
            //
            // On some implementations (e.g. ???), only text with its foreground color is
            // overlayed on the image, also forever overwritten.
            //
            // On some implementations (e.g. patched Alactritty), image graphics are never
            // overwritten and simply draw over other UI elements.
            //
            // Note that [ResizeProtocol] forces to ignore this early return, since it will
            // always resize itself to the area.
            return;
        }
        Some(r) => r,
    };

    buf.get_mut(render_area.left(), render_area.top())
        .set_symbol(data);
    let mut skip_first = false;

    // Skip entire area
    for y in render_area.top()..render_area.bottom() {
        for x in render_area.left()..render_area.right() {
            if !skip_first {
                skip_first = true;
                continue;
            }
            buf.get_mut(x, y).set_skip(true);
        }
    }
}

fn render_area(rect: layout::Rect, area: layout::Rect, overdraw: bool) -> Option<layout::Rect> {
    if overdraw {
        return Some(layout::Rect::new(
            area.x,
            area.y,
            min(rect.width, area.width),
            min(rect.height, area.height),
        ));
    }

    if rect.width > area.width || rect.height > area.height {
        return None;
    }
    Some(layout::Rect::new(area.x, area.y, rect.width, rect.height))
}

// fn render(rect: Rect, data: &str, area: Rect, buf: &mut Buffer, overdraw: bool) {
//     let render_area = match render_area(rect, area, overdraw) {
//         None => {
//             // If we render out of area, then the buffer will attempt to write regular text (or
//             // possibly other sixels) over the image.
//             //
//             // On some implementations (e.g. Xterm), this actually works but the image is
//             // forever overwritten since we won't write out the same sixel data for the same
//             // (col,row) position again (see buffer diffing).
//             // Thus, when the area grows, the newly available cells will skip rendering and
//             // leave artifacts instead of the image data.
//             //
//             // On some implementations (e.g. ???), only text with its foreground color is
//             // overlayed on the image, also forever overwritten.
//             //
//             // On some implementations (e.g. patched Alactritty), image graphics are never
//             // overwritten and simply draw over other UI elements.
//             //
//             // Note that [ResizeProtocol] forces to ignore this early return, since it will
//             // always resize itself to the area.
//             return;
//         }
//         Some(r) => r,
//     };

//     buf.get_mut(render_area.left(), render_area.top())
//         .set_symbol(data);
//     let mut skip_first = false;

//     // Skip entire area
//     for y in render_area.top()..render_area.bottom() {
//         for x in render_area.left()..render_area.right() {
//             if !skip_first {
//                 skip_first = true;
//                 continue;
//             }
//             buf.get_mut(x, y).set_skip(true);
//         }
//     }
// }

// fn render_area(rect: Rect, area: Rect, overdraw: bool) -> Option<Rect> {
//     if overdraw {
//         return Some(Rect::new(
//             area.x,
//             area.y,
//             min(rect.width, area.width),
//             min(rect.height, area.height),
//         ));
//     }

//     if rect.width > area.width || rect.height > area.height {
//         return None;
//     }
//     Some(Rect::new(area.x, area.y, rect.width, rect.height))
// }

// #[derive(Clone)]
// pub struct StatefulSixel {
//     source: ImageSource,
//     current: Sixel,
//     hash: u64,
// }

// impl StatefulSixel {
//     pub fn new(source: ImageSource) -> StatefulSixel {
//         StatefulSixel {
//             source,
//             current: Sixel::default(),
//             hash: u64::default(),
//         }
//     }
// }

// impl StatefulProtocol for StatefulSixel {
//     fn needs_resize(&mut self, resize: &Resize, area: Rect) -> Option<Rect> {
//         resize.needs_resize(&self.source, self.current.rect, area, false)
//     }
//     fn resize_encode(&mut self, resize: &Resize, background_color: Option<Rgb<u8>>, area: Rect) {
//         if area.width == 0 || area.height == 0 {
//             return;
//         }

//         let force = self.source.hash != self.hash;
//         if let Some((img, rect)) = resize.resize(
//             &self.source,
//             self.current.rect,
//             area,
//             background_color,
//             force,
//         ) {
//             match encode(img) {
//                 Ok(data) => {
//                     self.current = Sixel { data, rect };
//                     self.hash = self.source.hash;
//                 }
//                 Err(_err) => {
//                     // TODO: save err in struct and expose in trait?
//                 }
//             }
//         }
//     }
//     fn render(&mut self, area: Rect, buf: &mut Buffer) {
//         render(self.current.rect, &self.current.data, area, buf, true);
//     }
// }
