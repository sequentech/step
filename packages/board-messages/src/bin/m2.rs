use anyhow::Result;

use board_messages::grpc::pgsql::{B3IndexRow, PgsqlConnectionParams, XPgsqlB3Client};
use cursive::style::{BaseColor, Color, ColorStyle};
use cursive::theme::{BorderStyle, Theme};
use cursive::view::Resizable;
use cursive::views::{Canvas, Layer};
use cursive::{Printer, Rect, Vec2};

const PG_DATABASE: &'static str = "protocoldb";
const PG_HOST: &'static str = "localhost";
const PG_USER: &'static str = "postgres";
const PG_PASSW: &'static str = "postgrespw";
const PG_PORT: u32 = 49153;

fn main() {
    // Start as usual
    let mut siv = cursive::default();
    let mut theme = Theme::terminal_default();
    theme.borders = BorderStyle::None;
    siv.set_theme(theme);
    siv.add_global_callback('q', |s| s.quit());
    siv.set_fps(1);

    // Canvas lets us easily override any method.
    // Canvas can have states, but we don't need any here, so we use `()`.
    // siv.add_layer(Canvas::new(()).with_draw(draw).fixed_size((20, 10)));

    let canvas = Canvas::new(()).with_draw(draw);
    /*let r = ResizedView::new(
        cursive::view::SizeConstraint::Full,
        cursive::view::SizeConstraint::Full,
        canvas,
    );*/
    /*let panel = Panel::new(r)
    .title("hohohoho")
    .title_position(cursive::align::HAlign::Left);*/

    let style = ColorStyle::new(BaseColor::White, BaseColor::Black);
    let mut layer = Layer::new(canvas);
    layer.set_color(style);

    siv.add_layer(layer.full_screen());

    siv.run();
}
use tokio::runtime::Runtime;

async fn query() -> Result<Vec<B3IndexRow>> {
    let c = PgsqlConnectionParams::new(PG_HOST, PG_PORT, PG_USER, PG_PASSW);
    let c_db = c.with_database(PG_DATABASE);
    let client = XPgsqlB3Client::new(&c_db).await?;
    // info!("pgsql connection ok");
    client.get_boards().await
}

fn q() -> Result<Vec<B3IndexRow>> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    // Call the asynchronous connect method using the runtime.
    let inner = rt.block_on(query())?;

    Ok(inner)
}

/// Method used to draw the cube.
///
/// This takes as input the Canvas state and a printer.
fn draw(_: &(), p: &Printer) {
    let cells = q().unwrap();

    let len = cells.len();
    let lenrt = (cells.len() as f64).sqrt() as usize;

    let width = if len % lenrt == 0 && len / lenrt == lenrt {
        lenrt
    } else {
        lenrt + 1 // 2
    };

    let height = if len % width == 0 {
        len / width
    } else {
        1 + (len / width)
    };

    let cell_counts = Vec2::new(width, height);

    let x_offset = (p.size.x % cell_counts.x) / 2;
    let cell_width = p.size.x / cell_counts.x;
    let y_offset = (p.size.y % cell_counts.y) / 2;
    let cell_height = p.size.y / cell_counts.y;

    let cell_size = Vec2::new(cell_width, cell_height);

    let mut x = x_offset;
    let mut y = y_offset;
    let mut index = 0;
    for _i in 0..cell_counts.y {
        for _j in 0..cell_counts.x {
            draw_cell(p, &Vec2::new(x, y), &cell_size, &cells[index]);
            x = x + cell_width;
            index = index + 1;
            if index == len {
                return;
            }
        }
        y = y + cell_height;
        x = x_offset;
    }
}

fn get_values(row: &B3IndexRow) -> (f64, f64) {
    let dkg_max = 1 + (5 * row.trustees_no);
    let tally_size = f64::from(
        1 + (2 * row.threshold_no) + (row.threshold_no * (row.threshold_no - 1)) + row.trustees_no,
    );

    if row.message_count <= dkg_max {
        (f64::from(row.message_count) / f64::from(dkg_max), 0.0)
    } else if row.batch_count == 0 {
        (1.0, 0.0)
    } else {
        let tally = f64::from(row.message_count - dkg_max);
        let batches = f64::from(row.batch_count);
        //  let batches = (tally / tally_size).ceil();
        let target = batches * tally_size;

        (1.0, (tally / target))
    }
}

fn draw_cell(p: &Printer, origin: &Vec2, size: &Vec2, data: &B3IndexRow) {
    let black = ColorStyle::new(BaseColor::White, BaseColor::Black);
    let gray = ColorStyle::new(BaseColor::White, Color::from_256colors(234));
    let green_c = ColorStyle::new(BaseColor::Black, BaseColor::Green);
    let blue_c = ColorStyle::new(BaseColor::Black, BaseColor::Blue);

    p.with_color(gray, |printer| {
        printer.print_rect(Rect::from_size(origin, size), " ");
    });

    let (green, blue) = get_values(data);
    let bar_height = (green * f64::from(size.y as u32)).round() as usize;
    let bar_origin_y = origin.y + (size.y - bar_height);

    let bar_origin = Vec2::new(origin.x, bar_origin_y);
    let bar_size = Vec2::new(size.x, bar_height);

    if bar_height > 0 {
        p.with_color(green_c, |printer| {
            printer.print_rect(Rect::from_size(bar_origin, bar_size), " ");
        });
    }

    let bar_height = (blue * f64::from(size.y as u32)).round() as usize;
    let bar_origin_y = origin.y + (size.y - bar_height);

    let bar_origin = Vec2::new(origin.x, bar_origin_y);
    let bar_size = Vec2::new(size.x, bar_height);

    if bar_height > 0 {
        p.with_color(blue_c, |printer| {
            printer.print_rect(Rect::from_size(bar_origin, bar_size), " ");
        });
    }

    let name = &data.board_name;
    // let max_chars = data.len().min(size.x - 2);
    let pct = if blue == 0.0 {
        (green * 100.0).round()
    } else {
        (blue * 100.0).round()
    };

    let title = format!("{name} ({}%)", pct);
    let max_chars = title.len().min(size.x - 2);

    let text_color = if blue == 1.0 {
        blue_c
    } else if green == 1.0 {
        green_c
    } else {
        black
    };

    p.with_color(text_color, |printer| {
        printer.print_hline(origin, size.x, " ");
        printer.print(origin, &title[0..max_chars]);
        // printer.print(origin, &blue.to_string());
    });
}

/*// Gradient for the front color
fn front_color(x: u8, y: u8, x_max: u8, y_max: u8) -> Color {
    // We return a full 24-bits RGB color, but some backends
    // will project it to a 256-colors palette.
    Color::Rgb(
        x * (255 / x_max),
        y * (255 / y_max),
        (x + 2 * y) * (255 / (x_max + 2 * y_max)),
    )
}

// Gradient for the background color
fn back_color(x: u8, y: u8, x_max: u8, y_max: u8) -> Color {
    // Let's try to have a gradient in a different direction than the front color.
    Color::Rgb(
        128 + (2 * y_max + x - 2 * y) * (128 / (x_max + 2 * y_max)),
        255 - y * (255 / y_max),
        255 - x * (255 / x_max),
    )
}*/
