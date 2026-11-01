// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The hubs of an event's trust policy that give voters a site of their own
//! (its `hub-explorer` lines), and the links the cast ballot receipt gives
//! into them (design decision D7). Each link carries the policy ID and the
//! ballot's full Tracking ID (`watch-id`) in its fragment, which a browser never sends to the
//! hub's server; the hub's site asks the voter for an address and for what to
//! watch. Windmill sends nothing and learns no address.
//!
//! Adapter: the vendored Vitrine package has no builder for the watch link
//! yet, and the voting portal builds the same link in
//! `services/vitrine/hubUpdates.ts`. Both are the place to swap when it has.

use crate::postgres::vitrine::VitrineEvent;
use rand::{seq::SliceRandom, Rng};
use reqwest::Url;
use sequent_core::ballot::{ElectionEventPresentation, HubUpdatesPolicy};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A hub that has a site, as its policy lines name it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HubSite {
    pub name: String,
    /// Who runs the hub, as its `hub-operator` line says.
    pub operator: String,
    pub site: Url,
}

/// A hub as the receipt template prints it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HubLink {
    pub name: String,
    pub operator: String,
    /// The hub's site, with the policy ID and the Tracking ID in its fragment.
    pub url: String,
}

/// The hubs of a policy note that have a site, in the order of the policy.
/// Only the policy's text is read, not the signatures after it. A hub whose
/// site isn't a plain HTTP(S) address, or that has no operator, is left out.
pub fn hub_sites(note: &str) -> Vec<HubSite> {
    let text = note.split("\n\n").next().unwrap_or_default();
    let mut names = Vec::new();
    let mut operators = HashMap::new();
    let mut explorers = HashMap::new();
    for line in text.lines() {
        let words: Vec<&str> = line.split(' ').collect();
        match words.as_slice() {
            ["hub", name, ..] => names.push(*name),
            ["hub-operator", name, operator] => {
                operators.entry(*name).or_insert(*operator);
            }
            ["hub-explorer", name, site] => {
                explorers.entry(*name).or_insert(*site);
            }
            _ => {}
        }
    }
    names
        .into_iter()
        .filter_map(|name| {
            Some(HubSite {
                name: name.to_string(),
                operator: operators.get(name)?.to_string(),
                site: plain_http_address(explorers.get(name)?)?,
            })
        })
        .collect()
}

/// An address to send voters to: HTTP(S), with a host and no credentials.
fn plain_http_address(site: &str) -> Option<Url> {
    Url::parse(site).ok().filter(|url| {
        matches!(url.scheme(), "http" | "https")
            && url.host_str().is_some()
            && url.username().is_empty()
            && url.password().is_none()
    })
}

/// A value as `URLSearchParams` writes it, which is how the voting portal
/// writes the same link.
fn encode_component(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' | b'*' | b'-' | b'.' | b'_' => {
                char::from(byte).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

/// `<site>#policy=<sha256:hex>&watch-id=<full Tracking ID>` (D7); the site's
/// own fragment, if it has one, is replaced. The Tracking ID keeps its
/// hyphens. A link that carries a receipt uses `watch=<base64url receipt>`
/// instead, which only the voting portal can build: windmill has no receipt.
pub fn watch_link(site: &Url, policy_id: &str, tracking_id: &str) -> String {
    let mut link = site.clone();
    link.set_fragment(Some(&format!(
        "policy={}&watch-id={}",
        encode_component(policy_id),
        encode_component(tracking_id)
    )));
    link.into()
}

/// The links into the policy's hubs, in a random order: no hub comes first
/// more often than another.
pub fn hub_links<R: Rng + ?Sized>(
    note: &str,
    policy_id: &str,
    tracking_id: &str,
    rng: &mut R,
) -> Vec<HubLink> {
    let mut links: Vec<HubLink> = hub_sites(note)
        .into_iter()
        .map(|hub| HubLink {
            url: watch_link(&hub.site, policy_id, tracking_id),
            name: hub.name,
            operator: hub.operator,
        })
        .collect();
    links.shuffle(rng);
    links
}

/// The hubs a cast ballot receipt offers: those of the event's policy, when
/// the event is transparent and its admin has not turned off the cast email's
/// hub updates (`presentation.hub_updates.cast_email`, on unless set `off`).
/// Nothing without a policy, a hub that has a site, or the ballot's Tracking
/// ID.
pub fn hubs_for_receipt<R: Rng + ?Sized>(
    presentation: &ElectionEventPresentation,
    vitrine: Option<&VitrineEvent>,
    tracking_id: Option<&str>,
    rng: &mut R,
) -> Vec<HubLink> {
    if !presentation.transparency_mode().is_enabled()
        || presentation.hub_updates().cast_email() == HubUpdatesPolicy::OFF
    {
        return Vec::new();
    }
    let (
        Some(VitrineEvent {
            policy: Some(policy),
            policy_id: Some(policy_id),
            ..
        }),
        Some(tracking_id),
    ) = (vitrine, tracking_id)
    else {
        return Vec::new();
    };
    hub_links(policy, policy_id, tracking_id, rng)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{rngs::StdRng, SeedableRng};
    use sequent_core::ballot::{HubUpdatesPresentation, TransparencyMode};
    use std::collections::BTreeSet;

    const TRACKING_ID: &str = "FTBE-MHRX-4QKC-D2PA-77ZG-N3WJ-E9TS-VY5B-8HDM-6ULA-RC2X-K4NF-QP9T";

    fn policy_id() -> String {
        format!("sha256:{}", "ab".repeat(32))
    }

    /// A policy's text with three hubs (two with a site), a monitor and the
    /// operator's explorer, then the blank line and a signature line that
    /// must not be read.
    fn note() -> String {
        [
            "vitrine/policy/v1",
            "visibility public",
            "operator sequent",
            "explorer https://operator.explorer.example/",
            "monitor-explorer observers https://monitor.explorer.example/",
            "hub h1 h1.example+aabbccdd+AQ== https://hub1.example/",
            "hub-operator h1 hubco1",
            "hub-explorer h1 https://hub1.explorer.example/",
            "hub h2 h2.example+aabbccdd+AQ== https://hub2.example/",
            "hub-operator h2 hubco2",
            "hub-explorer h2 https://hub2.explorer.example/path?lang=es#old-fragment",
            "hub h3 h3.example+aabbccdd+AQ== https://hub3.example/",
            "hub-operator h3 hubco3",
            "",
            "hub-explorer h3 https://after-the-blank-line.example/",
            "— signature line",
            "",
        ]
        .join("\n")
    }

    fn names(hubs: &[HubSite]) -> Vec<&str> {
        hubs.iter().map(|hub| hub.name.as_str()).collect()
    }

    #[test]
    fn hubs_with_a_site_are_listed_in_policy_order_with_their_operator() {
        let hubs = hub_sites(&note());
        assert_eq!(names(&hubs), ["h1", "h2"]);
        assert_eq!(hubs[0].operator, "hubco1");
        assert_eq!(hubs[0].site.as_str(), "https://hub1.explorer.example/");
        assert_eq!(hubs[1].operator, "hubco2");
        assert_eq!(
            hubs[1].site.as_str(),
            "https://hub2.explorer.example/path?lang=es#old-fragment"
        );
    }

    #[test]
    fn a_hub_run_by_the_election_authority_is_listed_like_any_other() {
        let text = note().replace("hub-operator h1 hubco1", "hub-operator h1 sequent");
        let hubs = hub_sites(&text);
        assert_eq!(hubs[0].operator, "sequent");
        assert_eq!(names(&hubs), ["h1", "h2"]);
    }

    #[test]
    fn explorers_of_monitors_and_of_the_operator_are_not_hubs() {
        let text = [
            "vitrine/policy/v1",
            "explorer https://operator.explorer.example/",
            "monitor-explorer observers https://monitor.explorer.example/",
            "hub h1 h1.example+aabbccdd+AQ== https://hub1.example/",
            "hub-operator h1 hubco1",
        ]
        .join("\n");
        assert_eq!(hub_sites(&text), []);
    }

    #[test]
    fn nothing_after_the_policy_text_is_read() {
        assert!(!hub_sites(&note()).iter().any(|hub| hub.name == "h3"));
    }

    #[test]
    fn a_site_that_is_not_a_plain_http_address_is_left_out() {
        for site in [
            "javascript:alert(1)",
            "ftp://hub1.explorer.example/",
            "https://user:secret@hub1.explorer.example/",
            "https://",
            "mailto:hub@example.org",
            "not-an-address",
        ] {
            let text = note().replace("https://hub1.explorer.example/", site);
            assert_eq!(names(&hub_sites(&text)), ["h2"], "{site}");
        }
    }

    #[test]
    fn a_hub_without_an_operator_is_left_out() {
        let text = note().replace("hub-operator h1 hubco1\n", "");
        assert_eq!(names(&hub_sites(&text)), ["h2"]);
    }

    #[test]
    fn an_unreadable_note_has_no_hubs() {
        assert_eq!(hub_sites(""), []);
        assert_eq!(hub_sites("vitrine/policy/v1\n"), []);
    }

    #[test]
    fn the_watch_link_carries_the_policy_id_and_the_tracking_id_in_the_fragment() {
        let site = Url::parse("https://hub1.explorer.example/").unwrap();
        let link = watch_link(&site, &policy_id(), TRACKING_ID);

        let (base, fragment) = link.split_once('#').unwrap();
        assert_eq!(base, "https://hub1.explorer.example/");
        let params: Vec<(&str, &str)> = fragment
            .split('&')
            .map(|pair| pair.split_once('=').unwrap())
            .collect();
        assert_eq!(params.len(), 2);
        // Spelled as the portal's URLSearchParams does.
        assert_eq!(
            params[0],
            ("policy", format!("sha256%3A{}", "ab".repeat(32)).as_str())
        );
        // The full Tracking ID with its hyphens, not a base64url payload of it.
        assert_eq!(params[1], ("watch-id", TRACKING_ID));
    }

    #[test]
    fn the_watch_link_replaces_the_fragment_of_the_site_and_keeps_its_path_and_query() {
        let site = Url::parse("https://hub2.explorer.example/path?lang=es#old-fragment").unwrap();
        let link = watch_link(&site, &policy_id(), TRACKING_ID);
        assert!(
            link.starts_with("https://hub2.explorer.example/path?lang=es#policy="),
            "{link}"
        );
        assert!(!link.contains("old-fragment"));
        assert_eq!(link.matches('#').count(), 1);
    }

    #[test]
    fn the_links_name_every_hub_that_has_a_site_once() {
        let mut rng = StdRng::seed_from_u64(7);
        let links = hub_links(&note(), &policy_id(), TRACKING_ID, &mut rng);
        let names: BTreeSet<_> = links.iter().map(|link| link.name.as_str()).collect();
        assert_eq!(links.len(), 2);
        assert_eq!(names, BTreeSet::from(["h1", "h2"]));
        for link in &links {
            let site = hub_sites(&note())
                .into_iter()
                .find(|hub| hub.name == link.name)
                .unwrap();
            assert_eq!(link.operator, site.operator);
            assert_eq!(link.url, watch_link(&site.site, &policy_id(), TRACKING_ID));
        }
    }

    #[test]
    fn the_links_come_in_a_random_order_and_the_same_seed_gives_the_same_one() {
        let order = |seed: u64| -> Vec<String> {
            let mut rng = StdRng::seed_from_u64(seed);
            hub_links(&note(), &policy_id(), TRACKING_ID, &mut rng)
                .into_iter()
                .map(|link| link.name)
                .collect()
        };
        assert_eq!(order(3), order(3));
        let firsts: BTreeSet<String> = (0..64).map(|seed| order(seed).remove(0)).collect();
        assert_eq!(firsts, BTreeSet::from(["h1".to_string(), "h2".to_string()]));
    }

    fn vitrine() -> VitrineEvent {
        VitrineEvent {
            policy_salt: None,
            policy: Some(note()),
            policy_id: Some(policy_id()),
        }
    }

    fn presentation(
        transparency: Option<TransparencyMode>,
        cast_email: Option<HubUpdatesPolicy>,
    ) -> ElectionEventPresentation {
        ElectionEventPresentation {
            transparency,
            hub_updates: Some(HubUpdatesPresentation {
                confirmation: None,
                cast_email,
            }),
            ..Default::default()
        }
    }

    fn receipt_hubs(
        presentation: &ElectionEventPresentation,
        vitrine: Option<&VitrineEvent>,
        tracking_id: Option<&str>,
    ) -> Vec<HubLink> {
        hubs_for_receipt(
            presentation,
            vitrine,
            tracking_id,
            &mut StdRng::seed_from_u64(1),
        )
    }

    #[test]
    fn a_transparent_event_offers_its_hubs_unless_the_admin_turned_the_email_off() {
        for mode in [TransparencyMode::PUBLIC, TransparencyMode::PRIVATE] {
            for cast_email in [None, Some(HubUpdatesPolicy::ON)] {
                let links = receipt_hubs(
                    &presentation(Some(mode), cast_email),
                    Some(&vitrine()),
                    Some(TRACKING_ID),
                );
                assert_eq!(links.len(), 2, "{mode:?} {cast_email:?}");
            }
        }
    }

    #[test]
    fn an_event_that_says_nothing_about_hub_updates_offers_its_hubs() {
        let presentation = ElectionEventPresentation {
            transparency: Some(TransparencyMode::PUBLIC),
            ..Default::default()
        };
        assert_eq!(
            receipt_hubs(&presentation, Some(&vitrine()), Some(TRACKING_ID)).len(),
            2
        );
    }

    #[test]
    fn nothing_is_offered_when_the_admin_turned_the_cast_email_off() {
        let off = presentation(Some(TransparencyMode::PUBLIC), Some(HubUpdatesPolicy::OFF));
        assert_eq!(receipt_hubs(&off, Some(&vitrine()), Some(TRACKING_ID)), []);
    }

    #[test]
    fn turning_the_confirmation_page_off_does_not_turn_the_cast_email_off() {
        let presentation = ElectionEventPresentation {
            transparency: Some(TransparencyMode::PUBLIC),
            hub_updates: Some(HubUpdatesPresentation {
                confirmation: Some(HubUpdatesPolicy::OFF),
                cast_email: None,
            }),
            ..Default::default()
        };
        assert_eq!(
            receipt_hubs(&presentation, Some(&vitrine()), Some(TRACKING_ID)).len(),
            2
        );
    }

    #[test]
    fn nothing_is_offered_when_the_event_is_not_transparent() {
        for mode in [None, Some(TransparencyMode::DISABLED)] {
            let presentation = presentation(mode, None);
            assert_eq!(
                receipt_hubs(&presentation, Some(&vitrine()), Some(TRACKING_ID)),
                []
            );
        }
    }

    #[test]
    fn nothing_is_offered_without_a_policy_a_policy_id_hubs_or_a_tracking_id() {
        let on = presentation(Some(TransparencyMode::PUBLIC), None);
        let without = |change: fn(&mut VitrineEvent)| {
            let mut event = vitrine();
            change(&mut event);
            receipt_hubs(&on, Some(&event), Some(TRACKING_ID))
        };
        assert_eq!(receipt_hubs(&on, None, Some(TRACKING_ID)), []);
        assert_eq!(without(|event| event.policy = None), []);
        assert_eq!(without(|event| event.policy_id = None), []);
        assert_eq!(
            without(|event| event.policy = Some(
                "vitrine/policy/v1\nexplorer https://operator.explorer.example/\n".to_string()
            )),
            []
        );
        assert_eq!(receipt_hubs(&on, Some(&vitrine()), None), []);
    }
}
