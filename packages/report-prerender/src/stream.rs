// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! Streaming PDF object writer: retain offsets/page IDs, never rendered pages.
use anyhow::{bail, Result};
use lopdf::{Dictionary, Object, ObjectId};
use std::io::{Seek, Write};

pub struct PdfStream<W> {
    writer: W,
    offsets: Vec<Option<(u64, u16)>>,
    next: u32,
}
impl<W: Write + Seek> PdfStream<W> {
    pub fn new(mut writer: W, max_id: u32) -> Result<Self> {
        writer.write_all(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n")?;
        Ok(Self {
            writer,
            offsets: vec![None; max_id as usize + 1],
            next: max_id + 1,
        })
    }
    pub fn allocate(&mut self) -> ObjectId {
        let id = self.next;
        self.next += 1;
        self.offsets.push(None);
        (id, 0)
    }
    pub fn object(&mut self, id: ObjectId, object: &Object) -> Result<()> {
        if self.offsets[id.0 as usize].is_some() {
            bail!("PDF object written twice");
        }
        self.offsets[id.0 as usize] = Some((self.writer.stream_position()?, id.1));
        writeln!(self.writer, "{} {} obj", id.0, id.1)?;
        write_object(&mut self.writer, object)?;
        self.writer.write_all(b"\nendobj\n")?;
        Ok(())
    }
    pub fn finish(mut self, root: ObjectId) -> Result<W> {
        let start = self.writer.stream_position()?;
        writeln!(self.writer, "xref\n0 {}", self.offsets.len())?;
        self.writer.write_all(b"0000000000 65535 f \n")?;
        for entry in self.offsets.iter().skip(1) {
            match entry {
                Some((offset, generation)) => {
                    if *offset > 9_999_999_999 {
                        bail!("PDF exceeds 10 GB");
                    }
                    writeln!(self.writer, "{offset:010} {generation:05} n ")?;
                }
                None => self.writer.write_all(b"0000000000 00000 f \n")?,
            }
        }
        writeln!(
            self.writer,
            "trailer\n<< /Size {} /Root {} {} R >>\nstartxref\n{start}\n%%EOF",
            self.offsets.len(),
            root.0,
            root.1
        )?;
        self.writer.flush()?;
        Ok(self.writer)
    }
}
fn name(w: &mut impl Write, value: &[u8]) -> Result<()> {
    w.write_all(b"/")?;
    for b in value {
        if !(33..=126).contains(b) || b"()<>[]{}/%#".contains(b) {
            write!(w, "#{b:02X}")?;
        } else {
            w.write_all(&[*b])?;
        }
    }
    Ok(())
}
fn dictionary(w: &mut impl Write, dict: &Dictionary) -> Result<()> {
    w.write_all(b"<<")?;
    for (key, value) in dict {
        w.write_all(b" ")?;
        name(w, key)?;
        w.write_all(b" ")?;
        write_object(w, value)?;
    }
    w.write_all(b" >>")?;
    Ok(())
}
fn write_object(w: &mut impl Write, object: &Object) -> Result<()> {
    match object {
        Object::Null => w.write_all(b"null")?,
        Object::Boolean(v) => w.write_all(if *v { b"true" } else { b"false" })?,
        Object::Integer(v) => write!(w, "{v}")?,
        Object::Real(v) => {
            if !v.is_finite() {
                bail!("Non-finite PDF number");
            }
            write!(w, "{v}")?;
        }
        Object::Name(v) => name(w, v)?,
        Object::String(v, _) => {
            w.write_all(b"<")?;
            for b in v {
                write!(w, "{b:02X}")?;
            }
            w.write_all(b">")?;
        }
        Object::Reference((id, generation)) => write!(w, "{id} {generation} R")?,
        Object::Array(v) => {
            w.write_all(b"[")?;
            for o in v {
                write_object(w, o)?;
                w.write_all(b" ")?;
            }
            w.write_all(b"]")?;
        }
        Object::Dictionary(v) => dictionary(w, v)?,
        Object::Stream(v) => {
            let mut dict = v.dict.clone();
            dict.set("Length", v.content.len() as i64);
            dictionary(w, &dict)?;
            w.write_all(b"\nstream\n")?;
            w.write_all(&v.content)?;
            w.write_all(b"\nendstream")?;
        }
    }
    Ok(())
}
