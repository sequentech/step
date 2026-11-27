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
