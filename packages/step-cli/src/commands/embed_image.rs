// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Put an image a workbook names by an Internet address into the workbook.
//!
//! A site with no Internet shows an empty space where a page's image is named
//! by an Internet address (a sign-in page's CSS, an event's `logo_url`). The
//! workbook is one of the files such a site is given, so the image travels in
//! it, as a `data:` address. The work is `sequent_core`'s
//! `election_config::page_images::embed_image`; this reads and writes files.

use anyhow::{anyhow, Context, Result};
use clap::Args;
use colored::Colorize;
use sequent_core::election_config::page_images::{check, embed_image};
use sequent_core::election_config::xlsx::read_xlsx;
use sequent_core::election_config::xlsx_write::write_xlsx;
use std::fs;
use std::path::PathBuf;

/// Embed an image a workbook names by an Internet address, as a data: address
#[derive(Args)]
#[command(about)]
pub struct EmbedImage {
    /// The workbook to read
    #[arg(short = 'w', long, value_name = "WORKBOOK")]
    workbook: PathBuf,

    /// The address the workbook names the image by, exactly as written there
    #[arg(short = 'a', long, value_name = "URL")]
    address: String,

    /// The image file (PNG, JPEG, GIF, WebP or SVG, at most 1 MiB): the one
    /// at that address, downloaded and looked at beforehand
    #[arg(short = 'i', long, value_name = "FILE")]
    image: PathBuf,

    /// Where to write the new workbook. Never the one read.
    #[arg(short = 'o', long, value_name = "OUT")]
    out: PathBuf,
}

impl EmbedImage {
    pub fn run(&self) {
        if let Err(error) = self.embed() {
            eprintln!("{} {error:#}", "error:".red().bold());
            std::process::exit(1);
        }
    }

    fn embed(&self) -> Result<()> {
        let same = match (
            fs::canonicalize(&self.workbook),
            fs::canonicalize(&self.out),
        ) {
            (Ok(a), Ok(b)) => a == b,
            _ => false,
        };
        if same {
            return Err(anyhow!(
                "--out is the workbook being read; write a new file"
            ));
        }
        let bytes = fs::read(&self.workbook)
            .with_context(|| format!("could not read {}", self.workbook.display()))?;
        let image = fs::read(&self.image)
            .with_context(|| format!("could not read {}", self.image.display()))?;
        let workbook = read_xlsx(&bytes).map_err(|p| anyhow!("{}: {}", p.path, p.message))?;
        let (embedded, replaced) = embed_image(&workbook, &self.address, &image)
            .map_err(|p| anyhow!("{}: {}", p.path, p.message))?;
        let written = write_xlsx(&embedded).map_err(|p| anyhow!("{}: {}", p.path, p.message))?;
        fs::write(&self.out, written)
            .with_context(|| format!("could not write {}", self.out.display()))?;
        println!(
            "{} {} mention(s) of {} now carry {}; wrote {}",
            "embedded:".green().bold(),
            replaced,
            self.address,
            self.image.display(),
            self.out.display()
        );
        // What a site with no Internet would still miss.
        for problem in check(&embedded) {
            println!(
                "{} {}: {}",
                "warning:".yellow().bold(),
                problem.path,
                problem.message
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequent_core::election_config::sheet::{Sheet, Workbook};
    use sequent_core::election_config::Cell;
    use std::path::Path;

    const SEAL: &str = "https://images.example.org/seal.svg";
    const FONT: &str = "https://fonts.example.org/a.woff2";
    // A synthetic image: no client file is in the repository.
    const SVG: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg"><circle r="1"/></svg>"#;

    /// A workbook whose event presentation names the image and a font.
    fn write_workbook(path: &Path) {
        let presentation = serde_json::json!({
            "logo_url": SEAL,
            "css": format!("@font-face {{ src: url({FONT}); }}"),
        });
        let grid = vec![
            vec![Cell::text("type"), Cell::text("key"), Cell::text("value")],
            vec![
                Cell::text("event"),
                Cell::text("election_event.presentation"),
                Cell::text(presentation.to_string()),
            ],
        ];
        let workbook = Workbook::new(vec![Sheet::from_grid("Parameters", &grid).unwrap()]).unwrap();
        fs::write(path, write_xlsx(&workbook).unwrap()).unwrap();
    }

    fn command(dir: &Path, address: &str, out: &str) -> EmbedImage {
        EmbedImage {
            workbook: dir.join("in.xlsx"),
            address: address.to_string(),
            image: dir.join("seal.svg"),
            out: dir.join(out),
        }
    }

    fn setup() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        write_workbook(&dir.path().join("in.xlsx"));
        fs::write(dir.path().join("seal.svg"), SVG).unwrap();
        dir
    }

    #[test]
    fn the_image_is_embedded_in_a_new_workbook() {
        let dir = setup();
        command(dir.path(), SEAL, "out.xlsx").embed().unwrap();
        let out = read_xlsx(&fs::read(dir.path().join("out.xlsx")).unwrap()).unwrap();
        // Only the font is left to warn about; the image travels inside.
        let left: Vec<String> = check(&out)
            .into_iter()
            .map(|p| p.details["address"].to_string())
            .collect();
        assert_eq!(left, [FONT]);
        let original = read_xlsx(&fs::read(dir.path().join("in.xlsx")).unwrap()).unwrap();
        assert_eq!(check(&original).len(), 2);
    }

    #[test]
    fn run_writes_the_workbook_when_it_can() {
        let dir = setup();
        command(dir.path(), SEAL, "run.xlsx").run();
        assert!(dir.path().join("run.xlsx").is_file());
    }

    #[test]
    fn the_workbook_read_is_never_overwritten() {
        let dir = setup();
        let error = command(dir.path(), SEAL, "in.xlsx").embed().unwrap_err();
        assert!(
            error.to_string().contains("--out is the workbook"),
            "{error}"
        );
    }

    #[test]
    fn files_that_cannot_be_read_or_used_are_refused() {
        let dir = setup();
        let mut missing = command(dir.path(), SEAL, "out.xlsx");
        missing.image = dir.path().join("absent.svg");
        let error = format!("{:#}", missing.embed().unwrap_err());
        assert!(error.contains("could not read"), "{error}");

        let mut no_workbook = command(dir.path(), SEAL, "out.xlsx");
        no_workbook.workbook = dir.path().join("absent.xlsx");
        let error = format!("{:#}", no_workbook.embed().unwrap_err());
        assert!(error.contains("could not read"), "{error}");

        let mut not_xlsx = command(dir.path(), SEAL, "out.xlsx");
        not_xlsx.workbook = dir.path().join("seal.svg");
        assert!(not_xlsx.embed().is_err());

        let unnamed = command(dir.path(), "https://other.example/x.png", "out.xlsx");
        let error = unnamed.embed().unwrap_err().to_string();
        assert!(error.contains("nothing in the workbook names"), "{error}");

        let unwritable = command(dir.path(), SEAL, "no-such-dir/out.xlsx");
        let error = format!("{:#}", unwritable.embed().unwrap_err());
        assert!(error.contains("could not write"), "{error}");
        assert!(!dir.path().join("out.xlsx").exists());
    }
}
