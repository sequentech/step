// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The draft of an event's trust policy (Vitrine's `specs/policy.md`), made
//! from the deployment's configuration and the event, for the Board to seal
//! with `vitrine policy seal`.

use anyhow::{anyhow, bail, ensure, Result};
use electoral_log::vitrine::entry::check_id;
use electoral_log::vitrine::{Vkey, ELECTION_TAG, EVENTS_TAG};
use sequent_core::ballot::TransparencyMode;
use sequent_core::types::hasura::core::Election;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use strum_macros::{Display, EnumString};

use super::config::{DeploymentConfig, HubRole, Mirror, MIRROR_PATH_PLACEHOLDER};
use super::contact::AuthorityContact;
use super::window::VotingWindow;
use super::{canonical_id, LogName};

pub const POLICY_HEADER: &str = "vitrine/policy/v1";
const ALG_ED25519: u8 = 0x01;
const ALG_COSIGNATURE: u8 = 0x04;
const ON: &str = "on";
const PARAM_EVENT_ID: &str = "event-id";
const PARAM_ELECTION_IDS: &str = "election-ids";
/// Votes a voter may cast when `num_allowed_revotes` is unset.
const DEFAULT_ALLOWED_VOTES: i64 = 1;
/// `num_allowed_revotes` for unlimited votes.
const UNLIMITED_VOTES: i64 = 0;

/// Step's first integration supports its two reviewed election profiles.
/// The sidecar performs full policy parsing, signature and infrastructure
/// validation. This application guard runs before either log is installed.
pub fn check_step_policy_admission(note: &str) -> Result<()> {
    use electoral_log::vitrine::{ELECTION_MEANING_HASH, EVENTS_MEANING_HASH};
    ensure!(note.len() <= 65536, "a policy note is at most 64 KiB");
    let text = note.split("\n\n").next().unwrap_or_default();
    let lines: Vec<&str> = text.lines().collect();
    ensure!(
        lines.first() == Some(&POLICY_HEADER),
        "not a Vitrine policy"
    );
    let required: Vec<_> = lines
        .iter()
        .filter(|line| line.split(' ').next() == Some("required"))
        .collect();
    ensure!(
        required == [&"required on"],
        "Step transparency requires exactly one required on line"
    );
    let profiles: Vec<_> = lines
        .iter()
        .filter(|line| line.split(' ').next() == Some("profile"))
        .map(|line| line.split(' ').collect::<Vec<_>>())
        .collect();
    ensure!(
        profiles.len() == 2,
        "Step requires exactly its two reviewed profile bindings"
    );
    for (name, tag, meaning) in [
        ("ballots", ELECTION_TAG, ELECTION_MEANING_HASH),
        ("events", EVENTS_TAG, EVENTS_MEANING_HASH),
    ] {
        ensure!(profiles.iter().filter(|fields| fields.as_slice() == ["profile", name, tag, meaning]).count() == 1,
            "Step requires the reviewed {name} profile and meaning hash without unsupported future versions");
    }
    Ok(())
}

/// Why an event can't be public.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, EnumString)]
pub enum PublicRefusal {
    /// A public ballots log shows a coercer whether a voter cast a ballot
    /// after the one they watched, so voters must be able to vote again as
    /// often as they need: every election allows unlimited votes.
    #[strum(serialize = "public-needs-unlimited-revotes")]
    LimitedRevotes,
}

/// Why `public` is refused for these elections, if it is.
pub fn public_refusal(elections: &[Election]) -> Option<PublicRefusal> {
    elections
        .iter()
        .any(|e| e.num_allowed_revotes.unwrap_or(DEFAULT_ALLOWED_VOTES) != UNLIMITED_VOTES)
        .then_some(PublicRefusal::LimitedRevotes)
}

/// An event log as the sidecar created it.
#[derive(Debug, Clone)]
pub struct DraftLog {
    pub origin: String,
    pub vkey: String,
    pub log_time_vkey: String,
    /// Private-only Ed25519 key, provisioned and held by the sidecar.
    pub blinded_vkey: Option<String>,
    /// Its tlog-tiles prefix.
    pub url: String,
    /// Its name on the sidecar, which mirrors keep.
    pub path: String,
}

#[derive(Debug, Clone)]
pub struct DraftEvent {
    pub name: String,
    pub mode: TransparencyMode,
    pub election_event_id: String,
    pub election_ids: Vec<String>,
    pub window: VotingWindow,
    /// Private events: 32 random bytes, in lowercase hex.
    pub policy_salt: Option<String>,
    /// How voters reach the election authority: one `contact` line per kind
    /// that is set.
    pub contact: AuthorityContact,
    pub ballots: DraftLog,
    pub events: DraftLog,
}

/// The policy's `name`: `<tenant slug>/<event id>`, or the tenant's ID when
/// its slug isn't one.
pub fn policy_name(
    tenant_slug: Option<&str>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<String> {
    let event = canonical_id(election_event_id)?;
    if let Some(slug) = tenant_slug {
        let name = format!("{slug}/{event}");
        if check_id(&name).is_ok() {
            return Ok(name);
        }
    }
    Ok(format!("{}/{event}", canonical_id(tenant_id)?))
}

fn check_log_keys(name: LogName, log: &DraftLog) -> Result<(Vkey, Vkey)> {
    let key = Vkey::parse(&log.vkey).map_err(|e| anyhow!("the {name} log's vkey: {e}"))?;
    let time = Vkey::parse(&log.log_time_vkey)
        .map_err(|e| anyhow!("the {name} log's log-time vkey: {e}"))?;
    ensure!(
        key.alg == ALG_ED25519 && time.alg == ALG_COSIGNATURE,
        "the {name} log's keys aren't an Ed25519 key and a cosignature/v1 key"
    );
    ensure!(
        key.name == log.origin && time.name == log.origin,
        "the {name} log's keys aren't named by its origin {}",
        log.origin
    );
    Ok((key, time))
}

fn mirrors_of(config: &DeploymentConfig, name: LogName) -> impl Iterator<Item = &Mirror> {
    config.mirrors.iter().filter(move |m| {
        m.logs
            .as_ref()
            .is_none_or(|logs| logs.iter().any(|l| l == name.as_ref()))
    })
}

struct Lines(Vec<String>);

impl Lines {
    fn push(&mut self, words: &[&str]) {
        self.0.push(words.join(" "));
    }
}

fn threshold_line(lines: &mut Lines, keyword: &str, k: u64, authorities: &[String]) {
    let k = k.to_string();
    let mut words = vec![keyword, k.as_str()];
    words.extend(authorities.iter().map(String::as_str));
    lines.push(&words);
}

fn log_lines(
    lines: &mut Lines,
    config: &DeploymentConfig,
    name: LogName,
    log: &DraftLog,
    public: bool,
) -> Result<()> {
    let log_name = name.as_ref();
    lines.push(&["log", log_name, &log.vkey, &log.url]);
    lines.push(&["log-time", log_name, &log.log_time_vkey]);
    if public {
        for mirror in mirrors_of(config, name) {
            let url = mirror.url.replace(MIRROR_PATH_PLACEHOLDER, &log.path);
            lines.push(&["mirror", log_name, &url]);
            if let Some(party) = &mirror.party {
                lines.push(&["mirror-party", log_name, &url, party]);
            }
        }
    }
    let (tag, meaning) = match name {
        LogName::Ballots => (ELECTION_TAG, &config.profiles.election),
        LogName::Events => (EVENTS_TAG, &config.profiles.events),
    };
    lines.push(&["profile", log_name, tag, meaning]);
    Ok(())
}

/// The private anchor's unlinkable origin (Vitrine hub.md section 1).
pub fn pseudo_origin(origin: &str, salt: &str) -> Result<String> {
    ensure!(
        salt.len() == 64
            && salt
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "a policy salt is 32 bytes in lowercase hex"
    );
    let mut hash = Sha256::new();
    hash.update(b"vitrine/origin/v1");
    hash.update(origin.as_bytes());
    hash.update(hex::decode(salt)?);
    Ok(hex::encode(hash.finalize()))
}

/// Whether the deployment can draft policies of this mode: a public event's
/// ballots log needs a mirror that the operator doesn't run (rule 10), and a
/// private event needs an auditor (rule 7).
pub fn check_mode(config: &DeploymentConfig, mode: TransparencyMode) -> Result<()> {
    match mode {
        TransparencyMode::PUBLIC => ensure!(
            mirrors_of(config, LogName::Ballots)
                .any(|m| m.party.as_deref().is_some_and(|p| p != config.operator)),
            "a public event's ballots log needs a mirror that the operator doesn't run: add one to the deployment's [[mirrors]] with its party"
        ),
        TransparencyMode::PRIVATE => ensure!(
            !config.auditors.is_empty(),
            "a private event needs an auditor: add one to the deployment's [[auditors]]"
        ),
        TransparencyMode::DISABLED => {}
    }
    Ok(())
}

/// The draft. It is valid under every rule of `specs/policy.md`, section 3,
/// but the seal's.
pub fn draft(config: &DeploymentConfig, event: &DraftEvent) -> Result<String> {
    let public = match event.mode {
        TransparencyMode::PUBLIC => true,
        TransparencyMode::PRIVATE => false,
        TransparencyMode::DISABLED => bail!("the event's transparency is disabled"),
    };
    check_id(&event.name).map_err(|e| anyhow!("the policy name {:?}: {e}", event.name))?;
    ensure!(!event.election_ids.is_empty(), "the event has no elections");
    ensure!(
        event.window.opens >= 0 && event.window.opens < event.window.closes,
        "the voting window ends before it starts"
    );
    let event_id = canonical_id(&event.election_event_id)?;
    let election_ids: BTreeSet<String> = event
        .election_ids
        .iter()
        .map(|id| canonical_id(id))
        .collect::<Result<_>>()?;

    let (ballots_key, ballots_time) = check_log_keys(LogName::Ballots, &event.ballots)?;
    let (events_key, events_time) = check_log_keys(LogName::Events, &event.events)?;
    ensure!(
        event.ballots.origin != event.events.origin,
        "the event's logs have one origin"
    );
    let mut config_keys = BTreeSet::new();
    for vkey in config
        .authorities
        .iter()
        .map(|a| &a.vkey)
        .chain(config.auditors.iter().map(|a| &a.vkey))
        .chain(config.witnesses.iter().map(|w| &w.vkey))
        .chain(config.monitors.iter().map(|m| &m.vkey))
        .chain(config.hubs.iter().map(|h| &h.vkey))
        .chain(
            config
                .hubs
                .iter()
                .flat_map(|h| h.witnesses.iter().map(|w| &w.vkey)),
        )
    {
        config_keys.insert(Vkey::parse(vkey)?.key);
    }
    let log_keys = [&ballots_key, &ballots_time, &events_key, &events_time];
    let distinct: BTreeSet<&Vec<u8>> = log_keys.iter().map(|k| &k.key).collect();
    ensure!(
        distinct.len() == log_keys.len() && log_keys.iter().all(|k| !config_keys.contains(&k.key)),
        "a log key is another key of the policy"
    );

    let hubs: Vec<_> = config.hubs.iter().collect();
    for hub in &hubs {
        let origin = Vkey::parse(&hub.vkey)?.name;
        ensure!(
            origin != event.ballots.origin && origin != event.events.origin,
            "hub {} has the origin of one of the event's logs",
            hub.name
        );
    }
    check_mode(config, event.mode)?;

    let mut blinded = Vec::new();
    if !public {
        let salt = event
            .policy_salt
            .as_deref()
            .ok_or_else(|| anyhow!("a private event's policy needs a salt"))?;
        let mut used_keys = config_keys;
        used_keys.extend(log_keys.iter().map(|k| k.key.clone()));
        for (name, log) in [
            (LogName::Ballots, &event.ballots),
            (LogName::Events, &event.events),
        ] {
            let pseudo = pseudo_origin(&log.origin, salt)?;
            let vkey = log
                .blinded_vkey
                .as_deref()
                .ok_or_else(|| anyhow!("the {name} log has no provisioned blinded key"))?;
            let key = Vkey::parse(vkey)?;
            ensure!(
                key.alg == ALG_ED25519 && key.name == pseudo,
                "the {name} blinded key must be Ed25519 and named by its derived pseudo-origin"
            );
            ensure!(
                used_keys.insert(key.key),
                "a blinded key is another key of the policy"
            );
            blinded.push((name, pseudo, vkey));
        }
    } else {
        ensure!(
            event.ballots.blinded_vkey.is_none() && event.events.blinded_vkey.is_none(),
            "public policies have no blinded keys"
        );
    }

    let mut lines = Lines(Vec::new());
    lines.push(&[POLICY_HEADER]);
    lines.push(&["name", &event.name]);
    lines.push(&["visibility", if public { "public" } else { "private" }]);
    lines.push(&["required", ON]);
    if !public {
        let salt = event
            .policy_salt
            .as_deref()
            .ok_or_else(|| anyhow!("a private event's policy needs a salt"))?;
        ensure!(
            salt.len() == 64
                && salt
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
            "a policy salt is 32 bytes in lowercase hex"
        );
        lines.push(&["policy-salt", salt]);
    }
    lines.push(&["operator", &config.operator]);
    lines.push(&["platform", config.platform()]);
    event.contact.check()?;
    for contact in event.contact.lines() {
        lines.push(&contact);
    }
    if let Some(min_faults) = config.min_faults {
        lines.push(&["min-faults", &min_faults.to_string()]);
    }
    for a in &config.authorities {
        lines.push(&["authority", &a.name, &a.vkey]);
    }
    threshold_line(
        &mut lines,
        "sign",
        config.sign.threshold,
        &config.sign.authorities,
    );
    if let Some(redaction) = &config.redaction {
        threshold_line(
            &mut lines,
            "redaction",
            redaction.threshold,
            &redaction.authorities,
        );
    }
    for a in &config.auditors {
        lines.push(&["auditor", &a.name, &a.vkey]);
    }
    lines.push(&["mmd", &config.mmd().to_string()]);
    lines.push(&["opens", &event.window.opens.to_string()]);
    lines.push(&["closes", &event.window.closes.to_string()]);
    lines.push(&["quiet", &event.window.quiet().to_string()]);
    for m in &config.monitors {
        lines.push(&["monitor", &m.name, &m.vkey, &m.url]);
    }
    for m in &config.monitors {
        if let Some(party) = &m.party {
            lines.push(&["monitor-party", &m.name, party]);
        }
    }
    for m in &config.monitors {
        if let Some(url) = &m.explorer {
            lines.push(&["monitor-explorer", &m.name, url]);
        }
    }
    if let Some(k) = config.monitor_quorum {
        lines.push(&["monitor-quorum", &k.to_string()]);
    }
    for url in &config.explorers {
        lines.push(&["explorer", url]);
    }

    log_lines(&mut lines, config, LogName::Ballots, &event.ballots, public)?;
    lines.push(&[
        "param",
        LogName::Ballots.as_ref(),
        PARAM_EVENT_ID,
        &event_id,
    ]);
    let mut ids = vec!["param", LogName::Ballots.as_ref(), PARAM_ELECTION_IDS];
    ids.extend(election_ids.iter().map(String::as_str));
    lines.push(&ids);
    log_lines(&mut lines, config, LogName::Events, &event.events, public)?;

    for w in &config.witnesses {
        match &w.url {
            Some(url) => lines.push(&["witness", &w.name, &w.vkey, url]),
            None => lines.push(&["witness", &w.name, &w.vkey]),
        }
    }
    for w in &config.witnesses {
        if let Some(party) = &w.party {
            lines.push(&["witness-party", &w.name, party]);
        }
    }
    for g in &config.groups {
        let k = g.threshold.to_string();
        let mut words = vec!["group", g.name.as_str(), k.as_str()];
        words.extend(g.members.iter().map(String::as_str));
        lines.push(&words);
    }
    lines.push(&["quorum", &config.quorum]);

    for (name, pseudo, vkey) in &blinded {
        lines.push(&["blinded", name.as_ref(), pseudo, vkey]);
    }
    if !hubs.is_empty() {
        if let Some(anchor) = &config.anchor {
            lines.push(&[
                "anchor",
                &anchor.period.to_string(),
                &anchor.max_gap.to_string(),
            ]);
        }
        let rule = config.anchor_hubs_rule();
        lines.push(&[
            "anchor-hubs",
            &rule.min.to_string(),
            &rule.min_independent.to_string(),
        ]);
        for h in &hubs {
            lines.push(&["hub", &h.name, &h.vkey, &h.url]);
            lines.push(&["hub-operator", &h.name, &h.operator]);
            if let Some(url) = &h.explorer {
                lines.push(&["hub-explorer", &h.name, url]);
            }
            for w in &h.witnesses {
                match &w.url {
                    Some(url) => lines.push(&["hub-witness", &h.name, &w.name, &w.vkey, url]),
                    None => lines.push(&["hub-witness", &h.name, &w.name, &w.vkey]),
                }
            }
            for w in &h.witnesses {
                if let Some(party) = &w.party {
                    lines.push(&["hub-witness-party", &h.name, &w.name, party]);
                }
            }
            for g in &h.groups {
                let k = g.threshold.to_string();
                let mut words = vec!["hub-group", h.name.as_str(), g.name.as_str(), k.as_str()];
                words.extend(g.members.iter().map(String::as_str));
                lines.push(&words);
            }
            lines.push(&["hub-quorum", &h.name, &h.quorum]);
        }
        for h in hubs.iter().filter(|h| h.role == HubRole::Bootstrap) {
            lines.push(&["bootstrap", &h.name]);
        }
        if let Some(period) = config.browser_period {
            lines.push(&["browser", &period.to_string()]);
        }
    }

    let mut text = lines.0.join("\n");
    text.push('\n');
    Ok(text)
}

/// What step reads back from a signed policy: the lines its checks at
/// opening and its config statements use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyFacts {
    pub visibility: TransparencyMode,
    pub opens: i64,
    pub closes: i64,
    /// The `ballots` log's `event-id` parameter.
    pub event_id: Option<String>,
    /// The `ballots` log's `election-ids` parameter.
    pub election_ids: BTreeSet<String>,
}

impl PolicyFacts {
    /// Reads a policy note's text, up to its signatures. It checks only what
    /// it reads: the sidecar checks the rest when the policy is installed.
    pub fn read(note: &str) -> Result<Self> {
        let text = note.split("\n\n").next().unwrap_or_default();
        let mut lines = text.split('\n');
        ensure!(
            lines.next() == Some(POLICY_HEADER),
            "the policy doesn't start with {POLICY_HEADER}"
        );
        let mut visibility = None;
        let mut opens = None;
        let mut closes = None;
        let mut event_id = None;
        let mut election_ids = BTreeSet::new();
        let ballots = LogName::Ballots.as_ref();
        for line in lines {
            let words: Vec<&str> = line.split(' ').collect();
            match words.as_slice() {
                ["visibility", "public"] => visibility = Some(TransparencyMode::PUBLIC),
                ["visibility", "private"] => visibility = Some(TransparencyMode::PRIVATE),
                ["opens", time] => opens = Some(time.parse::<i64>()?),
                ["closes", time] => closes = Some(time.parse::<i64>()?),
                ["param", log, PARAM_EVENT_ID, id] if *log == ballots => {
                    event_id = Some(id.to_string());
                }
                ["param", log, PARAM_ELECTION_IDS, ids @ ..] if *log == ballots => {
                    election_ids.extend(ids.iter().map(|id| id.to_string()));
                }
                _ => {}
            }
        }
        let (Some(opens), Some(closes)) = (opens, closes) else {
            bail!("the policy has no opens and closes lines");
        };
        Ok(PolicyFacts {
            visibility: visibility.ok_or_else(|| anyhow!("the policy has no visibility line"))?,
            opens,
            closes,
            event_id,
            election_ids,
        })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::super::config::tests::{dev, DEV_CONFIG};
    use super::super::config::AnchorHubs;
    use super::super::window::QUIET_SECS;
    use super::*;
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use electoral_log::vitrine::note::key_hash;
    use electoral_log::vitrine::NoteSigner;
    use serde_json::json;

    pub(crate) const TENANT: &str = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5";
    pub(crate) const EVENT: &str = "e7a2b3c4-0000-4000-8000-000000000001";
    const MAYOR: &str = "e7a2b3c4-0000-4000-8000-000000000002";
    const COUNCIL: &str = "e7a2b3c4-0000-4000-8000-000000000003";

    pub(crate) fn ed25519_vkey(name: &str, seed: u8) -> String {
        NoteSigner::from_seed(name, &[seed; 32]).unwrap().vkey()
    }

    pub(crate) fn cosigner_vkey(name: &str, seed: u8) -> String {
        let signer = NoteSigner::from_seed(name, &[seed; 32]).unwrap();
        let key = Vkey::parse(&signer.vkey()).unwrap().key;
        let mut data = vec![ALG_COSIGNATURE];
        data.extend_from_slice(&key);
        format!(
            "{name}+{}+{}",
            hex::encode(key_hash(name, ALG_COSIGNATURE, &key)),
            STANDARD.encode(data)
        )
    }

    pub(crate) fn log(config: &DeploymentConfig, name: LogName, seed: u8) -> DraftLog {
        let origin = config.origin(TENANT, EVENT, name).unwrap();
        let path = super::super::sidecar_name(TENANT, EVENT, name).unwrap();
        DraftLog {
            vkey: ed25519_vkey(&origin, seed),
            log_time_vkey: cosigner_vkey(&origin, seed + 1),
            blinded_vkey: None,
            url: format!("http://localhost:8094/{path}/"),
            origin,
            path,
        }
    }

    pub(crate) fn event(config: &DeploymentConfig, mode: TransparencyMode) -> DraftEvent {
        let mut ballots = log(config, LogName::Ballots, 101);
        let mut events = log(config, LogName::Events, 103);
        if mode == TransparencyMode::PRIVATE {
            let salt = "ab".repeat(32);
            ballots.blinded_vkey = Some(ed25519_vkey(
                &pseudo_origin(&ballots.origin, &salt).unwrap(),
                111,
            ));
            events.blinded_vkey = Some(ed25519_vkey(
                &pseudo_origin(&events.origin, &salt).unwrap(),
                112,
            ));
        }
        DraftEvent {
            name: policy_name(Some("acme"), TENANT, EVENT).unwrap(),
            mode,
            election_event_id: EVENT.into(),
            election_ids: vec![MAYOR.into(), COUNCIL.into()],
            window: VotingWindow {
                opens: 1793520000,
                closes: 1793566800,
            },
            policy_salt: (mode == TransparencyMode::PRIVATE).then(|| "ab".repeat(32)),
            contact: AuthorityContact::default(),
            ballots,
            events,
        }
    }

    fn keywords(text: &str) -> Vec<&str> {
        text.lines().map(|l| l.split(' ').next().unwrap()).collect()
    }

    fn line<'a>(text: &'a str, keyword: &str) -> Vec<&'a str> {
        text.lines()
            .filter(|l| l.split(' ').next() == Some(keyword))
            .collect()
    }

    /// The development configuration, with a monitor and two hubs.
    pub(crate) fn with_hubs() -> DeploymentConfig {
        let mut table: toml::Table = toml::from_str(DEV_CONFIG).unwrap();
        let extra: toml::Table = toml::from_str(&format!(
            r#"
            monitor_quorum = 2
            browser_period = 600
            explorers = ["https://explorer.one.example/", "https://explorer.two.example/"]
            [anchor]
            period = 10
            max_gap = 60
            [anchor_hubs]
            min = 2
            min_independent = 1
            [[monitors]]
            name = "observers"
            vkey = "{monitor}"
            url = "https://monitor.observers.example/"
            party = "observers"
            explorer = "https://explorer.observers.example/"
            [[hubs]]
            name = "example"
            vkey = "{hub}"
            url = "https://hub.example.org/"
            operator = "example-org"
            explorer = "https://explorer.example.org/"
            quorum = "ew"
            witnesses = [
                {{ name = "ew1", vkey = "{ew1}", party = "example-org" }},
                {{ name = "ew2", vkey = "{ew2}", party = "press" }},
                {{ name = "ew3", vkey = "{ew3}", party = "lawyers" }},
            ]
            groups = [{{ name = "ew", threshold = "all", members = ["ew1", "ew2", "ew3"] }}]
            [[monitors]]
            name = "observers2"
            vkey = "{monitor2}"
            url = "https://monitor2.observers.example/"
            party = "observers2"
            [[hubs]]
            name = "other"
            vkey = "{hub2}"
            url = "https://hub2.example.org/"
            operator = "other-org"
            quorum = "ow1"
            witnesses = [{{ name = "ow1", vkey = "{ow1}" }}]
            [[hubs]]
            name = "boot"
            vkey = "{boot}"
            url = "https://bootstrap.example.net/"
            operator = "browser-vendor"
            role = "bootstrap"
            quorum = "bw1"
            witnesses = [{{ name = "bw1", vkey = "{bw1}" }}]
            "#,
            monitor = ed25519_vkey("monitor.observers.example", 201),
            monitor2 = ed25519_vkey("monitor2.observers.example", 208),
            hub2 = ed25519_vkey("hub2.example.org", 209),
            ow1 = cosigner_vkey("ow1.example.org", 210),
            hub = ed25519_vkey("hub.example.org", 202),
            ew1 = cosigner_vkey("ew1.example.org", 203),
            ew2 = cosigner_vkey("ew2.press.example", 204),
            ew3 = cosigner_vkey("ew3.lawyers.example", 205),
            boot = ed25519_vkey("bootstrap.example.net", 206),
            bw1 = cosigner_vkey("bw1.example.net", 207),
        ))
        .unwrap();
        table.extend(extra);
        DeploymentConfig::parse(&toml::to_string(&table).unwrap()).unwrap()
    }

    #[test]
    fn upload_admission_rejects_optional_unknown_and_legacy_profiles() {
        let config = dev();
        let valid = draft(&config, &event(&config, TransparencyMode::PUBLIC)).unwrap();
        check_step_policy_admission(&valid).unwrap();
        for invalid in [
            valid.replace("required on\n", "required off\n"),
            valid.replace("required on\n", ""),
            valid.replace("required on\n", "required on\nrequired on\n"),
            valid.replace(ELECTION_TAG, "vitrine/election/v1"),
            valid.replace(EVENTS_TAG, "vitrine/events/v1"),
            valid.replace(
                &config.profiles.election,
                &format!("sha256:{}", "ab".repeat(32)),
            ),
            format!(
                "{valid}profile ballots future/v2 sha256:{}\n",
                "ab".repeat(32)
            ),
        ] {
            assert!(
                check_step_policy_admission(&invalid).is_err(),
                "accepted {invalid}"
            );
        }
    }

    #[test]
    fn private_blinded_keys_must_match_both_derived_pseudo_origins() {
        let config = dev();
        let good = event(&config, TransparencyMode::PRIVATE);
        draft(&config, &good).unwrap();
        for name in [LogName::Ballots, LogName::Events] {
            let mut bad = good.clone();
            let log = if name == LogName::Ballots {
                &mut bad.ballots
            } else {
                &mut bad.events
            };
            log.blinded_vkey = None;
            assert!(draft(&config, &bad).is_err());
        }
        let mut bad = good.clone();
        bad.events.blinded_vkey = bad.ballots.blinded_vkey.clone();
        assert!(draft(&config, &bad).is_err());
        let mut bad = good.clone();
        bad.policy_salt = Some("ac".repeat(32));
        assert!(draft(&config, &bad).is_err());
        let mut bad = good;
        let pseudo =
            pseudo_origin(&bad.events.origin, bad.policy_salt.as_deref().unwrap()).unwrap();
        bad.events.blinded_vkey = Some(cosigner_vkey(&pseudo, 112));
        assert!(draft(&config, &bad).is_err());
    }

    #[test]
    fn a_public_draft_has_every_line_in_order() {
        let config = with_hubs();
        let text = draft(&config, &event(&config, TransparencyMode::PUBLIC)).unwrap();
        let order = keywords(&text);
        let mut expected = vec![
            "vitrine/policy/v1",
            "name",
            "visibility",
            "required",
            "operator",
            "platform",
        ];
        expected.extend(std::iter::repeat("authority").take(config.authorities.len()));
        expected.push("sign");
        expected.extend(std::iter::repeat("auditor").take(config.auditors.len()));
        expected.extend([
            "mmd",
            "opens",
            "closes",
            "quiet",
            "monitor",
            "monitor",
            "monitor-party",
            "monitor-party",
            "monitor-explorer",
            "monitor-quorum",
            "explorer",
            "explorer",
        ]);
        expected.extend(["log", "log-time"]);
        for m in mirrors_of(&config, LogName::Ballots) {
            expected.push("mirror");
            if m.party.is_some() {
                expected.push("mirror-party");
            }
        }
        expected.extend(["profile", "param", "param", "log", "log-time"]);
        for m in mirrors_of(&config, LogName::Events) {
            expected.push("mirror");
            if m.party.is_some() {
                expected.push("mirror-party");
            }
        }
        expected.push("profile");
        expected.extend(std::iter::repeat("witness").take(config.witnesses.len()));
        expected.extend(
            std::iter::repeat("witness-party").take(
                config
                    .witnesses
                    .iter()
                    .filter(|w| w.party.is_some())
                    .count(),
            ),
        );
        expected.extend(std::iter::repeat("group").take(config.groups.len()));
        expected.extend(["quorum", "anchor", "anchor-hubs"]);
        expected.extend([
            "hub",
            "hub-operator",
            "hub-explorer",
            "hub-witness",
            "hub-witness",
            "hub-witness",
        ]);
        expected.extend([
            "hub-witness-party",
            "hub-witness-party",
            "hub-witness-party",
            "hub-group",
            "hub-quorum",
        ]);
        expected.extend(["hub", "hub-operator", "hub-witness", "hub-quorum"]);
        expected.extend([
            "hub",
            "hub-operator",
            "hub-witness",
            "hub-quorum",
            "bootstrap",
            "browser",
        ]);
        assert_eq!(order, expected, "{text}");

        assert_eq!(
            line(&text, "name"),
            ["name acme/e7a2b3c4-0000-4000-8000-000000000001"]
        );
        assert_eq!(line(&text, "visibility"), ["visibility public"]);
        assert_eq!(
            line(&text, "monitor-explorer"),
            ["monitor-explorer observers https://explorer.observers.example/"]
        );
        assert_eq!(
            line(&text, "hub-explorer"),
            ["hub-explorer example https://explorer.example.org/"]
        );
        assert_eq!(
            line(&text, "explorer"),
            [
                "explorer https://explorer.one.example/",
                "explorer https://explorer.two.example/"
            ]
        );
        assert_eq!(line(&text, "required"), ["required on"]);
        assert_eq!(line(&text, "mmd"), ["mmd 30"]);
        assert_eq!(line(&text, "opens"), ["opens 1793520000"]);
        assert_eq!(line(&text, "closes"), ["closes 1793566800"]);
        assert_eq!(line(&text, "quiet"), [format!("quiet {QUIET_SECS}")]);
        assert!(line(&text, "words").is_empty(), "words in {text}");
        assert_eq!(line(&text, "platform"), ["platform Sequent"]);
        assert!(
            line(&text, "contact").is_empty(),
            "Step has no authority-contact settings, so a draft has no contact line: {text}"
        );
        assert_eq!(
            line(&text, "param"),
            [
                format!("param ballots event-id {EVENT}"),
                format!("param ballots election-ids {MAYOR} {COUNCIL}"),
            ]
        );
        assert_eq!(
            line(&text, "profile"),
            [
                format!(
                    "profile ballots vitrine/election/ballots/v1 {}",
                    config.profiles.election
                ),
                format!(
                    "profile events vitrine/election/events/v1 {}",
                    config.profiles.events
                ),
            ]
        );
        assert_eq!(line(&text, "bootstrap"), ["bootstrap boot"]);
        assert!(text.ends_with("browser 600\n"));
        assert!(!text.contains("policy-salt"));
        assert!(text
            .lines()
            .all(|l| !l.is_empty() && !l.starts_with(' ') && !l.ends_with(' ')));
    }

    #[test]
    fn the_quiet_period_is_the_whole_window_when_shorter() {
        let config = dev();
        let mut e = event(&config, TransparencyMode::PUBLIC);
        e.window = VotingWindow {
            opens: 1793520000,
            closes: 1793521800,
        };
        let text = draft(&config, &e).unwrap();
        assert_eq!(line(&text, "quiet"), ["quiet 1800"]);
    }

    #[test]
    fn a_private_draft_has_a_salt_both_blinded_logs_and_strong_hubs() {
        let config = with_hubs();
        let text = draft(&config, &event(&config, TransparencyMode::PRIVATE)).unwrap();
        assert_eq!(line(&text, "visibility"), ["visibility private"]);
        assert_eq!(
            line(&text, "policy-salt"),
            [format!("policy-salt {}", "ab".repeat(32))]
        );
        assert_eq!(line(&text, "blinded").len(), 2);
        assert_eq!(line(&text, "hub").len(), 3);
        assert_eq!(line(&text, "anchor-hubs"), ["anchor-hubs 2 1"]);
        assert_eq!(line(&text, "monitor-quorum"), ["monitor-quorum 2"]);
        for keyword in ["mirror", "mirror-party"] {
            assert!(line(&text, keyword).is_empty(), "{keyword} in {text}");
        }
        let mut e = event(&config, TransparencyMode::PRIVATE);
        e.policy_salt = None;
        assert!(draft(&config, &e).is_err());
    }

    #[test]
    fn a_public_draft_needs_a_mirror_the_operator_doesnt_run() {
        let mut config = dev();
        for m in &mut config.mirrors {
            m.party = Some(config.operator.clone());
        }
        let err = draft(&config, &event(&config, TransparencyMode::PUBLIC)).unwrap_err();
        assert!(err.to_string().contains("mirror"), "{err}");
    }

    #[test]
    fn a_private_draft_needs_an_auditor() {
        let mut config = dev();
        config.auditors.clear();
        assert!(draft(&config, &event(&config, TransparencyMode::PRIVATE)).is_err());
    }

    #[test]
    fn log_keys_must_be_the_logs_own_and_fresh() {
        let config = dev();
        let mut e = event(&config, TransparencyMode::PUBLIC);
        e.events.vkey = ed25519_vkey(&e.events.origin, 101);
        assert!(draft(&config, &e).is_err());
        let mut e = event(&config, TransparencyMode::PUBLIC);
        e.ballots.vkey = ed25519_vkey("another.example/ballots", 101);
        assert!(draft(&config, &e).is_err());
        let mut e = event(&config, TransparencyMode::PUBLIC);
        e.ballots.log_time_vkey = ed25519_vkey(&e.ballots.origin, 120);
        assert!(draft(&config, &e).is_err());
    }

    #[test]
    fn disabled_events_and_events_without_elections_have_no_draft() {
        let config = dev();
        assert!(draft(&config, &event(&config, TransparencyMode::DISABLED)).is_err());
        let mut e = event(&config, TransparencyMode::PUBLIC);
        e.election_ids.clear();
        assert!(draft(&config, &e).is_err());
    }

    #[test]
    fn the_platform_is_drafted_and_can_be_named_by_the_deployment() {
        let config = dev();
        let text = draft(&config, &event(&config, TransparencyMode::PUBLIC)).unwrap();
        assert_eq!(line(&text, "platform"), ["platform Sequent"]);
        assert_eq!(keywords(&text)[4..6], ["operator", "platform"]);
        let mut config = with_hubs();
        config.platform = Some("Vendor".into());
        let text = draft(&config, &event(&config, TransparencyMode::PUBLIC)).unwrap();
        assert_eq!(line(&text, "platform"), ["platform Vendor"]);
    }

    #[test]
    fn the_contact_is_drafted_after_the_platform_one_line_per_kind() {
        let config = dev();
        let mut e = event(&config, TransparencyMode::PUBLIC);
        assert!(line(&draft(&config, &e).unwrap(), "contact").is_empty());
        e.contact = AuthorityContact {
            name: Some("Electoral Board of Acme".into()),
            email: Some("elections@acme.example".into()),
            phone: None,
            url: Some("https://elections.acme.example/contact".into()),
        };
        let text = draft(&config, &e).unwrap();
        assert_eq!(
            line(&text, "contact"),
            [
                "contact name Electoral Board of Acme",
                "contact email elections@acme.example",
                "contact url https://elections.acme.example/contact",
            ]
        );
        assert_eq!(
            keywords(&text)[4..9],
            ["operator", "platform", "contact", "contact", "contact"]
        );
    }

    #[test]
    fn a_contact_that_breaks_its_syntax_refuses_the_draft() {
        let config = dev();
        for (name, email, phone, url) in [
            (Some("Two  spaces"), None, None, None),
            (None, Some("elections@acme"), None, None),
            (None, None, Some("+34 900 000 000"), None),
            (None, None, None, Some("http://elections.acme.example")),
        ] {
            let mut e = event(&config, TransparencyMode::PUBLIC);
            e.contact = AuthorityContact {
                name: name.map(Into::into),
                email: email.map(Into::into),
                phone: phone.map(Into::into),
                url: url.map(Into::into),
            };
            assert!(draft(&config, &e).is_err(), "{e:?}");
        }
    }

    #[test]
    fn a_hub_run_by_the_platform_isnt_independent() {
        let mut config = with_hubs();
        config.platform = Some("example-org".into());
        let [example, other, _] = &config.hubs[..] else {
            panic!("three hubs")
        };
        assert!(!config.hub_is_independent(example));
        assert!(config.hub_is_independent(other));
        // The other app hub still gives `anchor-hubs 2 1` its independent hub.
        assert_eq!(config.anchor_hubs_rule().min_independent, 1);
    }

    #[test]
    fn the_anchor_hubs_line_is_2_1_when_the_hubs_allow_it() {
        let config = with_hubs();
        assert_eq!(
            (
                config.anchor_hubs_rule().min,
                config.anchor_hubs_rule().min_independent
            ),
            (2, 1)
        );
        // A weaker rule in the deployment is raised to the recommended one.
        let mut config = with_hubs();
        config.anchor_hubs = Some(AnchorHubs {
            min: 1,
            min_independent: 0,
        });
        let text = draft(&config, &event(&config, TransparencyMode::PUBLIC)).unwrap();
        assert_eq!(line(&text, "anchor-hubs"), ["anchor-hubs 2 1"]);
        // A stronger one is kept when the hubs allow it.
        config.anchor_hubs = Some(AnchorHubs {
            min: 2,
            min_independent: 2,
        });
        let text = draft(&config, &event(&config, TransparencyMode::PUBLIC)).unwrap();
        assert_eq!(line(&text, "anchor-hubs"), ["anchor-hubs 2 2"]);
        // Without the deployment's rule, the hubs still give the recommended one.
        let mut table: toml::Table = toml::from_str(DEV_CONFIG).unwrap();
        table.remove("anchor_hubs");
        let config = DeploymentConfig::parse(&toml::to_string(&table).unwrap()).unwrap();
        let text = draft(&config, &event(&config, TransparencyMode::PUBLIC)).unwrap();
        assert_eq!(line(&text, "anchor-hubs"), ["anchor-hubs 2 1"]);
    }

    #[test]
    fn a_configuration_with_the_old_words_key_still_loads() {
        let mut table: toml::Table = toml::from_str(DEV_CONFIG).unwrap();
        table.insert("words".into(), "es".into());
        let config = DeploymentConfig::parse(&toml::to_string(&table).unwrap()).unwrap();
        let text = draft(&config, &event(&config, TransparencyMode::PUBLIC)).unwrap();
        assert!(line(&text, "words").is_empty(), "words in {text}");
    }

    #[test]
    fn policy_names_use_the_tenant_slug_when_they_can() {
        assert_eq!(
            policy_name(Some("acme"), TENANT, EVENT).unwrap(),
            format!("acme/{EVENT}")
        );
        assert_eq!(
            policy_name(Some("a cme"), TENANT, EVENT).unwrap(),
            format!("{TENANT}/{EVENT}")
        );
        assert_eq!(
            policy_name(None, TENANT, EVENT).unwrap(),
            format!("{TENANT}/{EVENT}")
        );
    }

    fn election(revotes: Option<i64>) -> Election {
        serde_json::from_value(json!({
            "id": MAYOR,
            "tenant_id": TENANT,
            "election_event_id": EVENT,
            "num_allowed_revotes": revotes,
        }))
        .unwrap()
    }

    #[test]
    fn public_needs_unlimited_revotes_in_every_election() {
        assert_eq!(public_refusal(&[election(Some(0))]), None);
        assert_eq!(public_refusal(&[]), None);
        assert_eq!(
            public_refusal(&[election(Some(0)), election(None)]),
            Some(PublicRefusal::LimitedRevotes)
        );
        assert_eq!(
            public_refusal(&[election(Some(3))]),
            Some(PublicRefusal::LimitedRevotes)
        );
        assert_eq!(
            PublicRefusal::LimitedRevotes.to_string(),
            "public-needs-unlimited-revotes"
        );
    }

    #[test]
    fn the_facts_of_a_signed_policy_come_from_its_text() {
        let config = dev();
        let text = draft(&config, &event(&config, TransparencyMode::PUBLIC)).unwrap();
        let sealed = format!("{text}seal board mJ2X9cQe1v0rTq8LkW4sPA==\n");
        let board = NoteSigner::from_seed("board.localhost/policy", &[9; 32]).unwrap();
        let facts = PolicyFacts::read(&board.sign(&sealed).unwrap()).unwrap();
        assert_eq!(
            facts,
            PolicyFacts {
                visibility: TransparencyMode::PUBLIC,
                opens: 1793520000,
                closes: 1793566800,
                event_id: Some(EVENT.to_string()),
                election_ids: [MAYOR, COUNCIL].map(String::from).into(),
            }
        );
        let private = draft(&config, &event(&config, TransparencyMode::PRIVATE)).unwrap();
        assert_eq!(
            PolicyFacts::read(&private).unwrap().visibility,
            TransparencyMode::PRIVATE
        );
    }

    #[test]
    fn a_policy_without_its_window_has_no_facts() {
        assert!(PolicyFacts::read("vitrine/config/v1\nopens 1\ncloses 2\n").is_err());
        assert!(PolicyFacts::read("vitrine/policy/v1\nvisibility public\nopens 1\n").is_err());
        assert!(PolicyFacts::read("vitrine/policy/v1\nopens 1\ncloses 2\n").is_err());
        assert!(
            PolicyFacts::read("vitrine/policy/v1\nvisibility public\nopens x\ncloses 2\n").is_err()
        );
        assert!(PolicyFacts::read("").is_err());
    }

    #[test]
    fn only_the_events_with_a_mirror_or_an_auditor_can_have_their_mode() {
        let mut config = dev();
        assert!(check_mode(&config, TransparencyMode::PUBLIC).is_ok());
        assert!(check_mode(&config, TransparencyMode::PRIVATE).is_ok());
        config.mirrors.clear();
        config.auditors.clear();
        assert!(check_mode(&config, TransparencyMode::PUBLIC).is_err());
        assert!(check_mode(&config, TransparencyMode::PRIVATE).is_err());
        assert!(check_mode(&config, TransparencyMode::DISABLED).is_ok());
    }
}
