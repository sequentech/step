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

/// The marshalled `ECqPGroup(P-256)` string, comment prefix included.
///
/// ρ commits to this verbatim, so it is stored as VMN writes it rather than
/// rebuilt from the curve name. The hex is the byte tree of the group's own
/// marshalling: the class name `com.verificatum.arithm.ECqPGroup` followed by
/// the curve name `P-256`.
pub const P256_PGROUP: &str = "ECqPGroup(P-256)::0000000002010000002\
0636f6d2e766572696669636174756d2e61726974686d2e4543715047726f757001000000\
05502d323536";

impl ProtocolInfo {
    /// A P-256 session with Verificatum's own defaults for everything the
    /// caller has no reason to choose.
    ///
    /// The three bit lengths are VMN's defaults rather than ours; changing them
    /// changes ρ, so a file that disagrees with the one the prover used is
    /// rejected wholesale rather than in part.
    #[must_use]
    pub fn p256(sid: &str, parties: usize, threshold: usize, width: usize) -> Self {
        ProtocolInfo {
            version: "3.1.0".to_string(),
            sid: sid.to_string(),
            parties,
            threshold,
            width,
            key_width: 1,
            n_r: 100,
            n_v: 256,
            n_e: 256,
            prg: "SHA-256".to_string(),
            rohash: "SHA-256".to_string(),
            pgroup: P256_PGROUP.to_string(),
        }
    }

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

    /// The property the whole scheme rests on: a file we write parses back to
    /// the same parameters, so rho derived from a synthesized file equals rho
    /// derived from a generated one.
    #[test]
    fn synthesized_files_round_trip() {
        let original = ProtocolInfo::parse(SAMPLE).unwrap();
        let reparsed = ProtocolInfo::parse(&original.to_xml()).unwrap();
        assert_eq!(original, reparsed);
        assert_eq!(
            original.prefix_params("default"),
            reparsed.prefix_params("default"),
            "the random-oracle prefix must be unaffected by the round trip"
        );
    }

    /// Round-tripping must hold across the shapes a sweep would use, not just
    /// the one sample -- that is the entire point of being able to write these.
    #[test]
    fn round_trips_across_session_shapes() {
        let base = ProtocolInfo::parse(SAMPLE).unwrap();
        for parties in 1..=8 {
            for threshold in 1..=parties {
                for width in [1usize, 2, 5] {
                    let shape = ProtocolInfo {
                        parties,
                        threshold,
                        width,
                        sid: format!("sweep{parties}x{threshold}x{width}"),
                        ..base.clone()
                    };
                    let back = ProtocolInfo::parse(&shape.to_xml())
                        .expect("a synthesized file must parse");
                    assert_eq!(shape, back, "{parties}-of-{threshold}, width {width}");
                    assert!(back.is_consistent());
                }
            }
        }
    }

    /// A synthesized file must say what it is. Its parties share a signature
    /// key, so it describes no protocol that could be run, and the format gives
    /// no other clue.
    #[test]
    fn synthesized_files_are_marked_verification_only() {
        let xml = ProtocolInfo::parse(SAMPLE).unwrap().to_xml();
        assert!(xml.contains("VERIFICATION ONLY"));
        // And the marking must not leak into the parsed values.
        assert_eq!(ProtocolInfo::parse(&xml).unwrap().sid, "braidpoc");
    }

    /// One party block per party, each with the key vmnv tolerates.
    #[test]
    fn a_party_block_is_written_for_each_party() {
        let info = ProtocolInfo {
            parties: 5,
            threshold: 3,
            ..ProtocolInfo::parse(SAMPLE).unwrap()
        };
        let xml = info.to_xml();
        assert_eq!(xml.matches("<party>").count(), 5);
        assert_eq!(xml.matches(PLACEHOLDER_PKEY).count(), 5);
        assert_eq!(ProtocolInfo::parse(&xml).unwrap().parties, 5);
    }
}

// -------------------------------------------------------------------------
// Writing protocol info files
// -------------------------------------------------------------------------

/// A signature key lifted verbatim from a generated protocol info file, reused
/// for every party in a synthesized one.
///
/// Producing genuine per-party keys would mean implementing VMN's marshalling of
/// RSA public keys, and buys nothing: Fiat–Shamir verification checks no
/// signatures, which is why `vmnv` accepts a file whose parties share a key.
/// That was confirmed by running it against a hand-built four-party file.
///
/// It is also why [`ProtocolInfo::to_xml`] stamps its output as
/// verification-only — a file with duplicate signing keys resembles a real
/// protocol configuration and must never be used as one.
pub(crate) const PLACEHOLDER_PKEY: &str = concat!(
    "com.verificatum.crypto.SignaturePKeyHeuristic(RSA, bitlength=2048)::",
    "0000000002010000002d636f6d2e766572696669636174756d2e63727970746f2e53",
    "69676e6174757265504b657948657572697374696300000000020100000126308201",
    "22300d06092a864886f70d01010105000382010f003082010a0282010100a9e0b6b8",
    "450981b9baf72550e4ac92ed78a886bff8c0f2a2f123e0c9e75449c63772c2215131",
    "1aa0800b2acc9d4dff21c95e9860be2a52258172b2339f8d265a5da4e176658a4477",
    "19527b6cbaa2d5c9609726361c5f24764ffc4f2976bc7d2e652c742f74e9be3a41d4",
    "7c965b2760631a8baad172df34291c0b911fb68dee88ff4f68ffb4d369a54cffe8e3",
    "aa8a4664139d961e14df715a5334d2ea0ea88d9ddc15fff041c30af33142f8e2e0d1",
    "5cf96364774f274757e80c3b26f1054d244554ab240acd5005e568239ca6d4b8b114",
    "3c6b071dc06dfb7287e420bae4f84e44ec42301a363fc053d224c37df40b0301c467",
    "aa506e7a6238aa9c9cb695b8207f0203010001010000000400000800",
);

impl ProtocolInfo {
    /// Render a protocol info file that `vmnv` will accept.
    ///
    /// The point is parameterization: `vmnv` takes its session shape — `k`, `λ`,
    /// the group, the widths — from this file, so testing a shape means having a
    /// file for it. Checking one in per combination does not scale, and
    /// generating them with `vmni` needs a Unix host.
    ///
    /// # This is not a protocol configuration
    ///
    /// Every party gets the same signature key, so the result is usable only for
    /// verification. It carries a comment saying so, because the format is
    /// otherwise indistinguishable from a real one.
    ///
    /// # Round-trip
    ///
    /// `parse(x.to_xml())` returns `x`, which is what makes ρ derived from a
    /// synthesized file agree with ρ derived from a generated one.
    #[must_use]
    pub fn to_xml(&self) -> String {
        let mut out = String::new();
        out.push_str(
            "<!-- GENERATED FOR VERIFICATION ONLY.\n\
             \x20    Every party shares one signature key, so this file describes no\n\
             \x20    protocol that could actually be run. It exists so a verifier can be\n\
             \x20    told the shape of a session. -->\n\n<protocol>\n",
        );

        let mut element = |tag: &str, value: &str| {
            out.push_str(&format!("   <{tag}>{value}</{tag}>\n"));
        };
        element("version", &self.version);
        element("sid", &self.sid);
        element("name", "Synthesized");
        element("descr", "");
        element("nopart", &self.parties.to_string());
        element("statdist", &self.n_r.to_string());
        element("bullboard", "com.verificatum.protocol.com.BullBoardBasicHTTPW");
        element("thres", &self.threshold.to_string());
        element("pgroup", &self.pgroup);
        element("keywidth", &self.key_width.to_string());
        element("vbitlen", "128");
        element("vbitlenro", &self.n_v.to_string());
        element("ebitlen", "128");
        element("ebitlenro", &self.n_e.to_string());
        element("prg", &self.prg);
        element("rohash", &self.rohash);
        element("corr", "noninteractive");
        element("width", &self.width.to_string());
        element("maxciph", "0");

        for party in 1..=self.parties {
            out.push_str(&format!(
                "\n   <party>\n      <name>Party{party}</name>\n      \
                 <srtbyrole>anyrole</srtbyrole>\n      <descr></descr>\n      \
                 <pkey>{PLACEHOLDER_PKEY}</pkey>\n      \
                 <http>http://localhost:8040</http>\n      \
                 <hint>localhost:4040</hint>\n   </party>\n"
            ));
        }

        out.push_str("\n</protocol>\n");
        out
    }
}
