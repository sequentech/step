// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The deployment's Vitrine configuration: who approves the policies of its
//! election events, and who witnesses, monitors, mirrors and anchors their
//! logs. Harvest drafts each event's policy from it and from the event
//! (Vitrine's `specs/policy.md`).

use anyhow::{anyhow, bail, ensure, Context, Result};
use electoral_log::vitrine::entry::check_id;
use electoral_log::vitrine::Vkey;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;
use strum_macros::{Display, EnumString};

use super::quorum::{party_split_cost, split_cost, Node, QuorumTree};

/// The deployment configuration file.
pub const CONFIG_FILE_ENV: &str = "HARVEST_VITRINE_CONFIG_FILE";

/// Key types of signed-note verifier keys.
const ALG_ED25519: u8 = 0x01;
const ALG_COSIGNATURE: u8 = 0x04;
const MAX_NAME: usize = 64;
const MAX_URL: usize = 512;
const HTTPS: &str = "https";
const HTTP: &str = "http";
const LOCAL_HOSTS: [&str; 3] = ["localhost", "127.0.0.1", "[::1]"];
const MAX_MMD: u64 = 3600;
/// A hub refuses a declaration with more witnesses (policy.md, rule 16).
const MAX_HUB_WITNESSES: usize = 31;
const PROFILE_HASH_PREFIX: &str = "sha256:";
const POLICY_ID_PLACEHOLDER: &str = "{policy_id}";
/// The place of a log's path (`<tenant id>/<event id>/<log>`) in a mirror URL.
pub const MIRROR_PATH_PLACEHOLDER: &str = "{path}";
pub const WORD_LISTS: [&str; 10] = [
    "en", "es", "fr", "it", "pt", "cs", "ja", "ko", "zh-hans", "zh-hant",
];

/// What an authority does for the deployment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Display, EnumString)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum AuthorityRole {
    /// The Electoral Board: it seals each event's policy and announces its
    /// words.
    Board,
    /// The operator: windmill signs the `events` statements with its key.
    Operator,
    Other,
}

/// Whether a hub takes the event's logs' anchors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Display, EnumString)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum HubRole {
    /// The logs anchor in it.
    #[default]
    App,
    /// It stands in for the browser layer: the other hubs anchor in it.
    Bootstrap,
}

/// A group's threshold: a number of members, `any` or `all`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Threshold {
    Count(u64),
    Any,
    All,
}

impl Threshold {
    pub fn k(&self, members: usize) -> usize {
        match self {
            Threshold::Count(k) => usize::try_from(*k).unwrap_or(usize::MAX),
            Threshold::Any => 1,
            Threshold::All => members,
        }
    }
}

impl std::fmt::Display for Threshold {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Threshold::Count(k) => write!(f, "{k}"),
            Threshold::Any => f.write_str("any"),
            Threshold::All => f.write_str("all"),
        }
    }
}

impl<'de> Deserialize<'de> for Threshold {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Count(u64),
            Word(String),
        }
        match Raw::deserialize(d)? {
            Raw::Count(k) => Ok(Threshold::Count(k)),
            Raw::Word(w) if w == "any" => Ok(Threshold::Any),
            Raw::Word(w) if w == "all" => Ok(Threshold::All),
            Raw::Word(w) => Err(serde::de::Error::custom(format!(
                "a threshold is a number, any or all, not {w:?}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Authority {
    pub name: String,
    pub vkey: String,
    pub role: AuthorityRole,
}

/// `k` of the listed authorities.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorityThreshold {
    pub threshold: u64,
    pub authorities: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Auditor {
    pub name: String,
    pub vkey: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Witness {
    pub name: String,
    /// A cosignature/v1 key.
    pub vkey: String,
    pub url: Option<String>,
    /// Who runs it; its own name if absent.
    pub party: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Group {
    pub name: String,
    pub threshold: Threshold,
    pub members: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Monitor {
    pub name: String,
    pub vkey: String,
    pub url: String,
    pub party: Option<String>,
}

/// A mirror of the event logs' tiles.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mirror {
    /// Its URL for a log, with `{path}` for the log's path on the sidecar.
    pub url: String,
    /// Who runs it; the operator if absent.
    pub party: Option<String>,
    /// The policy names of the logs it mirrors; both if absent.
    pub logs: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hub {
    pub name: String,
    pub vkey: String,
    pub url: String,
    pub operator: String,
    #[serde(default)]
    pub role: HubRole,
    pub witnesses: Vec<Witness>,
    #[serde(default)]
    pub groups: Vec<Group>,
    pub quorum: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Anchor {
    pub period: u64,
    pub max_gap: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnchorHubs {
    pub min: u64,
    pub min_independent: u64,
}

/// The meaning hashes of the profiles of an event's logs, as the policy
/// pins them.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profiles {
    pub election: String,
    pub events: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeploymentConfig {
    /// The logs' origins are `<domain>/<tenant id>/<event id>/<log>`.
    pub domain: String,
    /// The party that runs the logs.
    pub operator: String,
    /// The app quorum: a group or a witness.
    pub quorum: String,
    /// Every quorum tolerates this many more faulty parties.
    pub min_faults: Option<u64>,
    /// The language of policy and receipt words.
    pub words: Option<String>,
    /// Maximum merge delay of promises, in seconds.
    pub mmd: Option<u64>,
    /// The explorer's page for a policy, with `{policy_id}` in its place.
    pub explorer_url: Option<String>,
    pub monitor_quorum: Option<u64>,
    /// How often hubs anchor in the browser layer, in seconds.
    pub browser_period: Option<u64>,
    pub profiles: Profiles,
    pub sign: AuthorityThreshold,
    pub redaction: Option<AuthorityThreshold>,
    pub anchor: Option<Anchor>,
    pub anchor_hubs: Option<AnchorHubs>,
    pub authorities: Vec<Authority>,
    #[serde(default)]
    pub auditors: Vec<Auditor>,
    pub witnesses: Vec<Witness>,
    #[serde(default)]
    pub groups: Vec<Group>,
    #[serde(default)]
    pub monitors: Vec<Monitor>,
    #[serde(default)]
    pub mirrors: Vec<Mirror>,
    #[serde(default)]
    pub hubs: Vec<Hub>,
}

/// The default maximum merge delay of the policies step drafts.
pub const DEFAULT_MMD: u64 = 30;

/// A name of a log, witness, group, hub, monitor, authority or party.
pub fn check_name(what: &str, name: &str) -> Result<()> {
    ensure!(
        !name.is_empty()
            && name.len() <= MAX_NAME
            && name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
            && name != "none",
        "{what} {name:?} isn't 1 to 64 bytes of A-Z a-z 0-9 . _ -"
    );
    Ok(())
}

/// An absolute `https` URL, or `http` on `localhost`, `127.0.0.1` or `[::1]`
/// (policy.md, section 2).
pub fn check_url(what: &str, url: &str) -> Result<()> {
    ensure!(
        url.len() <= MAX_URL && url.bytes().all(|b| b.is_ascii_graphic()),
        "{what} {url:?} isn't a URL"
    );
    let (scheme, rest) = url
        .split_once("://")
        .ok_or_else(|| anyhow!("{what} {url:?} isn't an absolute URL"))?;
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    ensure!(
        !authority.is_empty() && !authority.contains('@'),
        "{what} {url:?} has no host, or has user information"
    );
    let host = match authority.strip_prefix('[') {
        Some(v6) => v6
            .split_once(']')
            .map(|(address, _)| format!("[{address}]"))
            .ok_or_else(|| anyhow!("{what} {url:?} has an invalid host"))?,
        None => authority.split(':').next().unwrap_or_default().to_string(),
    };
    match scheme {
        HTTPS => Ok(()),
        HTTP if LOCAL_HOSTS.contains(&host.as_str()) => Ok(()),
        HTTP => bail!("{what} {url:?} is http on a host other than localhost"),
        _ => bail!("{what} {url:?} isn't an https URL"),
    }
}

fn check_vkey(what: &str, vkey: &str, alg: u8) -> Result<Vkey> {
    let parsed = Vkey::parse(vkey).with_context(|| format!("{what}'s vkey"))?;
    ensure!(
        parsed.alg == alg && parsed.key.len() == 32,
        "{what}'s vkey isn't {}",
        if alg == ALG_ED25519 {
            "an Ed25519 key"
        } else {
            "a cosignature/v1 key"
        }
    );
    Ok(parsed)
}

fn check_meaning_hash(what: &str, hash: &str) -> Result<()> {
    let hex = hash.strip_prefix(PROFILE_HASH_PREFIX).unwrap_or_default();
    ensure!(
        hex.len() == 64
            && hex
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "{what} is sha256: and 64 lowercase hex digits"
    );
    Ok(())
}

/// The quorum tree of a set of witnesses and groups.
fn quorum_tree(
    what: &str,
    witnesses: &[Witness],
    groups: &[Group],
    quorum: &str,
    names: &mut BTreeSet<String>,
) -> Result<QuorumTree> {
    let mut tree = QuorumTree::default();
    for w in witnesses {
        check_name("a witness", &w.name)?;
        ensure!(
            names.insert(w.name.clone()),
            "{what}: {} is defined twice",
            w.name
        );
        let party = w.party().to_string();
        check_name("a party", &party)?;
        if let Some(url) = &w.url {
            check_url("a witness URL", url)?;
        }
        tree.nodes.insert(w.name.clone(), Node::Witness { party });
    }
    let mut grouped = BTreeSet::new();
    for g in groups {
        check_name("a group", &g.name)?;
        ensure!(
            names.insert(g.name.clone()),
            "{what}: {} is defined twice",
            g.name
        );
        ensure!(
            !g.members.is_empty(),
            "{what}: group {} has no members",
            g.name
        );
        let k = g.threshold.k(g.members.len());
        ensure!(
            k >= 1 && k <= g.members.len(),
            "{what}: group {} has threshold {} of {} members",
            g.name,
            g.threshold,
            g.members.len()
        );
        for m in &g.members {
            ensure!(
                tree.nodes.contains_key(m),
                "{what}: group {} names {m} before it's defined",
                g.name
            );
            ensure!(
                grouped.insert(m.clone()),
                "{what}: {m} is a member of more than one group, or twice of one"
            );
        }
        tree.nodes.insert(
            g.name.clone(),
            Node::Group {
                k,
                members: g.members.clone(),
            },
        );
    }
    ensure!(
        tree.nodes.contains_key(quorum),
        "{what}: the quorum {quorum} is neither a group nor a witness"
    );
    tree.root = quorum.to_string();
    Ok(tree)
}

/// Rule 5: with the faulty parties' witnesses counted as faulty, a quorum's
/// split cost and party split cost are at least 1, and the party split cost
/// at least `min_faults + 1`.
fn check_quorum_safety(
    what: &str,
    tree: &QuorumTree,
    faulty: &BTreeSet<String>,
    min_faults: u64,
) -> Result<()> {
    let cost = split_cost(tree, faulty);
    ensure!(
        cost >= 1,
        "{what}: two inconsistent checkpoints could both meet the quorum with only its faulty parties' witnesses (split cost 0)"
    );
    let parties = party_split_cost(tree, faulty);
    ensure!(
        parties as u64 > min_faults,
        "{what}: the quorum tolerates {} more faulty parties, and min_faults asks for {min_faults}",
        parties.saturating_sub(1)
    );
    Ok(())
}

impl DeploymentConfig {
    pub fn parse(text: &str) -> Result<Self> {
        let config: DeploymentConfig =
            toml::from_str(text).context("Error parsing the Vitrine deployment configuration")?;
        config.validate()?;
        Ok(config)
    }

    pub fn load(path: &str) -> Result<Self> {
        let text = std::fs::read_to_string(path).with_context(|| {
            format!("Error reading the Vitrine deployment configuration {path}")
        })?;
        Self::parse(&text)
    }

    /// The configuration named by `HARVEST_VITRINE_CONFIG_FILE`.
    pub fn from_env() -> Result<Self> {
        let path =
            std::env::var(CONFIG_FILE_ENV).map_err(|_| anyhow!("{CONFIG_FILE_ENV} isn't set"))?;
        Self::load(&path)
    }

    pub fn mmd(&self) -> u64 {
        self.mmd.unwrap_or(DEFAULT_MMD)
    }

    /// The hubs the logs anchor in.
    pub fn app_hubs(&self) -> impl Iterator<Item = &Hub> {
        self.hubs.iter().filter(|h| h.role == HubRole::App)
    }

    /// The explorer page of a policy, if the deployment has an explorer.
    pub fn explorer_url(&self, policy_id: &str) -> Option<String> {
        self.explorer_url
            .as_ref()
            .map(|template| template.replace(POLICY_ID_PLACEHOLDER, policy_id))
    }

    /// The authority whose key signs the `events` statements.
    pub fn operator_authority(&self) -> Option<&Authority> {
        self.authorities
            .iter()
            .find(|a| a.role == AuthorityRole::Operator)
    }

    /// Checks the rules of `specs/policy.md`, section 3, that the
    /// configuration decides, so that every draft made from it is valid.
    pub fn validate(&self) -> Result<()> {
        ensure!(
            !self.domain.is_empty()
                && self
                    .domain
                    .bytes()
                    .all(|b| b.is_ascii_graphic() && b != b'+')
                && !self.domain.starts_with("vitrine/")
                && !self.domain.starts_with("cosignature/"),
            "domain {:?} can't start a log origin",
            self.domain
        );
        check_name("the operator", &self.operator)?;
        check_meaning_hash("profiles.election", &self.profiles.election)?;
        check_meaning_hash("profiles.events", &self.profiles.events)?;
        if let Some(words) = &self.words {
            ensure!(
                WORD_LISTS.contains(&words.as_str()),
                "words {words:?} isn't one of {}",
                WORD_LISTS.join(", ")
            );
        }
        ensure!(self.mmd() <= MAX_MMD, "mmd is at most {MAX_MMD} seconds");
        if let Some(url) = &self.explorer_url {
            ensure!(
                url.contains(POLICY_ID_PLACEHOLDER),
                "explorer_url has no {POLICY_ID_PLACEHOLDER}"
            );
        }

        // Rule 1a: no key appears twice anywhere.
        let mut keys: BTreeMap<Vec<u8>, String> = BTreeMap::new();
        let mut claim = |what: String, key: &Vkey| -> Result<()> {
            if let Some(other) = keys.insert(key.key.clone(), what.clone()) {
                bail!("{what} and {other} have the same key");
            }
            Ok(())
        };

        let mut authority_names = BTreeSet::new();
        for a in &self.authorities {
            check_name("an authority", &a.name)?;
            ensure!(
                authority_names.insert(a.name.clone()),
                "authority {} is defined twice",
                a.name
            );
            let key = check_vkey(&format!("authority {}", a.name), &a.vkey, ALG_ED25519)?;
            claim(format!("authority {}", a.name), &key)?;
        }
        ensure!(
            self.authorities
                .iter()
                .filter(|a| a.role == AuthorityRole::Operator)
                .count()
                == 1,
            "exactly one authority has the operator role: its key signs the events statements"
        );
        let boards: Vec<&Authority> = self
            .authorities
            .iter()
            .filter(|a| a.role == AuthorityRole::Board)
            .collect();
        ensure!(
            boards.len() == 1,
            "exactly one authority has the board role"
        );
        // Rule 9.
        let check_threshold = |what: &str, t: &AuthorityThreshold| -> Result<()> {
            let mut seen = BTreeSet::new();
            for name in &t.authorities {
                ensure!(
                    authority_names.contains(name),
                    "{what} names {name}, which isn't an authority"
                );
                ensure!(seen.insert(name), "{what} lists {name} twice");
            }
            ensure!(
                t.threshold >= 1 && t.threshold as usize <= t.authorities.len(),
                "{what} has threshold {} of {} authorities",
                t.threshold,
                t.authorities.len()
            );
            Ok(())
        };
        check_threshold("sign", &self.sign)?;
        if let Some(redaction) = &self.redaction {
            check_threshold("redaction", redaction)?;
        }
        // Rule 13: the Board seals, so `sign` lists it.
        ensure!(
            self.sign.authorities.contains(&boards[0].name),
            "sign doesn't list the board, which seals the policies"
        );

        let mut auditor_names = BTreeSet::new();
        for a in &self.auditors {
            check_name("an auditor", &a.name)?;
            ensure!(
                auditor_names.insert(a.name.clone()),
                "auditor {} is defined twice",
                a.name
            );
            let key = check_vkey(&format!("auditor {}", a.name), &a.vkey, ALG_ED25519)?;
            claim(format!("auditor {}", a.name), &key)?;
        }

        // Rules 1, 1b, 4 and 5 for the app quorum.
        let mut names = BTreeSet::new();
        let tree = quorum_tree(
            "the app quorum",
            &self.witnesses,
            &self.groups,
            &self.quorum,
            &mut names,
        )?;
        let mut witness_key_names = BTreeSet::new();
        for w in &self.witnesses {
            let key = check_vkey(&format!("witness {}", w.name), &w.vkey, ALG_COSIGNATURE)?;
            claim(format!("witness {}", w.name), &key)?;
            if self.app_hubs().next().is_some() {
                ensure!(
                    witness_key_names.insert(key.name.clone()),
                    "with app hubs, witnesses have distinct key names"
                );
            }
        }
        if self.app_hubs().next().is_some() {
            ensure!(
                self.witnesses.len() <= MAX_HUB_WITNESSES,
                "with app hubs, a log has at most {MAX_HUB_WITNESSES} witnesses"
            );
        }
        let min_faults = self.min_faults.unwrap_or(0);
        let operator_faulty = BTreeSet::from([self.operator.clone()]);
        check_quorum_safety("the app quorum", &tree, &operator_faulty, min_faults)?;

        let mut monitor_names = BTreeSet::new();
        for m in &self.monitors {
            check_name("a monitor", &m.name)?;
            ensure!(
                monitor_names.insert(m.name.clone()),
                "monitor {} is defined twice",
                m.name
            );
            check_url("a monitor URL", &m.url)?;
            if let Some(party) = &m.party {
                check_name("a party", party)?;
            }
            let key = check_vkey(&format!("monitor {}", m.name), &m.vkey, ALG_ED25519)?;
            claim(format!("monitor {}", m.name), &key)?;
        }
        if let Some(k) = self.monitor_quorum {
            ensure!(
                k as usize <= self.monitors.len(),
                "monitor_quorum is at most the number of monitors"
            );
        }

        let mut mirror_urls = BTreeSet::new();
        for m in &self.mirrors {
            ensure!(
                m.url.contains(MIRROR_PATH_PLACEHOLDER),
                "mirror {} has no {MIRROR_PATH_PLACEHOLDER}",
                m.url
            );
            check_url(
                "a mirror URL",
                &m.url.replace(MIRROR_PATH_PLACEHOLDER, "t/e/ballots"),
            )?;
            ensure!(
                mirror_urls.insert(&m.url),
                "mirror {} is listed twice",
                m.url
            );
            if let Some(party) = &m.party {
                check_name("a party", party)?;
            }
            for log in m.logs.iter().flatten() {
                super::LogName::from_str(log)
                    .map_err(|_| anyhow!("mirror {} names log {log}: ballots or events", m.url))?;
            }
        }

        let mut hub_names = BTreeSet::new();
        let mut hub_origins = BTreeSet::new();
        for h in &self.hubs {
            check_name("a hub", &h.name)?;
            ensure!(
                hub_names.insert(h.name.clone()),
                "hub {} is defined twice",
                h.name
            );
            check_url("a hub URL", &h.url)?;
            check_name("a party", &h.operator)?;
            let key = check_vkey(&format!("hub {}", h.name), &h.vkey, ALG_ED25519)?;
            ensure!(
                !key.name.starts_with("vitrine/") && !key.name.starts_with("cosignature/"),
                "hub {}'s origin can't start with vitrine/ or cosignature/",
                h.name
            );
            ensure!(
                hub_origins.insert(key.name.clone()),
                "hub {}'s origin {} is another hub's",
                h.name,
                key.name
            );
            claim(format!("hub {}", h.name), &key)?;
            let what = format!("hub {}'s quorum", h.name);
            let hub_tree = h.quorum_tree()?;
            let mut hub_key_names = BTreeSet::new();
            for w in &h.witnesses {
                let key = check_vkey(
                    &format!("hub {} witness {}", h.name, w.name),
                    &w.vkey,
                    ALG_COSIGNATURE,
                )?;
                claim(format!("hub {} witness {}", h.name, w.name), &key)?;
                ensure!(
                    hub_key_names.insert(key.name.clone()),
                    "hub {}'s witnesses have distinct key names",
                    h.name
                );
            }
            let faulty = BTreeSet::from([self.operator.clone(), h.operator.clone()]);
            check_quorum_safety(&what, &hub_tree, &faulty, min_faults)?;
        }
        if let Some(a) = &self.anchor {
            ensure!(a.period >= 1, "the anchor period is at least one second");
        }
        self.check_bootstrap_hubs()?;
        if let Some(rule) = &self.anchor_hubs {
            self.check_anchor_hubs(rule)?;
        }
        Ok(())
    }

    /// Rule 6: two hubs are independent of each other when their operators
    /// differ and each one's quorum keeps a split cost of at least 1 with the
    /// parties they share counted as faulty, besides its own faulty parties.
    fn hubs_independent(&self, a: &Hub, b: &Hub) -> Result<bool> {
        if a.operator == b.operator {
            return Ok(false);
        }
        let shared: BTreeSet<String> = a.parties().intersection(&b.parties()).cloned().collect();
        for hub in [a, b] {
            let mut faulty = shared.clone();
            faulty.insert(self.operator.clone());
            faulty.insert(hub.operator.clone());
            if split_cost(&hub.quorum_tree()?, &faulty) == 0 {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// A bootstrap hub is independent, and independent of each app hub.
    fn check_bootstrap_hubs(&self) -> Result<()> {
        for b in self.hubs.iter().filter(|h| h.role == HubRole::Bootstrap) {
            ensure!(
                b.operator != self.operator,
                "bootstrap hub {} is run by the operator",
                b.name
            );
            for h in self.app_hubs() {
                ensure!(
                    self.hubs_independent(b, h)?,
                    "bootstrap hub {} isn't independent of app hub {}",
                    b.name,
                    h.name
                );
            }
        }
        Ok(())
    }

    /// Rule 6: the app hubs include `min` hubs that are pairwise independent
    /// of each other, `min_independent` of them independent of the operator.
    fn check_anchor_hubs(&self, rule: &AnchorHubs) -> Result<()> {
        let hubs: Vec<&Hub> = self.app_hubs().collect();
        let mut linked = vec![vec![false; hubs.len()]; hubs.len()];
        for (i, a) in hubs.iter().enumerate() {
            for (j, b) in hubs.iter().enumerate().skip(i + 1) {
                let independent = self.hubs_independent(a, b)?;
                linked[i][j] = independent;
                linked[j][i] = independent;
            }
        }
        let independent: Vec<bool> = hubs.iter().map(|h| h.operator != self.operator).collect();
        let size = rule.min.max(rule.min_independent) as usize;
        let found = find_hubs(
            &linked,
            &independent,
            &mut Vec::new(),
            0,
            size,
            rule.min_independent as usize,
        );
        ensure!(
            found,
            "anchor_hubs asks for {} app hubs independent of each other, {} of them independent of the operator, and the hubs don't have them",
            rule.min,
            rule.min_independent
        );
        Ok(())
    }

    /// A log's origin, `<domain>/<tenant id>/<event id>/<log>`.
    pub fn origin(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        log: super::LogName,
    ) -> Result<String> {
        let origin = format!(
            "{}/{}",
            self.domain,
            super::sidecar_name(tenant_id, election_event_id, log)?
        );
        check_id(&origin).map_err(|e| anyhow!("the log origin {origin:?}: {e}"))?;
        Ok(origin)
    }
}

/// Looks for `size` hubs, pairwise linked, `need_independent` of them
/// independent, extending `chosen` with hubs from `from` on.
fn find_hubs(
    linked: &[Vec<bool>],
    independent: &[bool],
    chosen: &mut Vec<usize>,
    from: usize,
    size: usize,
    need_independent: usize,
) -> bool {
    if chosen.len() >= size {
        return chosen.iter().filter(|&&i| independent[i]).count() >= need_independent;
    }
    for i in from..linked.len() {
        if chosen.iter().all(|&c| linked[c][i]) {
            chosen.push(i);
            let found = find_hubs(linked, independent, chosen, i + 1, size, need_independent);
            chosen.pop();
            if found {
                return true;
            }
        }
    }
    false
}

impl Witness {
    pub fn party(&self) -> &str {
        self.party.as_deref().unwrap_or(&self.name)
    }
}

impl Hub {
    /// Its operator and its witnesses' parties.
    fn parties(&self) -> BTreeSet<String> {
        let mut parties: BTreeSet<String> = self
            .witnesses
            .iter()
            .map(|w| w.party().to_string())
            .collect();
        parties.insert(self.operator.clone());
        parties
    }

    fn quorum_tree(&self) -> Result<QuorumTree> {
        quorum_tree(
            &format!("hub {}'s quorum", self.name),
            &self.witnesses,
            &self.groups,
            &self.quorum,
            &mut BTreeSet::new(),
        )
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) const DEV_CONFIG: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../.devcontainer/vitrine/deployment.toml"
    ));

    pub(crate) fn dev() -> DeploymentConfig {
        DeploymentConfig::parse(DEV_CONFIG).expect("the development configuration is valid")
    }

    fn with(edit: impl Fn(&mut toml::Table)) -> Result<DeploymentConfig> {
        let mut table: toml::Table = toml::from_str(DEV_CONFIG).unwrap();
        edit(&mut table);
        DeploymentConfig::parse(&toml::to_string(&table).unwrap())
    }

    fn array<'a>(t: &'a mut toml::Table, key: &str) -> &'a mut Vec<toml::Value> {
        t.get_mut(key).and_then(|v| v.as_array_mut()).unwrap()
    }

    #[test]
    fn the_development_configuration_is_valid() {
        let config = dev();
        assert_eq!(
            config.operator_authority().unwrap().role,
            AuthorityRole::Operator
        );
        assert_eq!(config.witnesses.len(), 2);
        assert_eq!(config.app_hubs().count(), 0);
        assert_eq!(config.mmd(), DEFAULT_MMD);
    }

    #[test]
    fn unknown_keys_are_refused() {
        assert!(with(|t| {
            t.insert("domian".into(), "x".into());
        })
        .is_err());
    }

    #[test]
    fn a_quorum_that_the_operator_alone_meets_is_refused() {
        let err = with(|t| {
            let witnesses = array(t, "witnesses");
            for w in witnesses.iter_mut() {
                w.as_table_mut()
                    .unwrap()
                    .insert("party".into(), "sequent".into());
            }
        })
        .unwrap_err();
        assert!(format!("{err:#}").contains("split cost 0"), "{err:#}");
    }

    #[test]
    fn a_group_with_a_majority_threshold_or_less_is_refused() {
        let err = with(|t| {
            let groups = array(t, "groups");
            groups[0]
                .as_table_mut()
                .unwrap()
                .insert("threshold".into(), "any".into());
        })
        .unwrap_err();
        assert!(format!("{err:#}").contains("split cost 0"), "{err:#}");
    }

    #[test]
    fn min_faults_needs_enough_parties() {
        let err = with(|t| {
            t.insert("min_faults".into(), 5.into());
        })
        .unwrap_err();
        assert!(format!("{err:#}").contains("min_faults"), "{err:#}");
    }

    #[test]
    fn a_key_used_twice_is_refused() {
        let err = with(|t| {
            let authorities = array(t, "authorities");
            let board = authorities[0].as_table().unwrap()["vkey"].clone();
            authorities[1]
                .as_table_mut()
                .unwrap()
                .insert("vkey".into(), board);
        })
        .unwrap_err();
        assert!(format!("{err:#}").contains("same key"), "{err:#}");
    }

    #[test]
    fn witness_keys_are_cosigner_keys() {
        let err = with(|t| {
            let board = array(t, "authorities")[0].as_table().unwrap()["vkey"].clone();
            array(t, "witnesses")[0]
                .as_table_mut()
                .unwrap()
                .insert("vkey".into(), board);
        })
        .unwrap_err();
        assert!(format!("{err:#}").contains("cosignature"), "{err:#}");
    }

    #[test]
    fn sign_must_list_the_board() {
        let err = with(|t| {
            let sign = t.get_mut("sign").unwrap().as_table_mut().unwrap();
            sign.insert("authorities".into(), vec!["sequent"].into());
        })
        .unwrap_err();
        assert!(format!("{err:#}").contains("board"), "{err:#}");
    }

    #[test]
    fn urls_are_https_except_on_localhost() {
        assert!(check_url("u", "https://log.example.org/").is_ok());
        assert!(check_url("u", "http://localhost:8094/").is_ok());
        assert!(check_url("u", "http://127.0.0.1/x").is_ok());
        assert!(check_url("u", "http://[::1]:80/").is_ok());
        assert!(check_url("u", "http://log.example.org/").is_err());
        assert!(check_url("u", "http://localhost.example.org/").is_err());
        assert!(check_url("u", "ftp://localhost/").is_err());
        assert!(check_url("u", "https://").is_err());
    }

    #[test]
    fn names_follow_the_policy_rule() {
        assert!(check_name("n", "w-1.example_a").is_ok());
        assert!(check_name("n", "").is_err());
        assert!(check_name("n", "a b").is_err());
        assert!(check_name("n", "a/b").is_err());
        assert!(check_name("n", "none").is_err());
        assert!(check_name("n", &"a".repeat(65)).is_err());
    }

    #[test]
    fn thresholds_read_numbers_and_words() {
        #[derive(Deserialize)]
        struct T {
            t: Threshold,
        }
        assert_eq!(toml::from_str::<T>("t = 2").unwrap().t, Threshold::Count(2));
        assert_eq!(
            toml::from_str::<T>("t = \"any\"").unwrap().t,
            Threshold::Any
        );
        assert_eq!(
            toml::from_str::<T>("t = \"all\"").unwrap().t,
            Threshold::All
        );
        assert!(toml::from_str::<T>("t = \"most\"").is_err());
    }

    #[test]
    fn anchor_hubs_needs_independent_hubs() {
        let err = with(|t| {
            let mut rule = toml::Table::new();
            rule.insert("min".into(), 3.into());
            rule.insert("min_independent".into(), 1.into());
            t.insert("anchor_hubs".into(), rule.into());
        })
        .unwrap_err();
        assert!(format!("{err:#}").contains("anchor_hubs"), "{err:#}");
    }

    #[test]
    fn the_explorer_url_names_the_policy() {
        assert_eq!(dev().explorer_url("sha256:ab"), None);
        let config = with(|t| {
            t.insert(
                "explorer_url".into(),
                "https://explorer.example/policy/{policy_id}".into(),
            );
        })
        .unwrap();
        assert_eq!(
            config.explorer_url("sha256:ab").as_deref(),
            Some("https://explorer.example/policy/sha256:ab")
        );
        assert!(with(|t| {
            t.insert("explorer_url".into(), "https://explorer.example/".into());
        })
        .is_err());
    }
}
