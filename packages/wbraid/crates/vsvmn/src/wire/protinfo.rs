// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Reading `protInfo.xml`, the protocol info file.
//!
//! Every parameter a verifier needs about a session lives here rather than in
//! the proof directory: the group, the hash functions, the bit lengths, the
//! party count and threshold. Above all it is where ρ comes from — the prefix
//! every random-oracle query is salted with (VMNV §9.3 step 4) — so getting a
//! single field wrong makes every challenge differ and every proof appear
//! invalid.
//!
//! That is why these are read rather than assumed. Before this existed the
//! session parameters were `const`s repeated across the test files, which meant
//! a proof could only be checked if it happened to have been produced with the
//! same `sid`, group and widths.
//!
//! # Parsing deliberately narrowly
//!
//! This is not a general XML parser and should not become one. The format is a
//! flat list of elements followed by repeated `<party>` blocks, and a verifier
//! reading it wants to be *unsurprising*: it extracts a fixed set of top-level
//! fields and refuses anything ambiguous. Concretely:
//!
//! - **Comments are stripped first.** They are not decorative — the shipped
//!   files contain `<hostname>:<port>` inside one, so a naive scan for
//!   `<tag>…</tag>` can match text that is not an element at all.
//! - **Only the region before the first `<party>` is searched**, because
//!   `<name>` and `<descr>` occur at both levels.
//! - **A field must occur exactly once.** Absent is an error; repeated is an
//!   error rather than first-wins, since a duplicate means the file is not what
//!   we think it is.
//!
//! Verificatum's own files warn that "many XML features are disabled and throw
//! errors, so parsing is more restrictive than the schema implies" — the same
//! spirit applies here.

use crate::wire::crypto::PrefixParams;
use crate::wire::error::{Error, Result};

/// The session parameters carried by a protocol info file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProtocolInfo {
    /// VMN version this session was run with; a proof must match it.
    pub version: String,
    /// Session identifier (`<sid>`).
    pub sid: String,
    /// Number of parties `k` (`<nopart>`).
    pub parties: usize,
    /// Threshold `λ` needed to decrypt (`<thres>`).
    pub threshold: usize,
    /// Default ciphertext width `ω` (`<width>`); a session may override it.
    pub width: usize,
    /// El Gamal key width `κ` (`<keywidth>`).
    pub key_width: usize,
    /// `n_r`, the statistical distance parameter (`<statdist>`).
    pub n_r: u32,
    /// `n_v`, challenge bit length (`<vbitlenro>`).
    pub n_v: u32,
    /// `n_e`, batching-component bit length (`<ebitlenro>`).
    pub n_e: u32,
    /// PRG name (`<prg>`).
    pub prg: String,
    /// Random-oracle hash name (`<rohash>`).
    pub rohash: String,
    /// The **full** marshalled group string from `<pgroup>`, comment prefix
    /// included — ρ commits to it verbatim, so it must not be normalised.
    pub pgroup: String,
}

impl ProtocolInfo {
    /// Parse a protocol info file.
    ///
    /// # Errors
    ///
    /// - `BadProtocolInfo` if a required field is missing, repeated, or not the
    ///   expected kind of value.
    pub fn parse(xml: &str) -> Result<Self> {
        let body = top_level(xml);

        Ok(ProtocolInfo {
            version: field(&body, "version")?,
            sid: field(&body, "sid")?,
            parties: number(&body, "nopart")?,
            threshold: number(&body, "thres")?,
            width: number(&body, "width")?,
            key_width: number(&body, "keywidth")?,
            n_r: number(&body, "statdist")? as u32,
            n_v: number(&body, "vbitlenro")? as u32,
            n_e: number(&body, "ebitlenro")? as u32,
            prg: field(&body, "prg")?,
            rohash: field(&body, "rohash")?,
            pgroup: field(&body, "pgroup")?,
        })
    }

    /// The random-oracle prefix parameters for a session of this protocol.
    ///
    /// `auxsid` is the one input that does *not* come from here — it identifies
    /// a session within the protocol and is read from the proof directory's
    /// `auxsid` file (`"default"` unless set explicitly).
    #[must_use]
    pub fn prefix_params(&self, auxsid: &str) -> PrefixParams {
        PrefixParams {
            version: self.version.clone(),
            sid: self.sid.clone(),
            auxsid: auxsid.to_string(),
            n_r: self.n_r,
            n_v: self.n_v,
            n_e: self.n_e,
            prg: self.prg.clone(),
            pgroup: self.pgroup.clone(),
            rohash: self.rohash.clone(),
        }
    }

    /// Whether this file describes a threshold every party could meet.
    ///
    /// VMN's own generator enforces it; a verifier should not assume it holds
    /// of a file it was handed.
    #[must_use]
    pub fn is_consistent(&self) -> bool {
        self.threshold >= 1 && self.threshold <= self.parties && self.width >= 1
    }
}

/// The document with comments removed and everything from the first `<party>`
/// onwards discarded.
fn top_level(xml: &str) -> String {
    let mut out = String::with_capacity(xml.len());
    let mut rest = xml;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        match rest[start..].find("-->") {
            // An unterminated comment swallows the remainder, which is what a
            // conforming parser would do.
            None => return truncate_at_parties(&out),
            Some(end) => rest = &rest[start + end + 3..],
        }
    }
    out.push_str(rest);
    truncate_at_parties(&out)
}

fn truncate_at_parties(xml: &str) -> String {
    match xml.find("<party>") {
        Some(at) => xml[..at].to_string(),
        None => xml.to_string(),
    }
}

/// The text content of the single `<tag>…</tag>` element.
fn field(body: &str, tag: &str) -> Result<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");

    let mut found: Option<&str> = None;
    let mut rest = body;
    while let Some(start) = rest.find(&open) {
        let after = &rest[start + open.len()..];
        let end = after
            .find(&close)
            .ok_or(Error::BadProtocolInfo("unclosed element"))?;
        if found.is_some() {
            return Err(Error::BadProtocolInfo("element occurs more than once"));
        }
        found = Some(&after[..end]);
        rest = &after[end + close.len()..];
    }

    found
        .map(|value| value.trim().to_string())
        .ok_or(Error::BadProtocolInfo("required element is missing"))
}

fn number(body: &str, tag: &str) -> Result<usize> {
    field(body, tag)?
        .parse()
        .map_err(|_| Error::BadProtocolInfo("element is not a non-negative integer"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
<!-- ATTENTION! Do not edit. A comment may itself contain markup such as
     <hostname>:<port>, or even <sid>decoy</sid>, which a naive scan
     would pick up. -->
<protocol>
   <version>3.1.0</version>
   <sid>braidpoc</sid>
   <name>BraidPoC</name>
   <descr></descr>
   <nopart>3</nopart>
   <statdist>100</statdist>
   <thres>2</thres>
   <pgroup>ECqPGroup(P-256)::0000000002</pgroup>
   <keywidth>1</keywidth>
   <vbitlenro>256</vbitlenro>
   <ebitlenro>256</ebitlenro>
   <prg>SHA-256</prg>
   <rohash>SHA-256</rohash>
   <width>2</width>
   <party>
      <name>Party1</name>
      <descr></descr>
   </party>
   <party>
      <name>Party2</name>
      <descr></descr>
   </party>
</protocol>
"#;

    #[test]
    fn reads_every_field() {
        let info = ProtocolInfo::parse(SAMPLE).unwrap();
        assert_eq!(info.version, "3.1.0");
        assert_eq!(info.sid, "braidpoc");
        assert_eq!(info.parties, 3);
        assert_eq!(info.threshold, 2);
        assert_eq!(info.width, 2);
        assert_eq!(info.key_width, 1);
        assert_eq!(info.n_r, 100);
        assert_eq!(info.n_v, 256);
        assert_eq!(info.n_e, 256);
        assert_eq!(info.prg, "SHA-256");
        assert_eq!(info.rohash, "SHA-256");
        assert_eq!(info.pgroup, "ECqPGroup(P-256)::0000000002");
        assert!(info.is_consistent());
    }

    /// The decoy `<sid>` lives inside a comment. If comments were not stripped
    /// first, the duplicate check would reject a valid file — or worse, a
    /// differently ordered one would read the decoy.
    #[test]
    fn markup_inside_comments_is_ignored() {
        assert_eq!(ProtocolInfo::parse(SAMPLE).unwrap().sid, "braidpoc");
    }

    /// `<name>` and `<descr>` occur at both levels, so searching the whole
    /// document would see several. Only the top-level region is considered,
    /// which is also why the parties' fields are unreachable here.
    #[test]
    fn party_blocks_are_not_searched() {
        let body = top_level(SAMPLE);
        assert!(!body.contains("Party1"));
        assert_eq!(field(&body, "name").unwrap(), "BraidPoC");
    }

    #[test]
    fn a_missing_field_is_an_error() {
        let without = SAMPLE.replace("<sid>braidpoc</sid>", "");
        assert!(ProtocolInfo::parse(&without).is_err());
    }

    /// First-wins would be the tempting shortcut. A repeated element means the
    /// file is not what we think it is, and a verifier should say so.
    #[test]
    fn a_repeated_field_is_an_error() {
        let twice = SAMPLE.replace(
            "<sid>braidpoc</sid>",
            "<sid>braidpoc</sid>\n   <sid>other</sid>",
        );
        assert!(ProtocolInfo::parse(&twice).is_err());
    }

    #[test]
    fn a_non_numeric_count_is_an_error() {
        let bad = SAMPLE.replace("<nopart>3</nopart>", "<nopart>three</nopart>");
        assert!(ProtocolInfo::parse(&bad).is_err());
    }

    #[test]
    fn an_impossible_threshold_is_reported() {
        let bad = SAMPLE.replace("<thres>2</thres>", "<thres>4</thres>");
        assert!(!ProtocolInfo::parse(&bad).unwrap().is_consistent());
    }

    /// The group string feeds rho verbatim, comment prefix and all, so it must
    /// not be normalised or split on `::`.
    #[test]
    fn the_group_string_is_kept_whole() {
        let info = ProtocolInfo::parse(SAMPLE).unwrap();
        assert!(info.pgroup.starts_with("ECqPGroup(P-256)::"));
        assert_eq!(info.prefix_params("default").pgroup, info.pgroup);
    }

    #[test]
    fn auxsid_comes_from_the_caller_not_the_file() {
        let params = ProtocolInfo::parse(SAMPLE).unwrap().prefix_params("session7");
        assert_eq!(params.auxsid, "session7");
        assert_eq!(params.sid, "braidpoc");
    }
}
