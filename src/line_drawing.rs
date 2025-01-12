use crate::math::{arrow::Arrow, matrix2::Matrix2, point::Point, rect::Rect};

/// Draw the line using the slope along the x-axis to calculate the y coordinate for each x
/// position. If the slope is too steep use the slope along the y-axis instead.
/// Returns pixels on the line. Pixels are contiguous, in other words the following situation
/// should not happen:
/// ┌─┬─┐
/// └─┴─┼─┬─┐
///     └─┴─┘
/// Instead it should look like this
/// ┌─┬─┐
/// └─┼─┼─┬─┐
///   └─┴─┴─┘
/// https://en.wikipedia.org/wiki/Line_drawing_algorithm
pub fn slope_draw_thin_line(arrow: Arrow<i64>, contiguous: bool) -> Vec<Point<i64>> {
    // Rest of code requires dir.x != 0 or dir.y != 0
    if arrow.dir() == Point::ZERO {
        return vec![arrow.a];
    }

    // Transform arrow into the first 45° of the plane in other words `dir.x >= 0` and
    // `0 <= dir.y <= dir.x`.
    let phi = {
        let dir = arrow.dir();
        let mut phi = Matrix2::ID;
        if dir.x < 0 {
            phi = Matrix2::mirror_x();
        }

        if dir.y < 0 {
            phi = Matrix2::<i64>::mirror_y() * phi;
        }

        if dir.x.abs() < dir.y.abs() {
            phi = Matrix2::<i64>::SWAP_XY * phi;
        }

        phi
    };
    let phi_inv = phi.transpose();
    assert_eq!(phi * phi_inv, Matrix2::ID);

    let arrow = phi * arrow;
    let dir = arrow.dir();
    assert!(0 <= dir.y);
    assert!(0 < dir.x);
    assert!(dir.y <= dir.x);

    let slope = dir.y as f64 / dir.x as f64;

    let mut points: Vec<Point<i64>> = Vec::new();
    for x_offset in 0..=dir.x {
        let y_offset = (slope * (x_offset as f64)).round() as i64;
        let point = arrow.a + Point(x_offset, y_offset);
        if contiguous {
            // Make sure line is contiguous
            if let Some(&previous) = points.last() {
                if previous.y < point.y {
                    points.push(Point(previous.x, previous.y + 1))
                }
            }
        }

        points.push(point);
    }

    // Transform back
    for point in &mut points {
        *point = phi_inv * *point;
    }

    points
}

/// Low point of Rect<i64> with center closest to `center` and the given size
pub fn rectangle_with_center(center: Point<f64>, size: Point<i64>) -> Rect<i64> {
    // For a rectangle `rect` with `size` and `center`, `rect.low = center - size/2`
    let half_size: Point<f64> = 0.5 * size.as_f64();
    let low = (center - half_size).round().as_i64();
    Rect::low_size(low, size)
}

pub fn circular_brush(center: Point<f64>, radius: f64, pixel: Point<i64>) -> bool {
    // REVISIT: Use multisampling
    let pixel_center = pixel.as_f64() + Point(0.5, 0.5);
    let r = center.distance(pixel_center);
    r < radius
}

pub fn draw_line_slope(arrow: Arrow<f64>, width: f64) -> impl Iterator<Item = Point<i64>> {
    let size = width.ceil() as i64;
    assert!(size > 0);
    let stamp_size = Point(size, size);
    let a = rectangle_with_center(arrow.a, stamp_size).low();
    let b = rectangle_with_center(arrow.b, stamp_size).low();

    let offset_center = 0.5 * stamp_size.as_f64();
    let offset_rect = Rect::low_high(Point::ZERO, stamp_size);

    slope_draw_thin_line(Arrow::new(a, b), size == 1)
        .into_iter()
        .flat_map(move |top_left| {
            offset_rect.iter_half_open().filter_map(move |offset| {
                // Pixel centers are at half ints
                circular_brush(offset_center, 0.5 * width, offset).then_some(top_left + offset)
            })
        })
}

pub fn draw_line_distance_method(arrow: Arrow<f64>, radius: f64) -> Vec<Point<i64>> {
    let bounds = arrow.bounds().padded(radius);
    let pixel_bounds = Rect::low_high(bounds.low().floor().as_i64(), bounds.high().ceil().as_i64());

    let mut result = Vec::new();
    for pixel in pixel_bounds.iter_closed() {
        let distance = arrow.distance(pixel.as_f64());
        if distance < radius {
            result.push(pixel);
        }
    }

    result
}

#[cfg(test)]
mod test {
    use crate::{
        field::RgbaField,
        line_drawing::draw_line_slope,
        math::{arrow::Arrow, point::Point, rect::Rect, rgba8::Rgba8},
    };

    /// Not really a test, draws a bunch of lines with different methods and widths and saves
    /// the result to an image.
    #[test]
    pub fn draw_samples() {
        let mut image = RgbaField::filled(
            Rect::low_high(Point::ZERO, Point(512, 512)),
            Rgba8::TRANSPARENT,
        );

        let offsets: Vec<Point<f64>> = (0..32)
            .map(|k| {
                let angle = (k as f64) / 32.0 * 2.0 * std::f64::consts::PI;
                let dir = Point(angle.cos(), angle.sin());
                let offset = 128.0 * dir;
                offset.as_f64()
            })
            .collect();

        let start = Point(256.0, 256.0);
        for &offset in &offsets {
            let arrow = Arrow::new(start, start + offset);

            for line_pixel in draw_line_slope(arrow, 2.0) {
                image[line_pixel] = Rgba8::RED;
            }
        }

        image
            .save("test_resources/line_drawing/output.png")
            .unwrap();
    }
}
