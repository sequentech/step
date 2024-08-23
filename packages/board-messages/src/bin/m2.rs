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
    let mut siv = cursive::default();

    let canvas = Canvas::new(()).with_draw(draw);
    let style = ColorStyle::new(BaseColor::White, BaseColor::Black);
    let mut layer = Layer::new(canvas);
    layer.set_color(style);

    siv.add_layer(layer.full_screen());
    let mut theme = Theme::terminal_default();
    theme.borders = BorderStyle::None;
    siv.set_theme(theme);
    siv.add_global_callback('q', |s| s.quit());
    siv.set_fps(1);

    siv.run();
}

async fn query() -> Result<Vec<B3IndexRow>> {
    let c = PgsqlConnectionParams::new(PG_HOST, PG_PORT, PG_USER, PG_PASSW);
    let c_db = c.with_database(PG_DATABASE);
    let client = XPgsqlB3Client::new(&c_db).await?;
    client.get_boards().await
}

fn q() -> Result<Vec<B3IndexRow>> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    let inner = rt.block_on(query())?;

    Ok(inner)
}

fn draw(_: &(), p: &Printer) {
    let cells = q().unwrap();
    let draw_text = cells.len() < 300;

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
            draw_cell(p, &Vec2::new(x, y), &cell_size, &cells[index], draw_text);
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

fn draw_cell(p: &Printer, origin: &Vec2, size: &Vec2, data: &B3IndexRow, draw_text: bool) {
    let f_black = ColorStyle::new(BaseColor::White, BaseColor::Black);
    let b_gray = ColorStyle::new(BaseColor::White, Color::from_256colors(236));
    let b_magenta = ColorStyle::new(BaseColor::Black, BaseColor::Magenta);
    let b_blue = ColorStyle::new(BaseColor::Black, BaseColor::Blue);
    let b_green = ColorStyle::new(BaseColor::Black, BaseColor::Green);

    p.with_color(b_gray, |printer| {
        printer.print_rect(Rect::from_size(origin, size), " ");
    });
    let (dkg, mix) = get_values(data);
    let mut kind_origin_y = 0;
    let mut draw_kind = false;

    if mix == 0.0 {
        let bar_height = (dkg * f64::from(size.y as u32)).round() as usize;
        let bar_origin_y = origin.y + (size.y - bar_height);
        kind_origin_y = bar_origin_y;

        let bar_origin = Vec2::new(origin.x, bar_origin_y);
        let bar_size = Vec2::new(size.x, bar_height);

        let color = if dkg == 1.0 { b_green } else { b_blue };

        if bar_height > 0 {
            p.with_color(color, |printer| {
                printer.print_rect(Rect::from_size(bar_origin, bar_size), " ");
            });
            draw_kind = true;
        }
    } else {
        let bar_height = (mix * f64::from(size.y as u32)).round() as usize;
        let bar_origin_y = origin.y + (size.y - bar_height);
        kind_origin_y = bar_origin_y;

        let bar_origin = Vec2::new(origin.x, bar_origin_y);
        let bar_size = Vec2::new(size.x, bar_height);

        let color = if mix == 1.0 { b_green } else { b_magenta };

        if bar_height > 0 {
            p.with_color(color, |printer| {
                printer.print_rect(Rect::from_size(bar_origin, bar_size), " ");
            });
            draw_kind = true;
        }
    }

    if draw_text {
        if draw_kind {
            let text_color = if mix > 0.0 {
                b_magenta
            } else if dkg > 0.0 {
                b_blue
            } else {
                f_black
            };

            let title = format!("{}", &data.last_message_kind);
            let max_chars = title.len().min(size.x - 2);

            p.with_color(text_color, |printer| {
                printer.print((origin.x + 1, kind_origin_y), &title[0..max_chars]);
                // printer.print(origin, &blue.to_string());
            });
        }

        // Title

        let name = &data.board_name;
        // let max_chars = data.len().min(size.x - 2);
        let pct = if mix == 0.0 {
            (dkg * 100.0).round()
        } else {
            (mix * 100.0).round()
        };

        let text_color = if mix == 1.0 {
            b_green
        } else if dkg == 1.0 {
            b_green
        } else {
            f_black
        };

        let title = format!("{name} ({}%)", pct);
        let max_chars = title.len().min(size.x - 2);

        p.with_color(text_color, |printer| {
            printer.print_hline(origin, size.x, " ");
            printer.print((origin.x + 1, origin.y), &title[0..max_chars]);
            // printer.print(origin, &blue.to_string());
        });
    }
}
