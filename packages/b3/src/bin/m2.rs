// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//  cargo run --bin m2 --features=monitor 2> error
cfg_if::cfg_if! { if #[cfg(feature = "monitor")] {

use std::time::{Duration, SystemTime};
use anyhow::Result;

use b3::grpc::pgsql::{
    B3IndexRow, PgsqlB3Client, PgsqlConnectionParams, PgsqlDbConnectionParams,
};
use clap::Parser;
use cursive::event::Event;
use cursive::style::{BaseColor, Color, ColorStyle};
use cursive::theme::{BorderStyle, Theme};
use cursive::traits::Nameable;
use cursive::view::Resizable;
use cursive::views::{Canvas, Layer};
use cursive::{Cursive, Printer, Rect, Vec2};

const PG_DATABASE: &'static str = "protocoldb";
const PG_HOST: &'static str = "localhost";
const PG_USER: &'static str = "postgres";
const PG_PASSW: &'static str = "postgrespw";
const PG_PORT: u32 = 49153;

struct CData {
    params: PgsqlDbConnectionParams,
    timer_start: Option<SystemTime>,
}
impl CData {
    fn new(params: PgsqlDbConnectionParams) -> Self {
        CData {
            params,
            timer_start: None,
        }
    }
}

struct Cell {
    name: String,
    progress_dkg: f64,
    progress_mix: f64,
    last_kind: String,
}
impl Cell {
    fn new(name: String, progress_dkg: f64, progress_mix: f64, last_kind: String) -> Self {
        Cell {
            name,
            progress_dkg,
            progress_mix,
            last_kind,
        }
    }
}

struct Data {
    pub rows: Vec<Cell>,
    pub duration: Duration,
    pub total_messages: i32,
    pub max_messages: i32,
}
impl Data {
    fn new() -> Self {
        Self {
            duration: Duration::ZERO,
            rows: vec![],
            total_messages: 0,
            max_messages: 0,
        }
    }
}

#[derive(Parser)]
struct Cli {
    #[arg(long, default_value_t = PG_HOST.to_string())]
    host: String,

    #[arg(long, default_value_t = PG_PORT)]
    port: u32,

    #[arg(short, long, default_value_t = PG_USER.to_string())]
    username: String,

    #[arg(long, default_value_t = PG_PASSW.to_string())]
    password: String,

    #[arg(long, default_value_t = PG_DATABASE.to_string())]
    database: String,
}

fn main() {
    let args = Cli::parse();
    let params = PgsqlConnectionParams::new(&args.host, args.port, &args.username, &args.password);
    let params = params.with_database(&args.database);

    let mut siv = cursive::default();

    let data = Data::new();
    let canvas = Canvas::new(data).with_draw(draw).with_name("Canvas");
    let mut layer = Layer::new(canvas);
    let style = ColorStyle::new(BaseColor::White, BaseColor::Black);
    layer.set_color(style);

    siv.add_layer(layer.full_screen());
    let mut theme = Theme::terminal_default();
    theme.borders = BorderStyle::None;
    siv.set_theme(theme);
    siv.add_global_callback('q', |s| s.quit());
    siv.add_global_callback('t', timer);
    siv.set_fps(1);

    // detect db connection errors early
    q(&params).unwrap();

    siv.add_global_callback(Event::Refresh, step);
    siv.set_user_data(CData::new(params));

    siv.run();
}

fn timer(c: &mut Cursive) {
    let cdata: &mut CData = c.user_data().unwrap();
    cdata.timer_start = Some(SystemTime::now())
}

fn step(c: &mut Cursive) {
    let cdata: &CData = c.user_data().unwrap();
    let rows = if let Ok(rows) = q(&cdata.params) {
        rows
    } else {
        vec![]
    };

    let mut total_messages = 0;
    let mut max_messages = 0;
    let mut cells: Vec<Cell> = vec![];

    for r in rows {
        let (dkg, mix) = get_max_values(&r);
        let (progress_dkg, progress_mix) = get_progress(&r);
        max_messages = max_messages + (mix * &r.batch_count);
        total_messages = total_messages + &r.message_count;
        if r.batch_count == 0 {
            max_messages = max_messages + dkg;
        } else {
            total_messages = total_messages - dkg;
        }

        let cell = Cell::new(
            r.board_name,
            progress_dkg,
            progress_mix,
            r.last_message_kind,
        );
        cells.push(cell);
    }
    let timer_start = cdata.timer_start.clone();

    let mut canvas = c.find_name::<Canvas<Data>>("Canvas").unwrap();
    let state = canvas.state_mut();

    state.rows = cells;
    state.total_messages = total_messages;
    state.max_messages = max_messages;

    if let Some(start) = timer_start {
        if total_messages != max_messages {
            state.duration = start.elapsed().unwrap();
        }
    }
}

fn draw(data: &Data, p: &Printer) {
    let bar_gray = ColorStyle::new(BaseColor::White, Color::from_256colors(243));
    let bar_white = ColorStyle::new(BaseColor::White, Color::from_256colors(255));
    let bar_text = ColorStyle::new(BaseColor::Black, Color::from_256colors(255));

    let cells = &data.rows;
    let len = cells.len();

    if len == 0 {
        p.print((p.size.x / 2, p.size.y / 2), "No data yet..");
        return;
    }

    let (width, height) = get_cell_counts(cells.len());
    let cell_counts = Vec2::new(width, height);
    // progress
    let psize_y = p.size.y - 1;

    let mut x_offset = 0;
    if p.size.x > cell_counts.x {
        x_offset = (p.size.x % cell_counts.x) / 2;
    }

    let mut y_offset = 1;
    // if p.size.y > cell_counts.y {
    if psize_y > cell_counts.y {
        // max(1) required for progress
        // y_offset = ((p.size.y % cell_counts.y) / 2).max(1);
        y_offset = ((psize_y % cell_counts.y) / 2).max(1);
    }

    // let cell_height = (p.size.y / cell_counts.y).max(1);
    let cell_height = (psize_y / cell_counts.y).max(1);
    let cell_width = (p.size.x / cell_counts.x).max(1);
    let cell_size = Vec2::new(cell_width, cell_height);

    let mut x = x_offset;
    let mut y = y_offset;

    let mut index = 0;

    let total_messages = data.total_messages;
    let max_messages = data.max_messages;

    for _i in 0..cell_counts.y {
        for _j in 0..cell_counts.x {
            draw_cell(
                p,
                &Vec2::new(x, y),
                &cell_size,
                &cells[index],
                cells.len() < 300,
            );

            // Next column
            x = x + cell_width;
            index = index + 1;

            if index == len {
                let text = format!("{} / {}", total_messages, max_messages);
                let progress = f64::from(total_messages) / f64::from(max_messages);

                let w = cell_width * width;
                let progress = progress * f64::from(w as u32);
                let progress = progress.round() as usize;

                p.with_color(bar_gray, |printer| {
                    printer.print_hline((x_offset, y_offset - 1), w, " ");
                });
                p.with_color(bar_white, |printer| {
                    printer.print_hline((x_offset, y_offset - 1), progress, " ");
                });
                p.with_color(bar_text, |printer| {
                    printer.print((x_offset, y_offset - 1), &text[0..text.len().min(progress)]);
                });

                if data.duration != Duration::ZERO {
                    let text = data.duration.as_secs().to_string();
                    let text_length = text.len().min(w);

                    p.with_color(bar_gray, |printer| {
                        printer.print((w - text_length, y_offset - 1), &text[0..text_length]);
                    });
                }

                return;
            }
        }

        // Next row
        y = y + cell_height;
        x = x_offset;
    }

    // Can never reach here!
}

fn draw_cell(p: &Printer, origin: &Vec2, size: &Vec2, data: &Cell, draw_text: bool) {
    let f_black = ColorStyle::new(BaseColor::White, BaseColor::Black);
    let b_gray = ColorStyle::new(BaseColor::White, Color::from_256colors(236));
    let b_magenta = ColorStyle::new(BaseColor::Black, BaseColor::Magenta);
    let b_blue = ColorStyle::new(BaseColor::Black, BaseColor::Blue);
    let b_green = ColorStyle::new(BaseColor::Black, BaseColor::Green);

    // Background
    p.with_color(b_gray, |printer| {
        printer.print_rect(Rect::from_size(origin, size), " ");
    });

    let (dkg, mix) = (data.progress_dkg, data.progress_mix);
    let mut draw_kind = false;

    let mut kind_origin_y = if mix == 0.0 {
        // Dkg
        let bar_height = (dkg * f64::from(size.y as u32)).round() as usize;
        let bar_origin_y = origin.y + (size.y - bar_height);
        let bar_origin_y = bar_origin_y.max(0);

        let bar_origin = Vec2::new(origin.x, bar_origin_y);
        let bar_size = Vec2::new(size.x, bar_height);

        let color = if dkg == 1.0 { b_green } else { b_blue };

        if bar_height > 0 {
            p.with_color(color, |printer| {
                printer.print_rect(Rect::from_size(bar_origin, bar_size), " ");
            });
            if dkg != 1.0 {
                draw_kind = true;
            };
        }
        bar_origin_y
    } else {
        // Mix
        let bar_height = (mix * f64::from(size.y as u32)).round() as usize;
        let bar_origin_y = origin.y + (size.y - bar_height);
        let bar_origin_y = bar_origin_y.max(0);

        let bar_origin = Vec2::new(origin.x, bar_origin_y);
        let bar_size = Vec2::new(size.x, bar_height);

        let color = if mix == 1.0 { b_green } else { b_magenta };

        if bar_height > 0 {
            p.with_color(color, |printer| {
                printer.print_rect(Rect::from_size(bar_origin, bar_size), " ");
            });
            if mix != 1.0 {
                draw_kind = true;
            };
        }
        bar_origin_y
    };

    if draw_text {
        // Kind
        if draw_kind {
            let text_color = if mix > 0.0 {
                b_magenta
            } else if dkg > 0.0 {
                b_blue
            } else {
                f_black
            };

            if kind_origin_y == origin.y && size.y > 1 {
                kind_origin_y = kind_origin_y + 1;
            }

            let title = format!("{}", &data.last_kind);
            let max_chars = title.len().min(size.x - 2);

            p.with_color(text_color, |printer| {
                printer.print((origin.x + 1, kind_origin_y), &title[0..max_chars]);
                // printer.print(origin, &blue.to_string());
            });
        }

        // Title

        let name = &data.name;

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
        });
    }
}

/*
n trustees
t threshold
b batches

DKG phase: 1 + 5n
                     ballot  mix     mix signature     decrypt factors    plaintext + sig
Tally phase:    b * (1 +     t +    (t * (t - 1)) +    t +                 n)
= b * (n + (t * t + 1) + 1)

*/
fn get_max_values(row: &B3IndexRow) -> (i32, i32) {
    let dkg_max = 1 + (5 * row.trustees_no);
    let mix_max =
        1 + (2 * row.threshold_no) + (row.threshold_no * (row.threshold_no - 1)) + row.trustees_no;

    (dkg_max, mix_max)
}

fn get_progress(row: &B3IndexRow) -> (f64, f64) {
    let (dkg_max, mix_max) = get_max_values(row);

    let tally_size = f64::from(mix_max);

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

async fn query(params: &PgsqlDbConnectionParams) -> Result<Vec<B3IndexRow>> {
    let client = PgsqlB3Client::new(&params).await?;
    client.get_boards().await
}

fn q(params: &PgsqlDbConnectionParams) -> Result<Vec<B3IndexRow>> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    let inner = rt.block_on(query(params))?;

    Ok(inner)
}

fn get_cell_counts(len: usize) -> (usize, usize) {
    // let len = rows.len();
    let lenrt = (len as f64).sqrt() as usize;

    let width = if len % lenrt == 0 && len / lenrt == lenrt {
        lenrt
    } else {
        lenrt + 1
    };

    let height = if len % width == 0 {
        len / width
    } else {
        1 + (len / width)
    };

    (width, height)
}

}
else {
    fn main() {
        println!("Requires the 'monitor' feature");
    }
}}
