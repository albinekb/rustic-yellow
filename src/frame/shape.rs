use std::io::{Error, Result, Stdout};
use termwiz::cell::*;
use termwiz::color::ColorAttribute;
use termwiz::surface::Position::*;
use termwiz::surface::{Change, Surface};

/// A *Drawable* is something, that can be drawn in the terminal.
pub trait Drawable {
    fn draw(
        &self,
        surface: &mut Surface,
        stroke_color: ColorAttribute,
        fill_color: ColorAttribute,
    ) -> Result<()>;
}

/// A struct that makes it possible to draw a background.
///
/// # Example
///
/// ```
/// let out = stdout();
/// Background.draw(out, Color::Black, Color::Reset);
/// ```
/// You can also use the macro:
/// ```
/// let out = stdout();
/// draw_background!(out, Color::Black);
/// ```
pub struct Background;

impl Drawable for Background {
    fn draw(
        &self,
        surface: &mut Surface,
        stroke_color: ColorAttribute,
        _fill_color: ColorAttribute,
    ) -> Result<()> {
        let (w, h) = surface.dimensions();

        surface.add_change(Change::Attribute(AttributeChange::Background(stroke_color)));
        for x in 0..w - 1 {
            for y in 0..h - 1 {
                surface.add_change(Change::CursorPosition {
                    x: Absolute(x),
                    y: Absolute(y),
                });
                surface.add_change(Change::Text(" ".to_string()));
            }
        }

        Ok(())
    }
}

/// A struct that makes it possible to draw a point.
///
/// # Example
///
/// ```
/// let out = stdout();
/// Point(0, 0).draw(&mut out, Color::White, Color::Reset);
/// ```
/// You can also use the macro:
/// ```
/// let out = stdout();
/// draw_point!(out, 0, 0, Color::White);
/// ```
pub struct Point(pub u16, pub u16);

impl Drawable for Point {
    fn draw(
        &self,
        surface: &mut Surface,
        stroke_color: ColorAttribute,
        _fill_color: ColorAttribute,
    ) -> Result<()> {
        surface.add_change(Change::CursorPosition {
            x: Absolute(self.0 as usize),
            y: Absolute(self.1 as usize),
        });
        surface.add_change(Change::Attribute(AttributeChange::Background(stroke_color)));
        surface.add_change(Change::Text(" ".to_string()));

        Ok(())
    }
}

/// A struct that makes it possible to draw a line.
///
/// # Example
///
/// ```
/// let out = stdout();
/// Line(0, 0, 10, 10).draw(&mut out, Color::White, Color::Reset);
/// ```
/// You can also use the macro:
/// ```
/// let out = stdout();
/// draw_line!(out, 0, 0, 10, 10, Color::White);
/// ```
pub struct Line(pub u16, pub u16, pub u16, pub u16);

impl Drawable for Line {
    fn draw(
        &self,
        surface: &mut Surface,
        stroke_color: ColorAttribute,
        _fill_color: ColorAttribute,
    ) -> Result<()> {
        surface.add_change(Change::Attribute(AttributeChange::Background(stroke_color)));

        if self.0 == self.2 {
            for y_offset in 0..self.3 {
                surface.add_change(Change::CursorPosition {
                    x: Absolute(self.0 as usize),
                    y: Absolute((self.1 + y_offset) as usize),
                });
                surface.add_change(Change::Text(" ".to_string()));
            }
        } else if self.1 == self.3 {
            for x_offset in 0..self.2 {
                surface.add_change(Change::CursorPosition {
                    x: Absolute((self.0 + x_offset) as usize),
                    y: Absolute(self.1 as usize),
                });
                surface.add_change(Change::Text(" ".to_string()));
            }
        } else {
            let y_delta = self.3 as i32 - self.1 as i32;
            let x_chunks = (self.2 as i32 - self.0 as i32) / y_delta;
            for y_offset in 0..y_delta + 1 {
                for x_offset in (x_chunks * y_offset)..(x_chunks * y_offset + 1) {
                    surface.add_change(Change::CursorPosition {
                        x: Absolute((self.0 as i32 + x_offset) as usize),
                        y: Absolute((self.1 as i32 + y_offset) as usize),
                    });

                    surface.add_change(Change::Text(" ".to_string()));
                }
            }
        }

        Ok(())
    }
}

/// A struct that makes it possible to draw custom shapes.
///
/// # Example
///
/// ```
/// let out = stdout();
/// let custom_shape = CustomShape(vec![
///     Point(0, 0),
///     Point(10, 0),
///     Point(5, 5)
/// ], true);
/// custom_shape.draw(&mut out, Color::White, Color::Reset);
/// ```
///
/// You can also use the macro:
///
/// ```
/// let out = stdout();
/// draw_custom_shape!(out, [0, 0, 10, 0, 5, 5], Color::White, true);
/// ```
pub struct CustomShape(pub Vec<Point>, pub bool);

impl Drawable for CustomShape {
    fn draw(
        &self,
        surface: &mut Surface,
        stroke_color: ColorAttribute,
        fill_color: ColorAttribute,
    ) -> Result<()> {
        if self.0.len() < 3 {
            return Err(Error::new(
                std::io::ErrorKind::Other,
                "Not enough vertecies!",
            ));
        }

        for i in 0..self.0.len() - 1 {
            let cur = &self.0[i];
            let next = &self.0[i + 1];

            Line(cur.0, cur.1, next.0, next.1).draw(surface, stroke_color, fill_color)?;
        }

        if self.1 {
            let first = self.0.first().unwrap();
            let last = self.0.last().unwrap();
            Line(first.0, first.1, last.0, last.1).draw(surface, stroke_color, fill_color)?;
        }

        Ok(())
    }
}

/// A struct that makes it possible to draw a rectangle.
///
/// # Example
///
/// ```
/// let out = stdout();
/// Rect(0, 0, 10, 10).draw(&mut out, Color::Black, Color::Reset)
/// ```
/// You can also use the macro:
/// ```
/// let out = stdout();
/// draw_rect!(out, 0, 0, 10, 10, Color::White, Color::Black);
/// ```
/// If you need to draw a Square, you can also use the [`draw_square`](macro.draw_square.html) macro:
/// ```
/// let out = stdout();
/// draw_square!(out, 0, 0, 10, Color::White, Color::Black);
/// ```
pub struct Rect(pub u16, pub u16, pub u16, pub u16);

impl Drawable for Rect {
    fn draw(
        &self,
        surface: &mut Surface,
        stroke_color: ColorAttribute,
        fill_color: ColorAttribute,
    ) -> Result<()> {
        for x_offset in 0..self.2 {
            for y_offset in 0..self.3 {
                let x = self.0 + x_offset;
                let y = self.1 + y_offset;
                surface.add_change(Change::CursorPosition {
                    x: Absolute(x as usize),
                    y: Absolute(y as usize),
                });
                if x_offset == 0
                    || x_offset == self.2 - 1
                    || y_offset == 0
                    || y_offset == self.3 - 1
                {
                    surface
                        .add_change(Change::Attribute(AttributeChange::Background(stroke_color)));
                } else {
                    surface.add_change(Change::Attribute(AttributeChange::Background(fill_color)));
                }
                surface.add_change(Change::Text(" ".to_string()));
            }
        }

        Ok(())
    }
}

/// A struct that makes it possible to draw a circle.
///
/// # Example
///
/// ```
/// let out = stdout();
/// Circle(0, 0, 10).draw(&mut out, Color::Black, Color::Reset)
/// ```
/// You can also use the macro:
/// ```
/// let out = stdout();
/// draw_circle!(out, 0, 0, 10, Color::White, Color::Black);
/// ```
pub struct Circle(pub u16, pub u16, pub u16);

impl Drawable for Circle {
    fn draw(
        &self,
        surface: &mut Surface,
        stroke_color: ColorAttribute,
        fill_color: ColorAttribute,
    ) -> Result<()> {
        let r = self.2 as i32;
        let (w, h) = surface.dimensions();
        for x_offset in -r..r + 1 {
            for y_offset in -(r - x_offset.abs())..(r - x_offset.abs() + 1) {
                let x = (self.0 as i32 + x_offset) as usize;
                let y = (self.1 as i32 + y_offset) as usize;
                if x >= w || y >= h {
                    continue;
                }

                surface.add_change(Change::CursorPosition {
                    x: Absolute(x),
                    y: Absolute(y),
                });
                if (x_offset + y_offset).abs() == r || (x_offset - y_offset).abs() == r {
                    surface
                        .add_change(Change::Attribute(AttributeChange::Background(stroke_color)));
                } else {
                    surface.add_change(Change::Attribute(AttributeChange::Background(fill_color)));
                }
                surface.add_change(Change::Text(" ".to_string()));
            }
        }

        Ok(())
    }
}

/// A macro that makes it possible to draw a background. See [`Background`](struct.Background.html).
#[macro_export]
macro_rules! draw_background {
    ($out:ident, $background_color:expr) => {
        rustic_yellow::frame::shape::Background.draw(&mut $out, $background_color, color)?;
    };
}

/// A macro that makes it possible to draw a point. See [`Point`](struct.Point.html).
#[macro_export]
macro_rules! draw_point {
    ($out:ident, $x:expr, $y:expr, $point_color:expr) => {
        rustic_yellow::frame::shape::Point($x, $y).draw(
            &mut $out,
            $point_color,
            termwiz::color::ColorAttribute::Default,
        )?;
    };
}

/// A macro that makes it possible to draw custom shapes. See [`CustomShape`](struct.CustomShape.html).
#[macro_export]
macro_rules! draw_custom_shape {
  ($out:ident, [$($x:expr, $y:expr),+], $stroke_color:expr, $close:literal) => {
      {
          let mut points = Vec::new();
          $(
              points.push(Point($x, $y));
          )+;
          CustomShape(points, $close).draw(&mut $out, $stroke_color, termwiz::color::ColorAttribute::Default)?;
      }
  };
}

/// A macro that makes it possible to draw a line. See [`Line`](struct.Line.html).
#[macro_export]
macro_rules! draw_line {
    ($out:ident, $x1:expr, $y1:expr, $x2:expr, $y2:expr, $stroke_color:expr) => {
        rustic_yellow::frame::shape::Line($x1, $y1, $x2, $y2).draw(
            &mut $out,
            $stroke_color,
            termwiz::color::ColorAttribute::Default,
        )?;
    };
}

/// A macro that makes it possible to draw a rectangle. See [`Rect`](struct.Rect.html).
#[macro_export]
macro_rules! draw_rect {
    ($out:ident, $x:expr, $y:expr, $w:expr, $h:expr, $stroke_color:expr, $fill_color:expr) => {
        rustic_yellow::frame::shape::Rect($x, $y, $w, $h).draw(
            &mut $out,
            $stroke_color,
            $fill_color,
        )?;
    };
}

/// A macro that makes it possible to draw a square. See [`Rect`](struct.Rect.html).
#[macro_export]
macro_rules! draw_square {
    ($out:ident, $x:expr, $y:expr, $a:expr, $stroke_color:expr, $fill_color:expr) => {
        rustic_yellow::frame::shape::Rect($x, $y, $a, $a).draw(
            &mut $out,
            $stroke_color,
            $fill_color,
        )?;
    };
}

/// A macro that makes it possible to draw a circle. See [`Circle`](struct.Circle.html).
#[macro_export]
macro_rules! draw_circle {
    ($out:ident, $x:expr, $y:expr, $r:expr, $stroke_color:expr, $fill_color:expr) => {
        rustic_yellow::frame::shape::Circle($x, $y, $r).draw(
            &mut $out,
            $stroke_color,
            $fill_color,
        )?;
    };
}

pub use draw_background;
pub use draw_circle;
pub use draw_custom_shape;
pub use draw_line;
pub use draw_point;
pub use draw_rect;
pub use draw_square;
