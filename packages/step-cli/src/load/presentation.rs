// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Portable HTML and SVG built from aggregate data, with no plotting runtime.
use super::{
    input::Input,
    report::{quantiles, Stage, Summary},
};
use anyhow::Result;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use rusqlite::{params, Connection};
use std::{collections::BTreeMap, fmt::Write as _, path::Path};

/// Escape all untrusted text before inserting it into HTML or SVG markup.
pub fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
fn number(value: Option<f64>, unit: &str) -> String {
    value
        .map(|value| format!("{value:.1} {unit}"))
        .unwrap_or_else(|| "—".into())
}

/// Draw two responsive charts with a bounded number of points and time buckets.
fn chart(db: &Connection, input: &Input, summary: &Summary) -> Result<String> {
    // SVG coordinates are visual layout, not sampling limits or performance targets.
    const WIDTH: f64 = 1100.0;
    const HEIGHT: f64 = 290.0;
    const PLOT_WIDTH: f64 = 440.0;
    const TOP: f64 = 20.0;
    const BOTTOM: f64 = 225.0;
    const LEFT: f64 = 65.0;
    const RIGHT: f64 = 630.0;
    let mut svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {WIDTH} {HEIGHT}" role="img" aria-label="Latency distribution and throughput over time"><style>text{{font:12px sans-serif;fill:#607680}}.grid{{stroke:#dce5e8;stroke-width:1}}</style>"##
    );
    let points = input.settings.reporting.cdf_points;
    let mut curves = Vec::new();
    for (stage, color) in [(Stage::Status, "#087f76"), (Stage::Cast, "#4576b8")] {
        let fractions: Vec<_> = (0..points)
            .map(|index| index as f64 / (points - 1) as f64)
            .collect();
        let curve: Vec<_> = quantiles(db, stage, &fractions)?
            .into_iter()
            .zip(fractions)
            .filter_map(|(value, fraction)| value.map(|value| (value, fraction)))
            .collect();
        curves.push((stage, color, curve));
    }
    let max_latency = curves
        .iter()
        .flat_map(|(_, _, curve)| curve.iter().map(|(value, _)| *value))
        .fold(1.0_f64, f64::max);
    for tick in 0..=4 {
        let fraction = tick as f64 / 4.0;
        let y = BOTTOM - fraction * (BOTTOM - TOP);
        write!(
            svg,
            r##"<path class="grid" d="M{LEFT},{y}h{PLOT_WIDTH}"/><text x="55" y="{}" text-anchor="end">{:.0}%</text><text x="{}" y="245" text-anchor="middle">{:.0}</text>"##,
            y + 4.0,
            fraction * 100.0,
            LEFT + fraction * PLOT_WIDTH,
            fraction * max_latency
        )?;
    }
    for (index, (stage, color, curve)) in curves.iter().enumerate() {
        let coordinates = curve
            .iter()
            .map(|(value, fraction)| {
                format!(
                    "{:.2},{:.2}",
                    LEFT + value / max_latency * PLOT_WIDTH,
                    BOTTOM - fraction * (BOTTOM - TOP)
                )
            })
            .collect::<Vec<_>>()
            .join(" ");
        write!(
            svg,
            r##"<polyline fill="none" stroke="{color}" stroke-width="2.5" points="{coordinates}"/><text x="{}" y="286" style="fill:{color}">{}</text>"##,
            LEFT + index as f64 * 170.0,
            stage.label()
        )?;
    }
    let status = summary.mode == "status";
    let mut buckets = BTreeMap::<usize, usize>::new();
    let mut bucket_count = 1;
    let mut width_ms = 1.0;
    if summary.completed > 0 {
        let (start, end): (f64, f64) =
            db.query_row("SELECT min(start),max(end) FROM samples", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?;
        bucket_count = input
            .settings
            .reporting
            .bins
            .min(((end - start) / 1000.0).ceil().max(1.0) as usize);
        width_ms = (end - start).max(1.0) / bucket_count as f64;
        let predicate = if status {
            "passed = 1"
        } else {
            "receipt IS NOT NULL"
        };
        let mut statement = db.prepare(&format!("SELECT min(cast((end-?)/? AS INTEGER),?),count(*) FROM samples WHERE {predicate} GROUP BY 1"))?;
        buckets = statement
            .query_map(params![start, width_ms, bucket_count - 1], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?
            .collect::<rusqlite::Result<_>>()?;
    }
    let max_rate = buckets
        .values()
        .map(|count| *count as f64 * 1000.0 / width_ms)
        .fold(1.0_f64, f64::max);
    for tick in 0..=4 {
        let fraction = tick as f64 / 4.0;
        let y = BOTTOM - fraction * (BOTTOM - TOP);
        write!(
            svg,
            r##"<path class="grid" d="M{RIGHT},{y}h{PLOT_WIDTH}"/><text x="620" y="{}" text-anchor="end">{:.1}</text><text x="{}" y="245" text-anchor="middle">{:.1}</text>"##,
            y + 4.0,
            fraction * max_rate,
            RIGHT + fraction * PLOT_WIDTH,
            fraction * width_ms * bucket_count as f64 / 1000.0
        )?;
    }
    for (index, count) in buckets {
        let bar = PLOT_WIDTH / bucket_count as f64;
        let height = count as f64 * 1000.0 / width_ms / max_rate * (BOTTOM - TOP);
        write!(
            svg,
            r##"<rect x="{}" y="{}" width="{}" height="{height}" fill="#087f76"/>"##,
            RIGHT + (index as f64 + 0.075) * bar,
            BOTTOM - height,
            bar * 0.85
        )?;
    }
    write!(
        svg,
        r##"<text x="285" y="267" text-anchor="middle">Response time · milliseconds</text><text x="850" y="267" text-anchor="middle">Elapsed time · seconds</text><text x="{RIGHT}" y="286">{}</text></svg>"##,
        if status {
            "Successful journeys / s"
        } else {
            "Accepted casts / s"
        }
    )?;
    Ok(svg)
}

/// Render only outcomes and performance; identifiers and HTTP inventories stay private.
pub fn render(directory: &Path, db: &Connection, input: &Input, result: &Summary) -> Result<()> {
    let svg = chart(db, input, result)?;
    std::fs::write(directory.join("performance.svg"), &svg)?;
    let status = result.mode == "status";
    let verdict = if result.errors.is_empty() {
        "Passed"
    } else {
        "Failed"
    };
    let mut latency_rows = String::new();
    for stage in Stage::ALL {
        let values = &result.latency[stage.column()];
        if values["p50"].is_some() {
            write!(
                latency_rows,
                r#"<tr><td>{}</td><td class="num">{}</td><td class="num">{}</td></tr>"#,
                stage.label(),
                number(values["p50"], "ms"),
                number(values["p99"], "ms")
            )?;
        }
    }
    let throughput = if status && result.elapsed_seconds > 0.0 {
        result.passed as f64 / result.elapsed_seconds
    } else {
        result.casts_per_second
    };
    let cards = [
        ("Successful journeys",format!("{} / {}",result.passed,result.planned),"Distinct synthetic voters".into()),
        (if status {"Journeys / second"} else {"Accepted casts / second"},format!("{throughput:.2}"),"Across the complete measured interval".into()),
        ("Journey p99",number(result.latency["journey_ms"]["p99"],"ms"),"Login through final response".into()),
        ("Measured duration",number(Some(result.elapsed_seconds),"s"),format!("{} missing or failed journeys",result.planned.saturating_sub(result.passed))),
    ].into_iter().map(|(label,value,note)| format!(r#"<div class="card"><div class="label">{label}</div><div class="value">{value}</div><div class="note">{note}</div></div>"#)).collect::<String>();
    let mut goals = String::new();
    let mut goal_row = |name: &str, target: String, passed: bool| {
        write!(
            goals,
            r#"<tr><td>{}</td><td class="num">{target}</td><td class="num {}">{}</td></tr>"#,
            escape(name),
            if passed { "pass" } else { "fail" },
            if passed { "Passed" } else { "Missed" }
        )
        .expect("Writing to String cannot fail");
    };
    for stage in Stage::ALL {
        if let Some(limits) = input.settings.goals.get(stage.column()) {
            for (quantile, limit) in limits {
                goal_row(
                    &format!("{} {quantile}", stage.label()),
                    format!("≤ {limit} ms"),
                    result.latency[stage.column()][quantile].is_some_and(|actual| actual <= *limit),
                );
            }
        }
    }
    if input.settings.min_casts_per_second > 0.0 {
        goal_row(
            "Accepted casts / s",
            format!("≥ {}", input.settings.min_casts_per_second),
            result.casts_per_second >= input.settings.min_casts_per_second,
        );
    }
    let goals = if goals.is_empty() {
        "<p>No latency or throughput thresholds configured.</p>".into()
    } else {
        format!(
            r#"<table><thead><tr><th>Measure</th><th class="num">Target</th><th class="num">Result</th></tr></thead><tbody>{goals}</tbody></table>"#
        )
    };
    let goals = format!(
        "{goals}<p class=\"note\">Every planned journey must succeed{}.</p>",
        if status {
            ""
        } else {
            " and return a unique ballot receipt"
        }
    );
    let errors = if result.errors.is_empty() {
        String::new()
    } else {
        format!(
            "<section class=\"errors\"><strong>Needs attention</strong><ul>{}</ul></section>",
            result
                .errors
                .iter()
                .map(|error| format!("<li>{}</li>", escape(error)))
                .collect::<String>()
        )
    };
    let browser = matches!(result.engine, super::Engine::Chromium);
    let mut html = include_str!("../../../voting-load/report.html").to_owned();
    for (name,value) in [
        ("verdict",verdict.to_owned()),("title",if status {"Voter status performance"} else {"Voting journey performance"}.into()),
        ("subtitle",format!("{} · {} concurrent voters per worker",if browser {"Chromium · Browser journey"} else {"k6 · HTTP journey"},input.settings.workload.concurrency)),
        ("badge_background",if result.errors.is_empty() {"#e5f4ee"} else {"#fce9e4"}.into()),
        ("badge_color",if result.errors.is_empty() {"#14704b"} else {"#a6372c"}.into()),
        ("cards",cards),("chart",svg),("latency_rows",latency_rows),("goals",goals),("errors",errors),
        ("coverage",if browser {"Includes login, rendering, ballot encryption and cast acceptance."} else if status {"Includes login and voter-status responses."} else {"Includes login, voter status, publication downloads and cast acceptance. Encryption is prepared beforehand."}.into()),
        ("verification",escape(&result.persistence_verification)),
        ("regular_font",STANDARD.encode(include_bytes!("../../../admin-portal/public/roboto/Roboto_latin_400.woff2"))),
        ("bold_font",STANDARD.encode(include_bytes!("../../../admin-portal/public/roboto/Roboto_latin_700.woff2"))),
        ("font_license",escape(concat!("Roboto Copyright 2015 Google Inc.\n",include_str!("../../../../LICENSES/Apache-2.0.txt")))),
    ] { html=html.replace(&format!("${name}"),&value); }
    std::fs::write(directory.join("report.html"), html)?;
    std::fs::write(directory.join("performance.md"),format!("# Voting performance · {verdict}\n\n{}/{} successful journeys.\n\n![Performance](performance.svg)\n",result.passed,result.planned))?;
    Ok(())
}
