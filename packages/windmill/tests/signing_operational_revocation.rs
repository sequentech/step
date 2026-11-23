// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The positive revocation proof an operational device claim needs: every
//! certificate below the anchor has a complete list its issuer signed, with
//! an explicit next update that is still ahead, and no list revokes it. The
//! proof lasts thirty seconds at most, and never past a next update. A
//! missing, expired, open-ended, future, delta, forged or unreadable list
//! proves nothing, whatever the event's legacy "accept unchecked" setting.
//!
//! NEEDS `windmill::services::signing::certificates::operational_revocation_until`
//! to be `pub`: its only caller in the crate is behind the
//! `operational-signing-candidate` feature, which the default build and the
//! coverage profile don't enable.

#[path = "support/signing_pki.rs"]
mod signing_pki;

use chrono::{DateTime, Duration, TimeZone, Utc};
use openssl::x509::X509;
use signing_pki::{
    crl_signed_by, crl_with, delta_indicator, ec_key, issued, issuing_distribution_point, person,
    pki_now, Idp, Issued, Pki, Spec, CRL_URL, REVOKED_SERIAL, ROOT_CRL_URL,
};
use windmill::services::signing::certificates::{operational_revocation_until, StoredCrl};

const DAY: i64 = 86_400;

fn now() -> DateTime<Utc> {
    Utc.timestamp_opt(pki_now(), 0).unwrap()
}

/// `leaf`'s path to the test root.
fn path(leaf: &Issued) -> Vec<X509> {
    let pki = Pki::get();
    vec![
        leaf.cert.clone(),
        pki.individual_ca.cert.clone(),
        pki.root.cert.clone(),
    ]
}

/// A list downloaded from `url` an hour ago.
fn stored(url: &str, der: Vec<u8>) -> StoredCrl {
    StoredCrl {
        url: url.to_owned(),
        der: Some(der),
        fetched_at: now() - Duration::hours(1),
    }
}

/// The individual CA's list of `revoked`, dated `this_update` seconds from
/// now, with its next update `next_update` seconds from now.
fn individual_list(revoked: &[u32], this_update: i64, next_update: Option<i64>) -> Vec<u8> {
    let ca = &Pki::get().individual_ca;
    crl_with(
        ca,
        &ca.key,
        revoked,
        pki_now() + this_update,
        next_update.map(|seconds| pki_now() + seconds),
        &[],
    )
}

/// The root's empty list, with the same dates.
fn root_list(this_update: i64, next_update: Option<i64>) -> Vec<u8> {
    let root = &Pki::get().root;
    crl_with(
        root,
        &root.key,
        &[],
        pki_now() + this_update,
        next_update.map(|seconds| pki_now() + seconds),
        &[],
    )
}

/// The current lists of both issuers.
fn current() -> Vec<StoredCrl> {
    let pki = Pki::get();
    vec![
        stored(CRL_URL, pki.individual_crl()),
        stored(ROOT_CRL_URL, pki.root_crl()),
    ]
}

/// The proof for `leaf` with the root's current list and `lists` of the
/// individual CA.
fn proof_with(leaf: &Issued, lists: Vec<StoredCrl>) -> Option<DateTime<Utc>> {
    let mut crls = vec![stored(ROOT_CRL_URL, Pki::get().root_crl())];
    crls.extend(lists);
    operational_revocation_until(&path(leaf), &crls, now())
}

#[test]
fn the_proof_lasts_thirty_seconds_and_never_past_a_next_update() {
    let pki = Pki::get();
    let official = &pki.jose;
    // Both lists are current for days: the proof is the short maximum.
    assert_eq!(
        operational_revocation_until(&path(official), &current(), now()),
        Some(now() + Duration::seconds(30))
    );
    // The signer's list runs out first.
    assert_eq!(
        proof_with(
            official,
            vec![stored(CRL_URL, individual_list(&[], -DAY, Some(10)))]
        ),
        Some(now() + Duration::seconds(10))
    );
    // Or the list of the signer's issuer.
    assert_eq!(
        operational_revocation_until(
            &path(official),
            &[
                stored(CRL_URL, individual_list(&[], -DAY, Some(10))),
                stored(ROOT_CRL_URL, root_list(-DAY, Some(5))),
            ],
            now()
        ),
        Some(now() + Duration::seconds(5))
    );
    // Of two lists that speak for a certificate, the later next update.
    assert_eq!(
        proof_with(
            official,
            vec![
                stored(CRL_URL, individual_list(&[], -2 * DAY, Some(10))),
                stored(CRL_URL, individual_list(&[], -DAY, Some(DAY))),
            ]
        ),
        Some(now() + Duration::seconds(30))
    );
}

#[test]
fn a_revoked_certificate_has_no_proof() {
    let pki = Pki::get();
    assert_eq!(
        operational_revocation_until(&path(&pki.revoked), &current(), now()),
        None
    );
    // A list that revokes it counts even beside a current one that doesn't,
    // and after its own next update.
    assert_eq!(
        proof_with(
            &pki.revoked,
            vec![
                stored(CRL_URL, individual_list(&[], -DAY, Some(DAY))),
                stored(
                    CRL_URL,
                    individual_list(&[REVOKED_SERIAL], -3 * DAY, Some(-2 * DAY))
                ),
            ]
        ),
        None
    );
    // The same lists are a proof for a certificate they don't name.
    assert!(operational_revocation_until(&path(&pki.jose), &current(), now()).is_some());
}

#[test]
fn a_list_that_is_missing_or_not_current_is_no_proof() {
    let pki = Pki::get();
    let official = &pki.jose;
    let none = |lists: Vec<StoredCrl>| assert_eq!(proof_with(official, lists), None);

    // No list of the signer's issuer.
    none(vec![]);
    // Past its next update, or exactly at it.
    none(vec![stored(
        CRL_URL,
        individual_list(&[], -2 * DAY, Some(-3600)),
    )]);
    none(vec![stored(CRL_URL, individual_list(&[], -DAY, Some(0)))]);
    // Without a next update: the legacy check takes it for a week.
    none(vec![stored(CRL_URL, individual_list(&[], -DAY, None))]);
    // Dated after now, or stored as downloaded after now.
    none(vec![stored(CRL_URL, individual_list(&[], 60, Some(DAY)))]);
    none(vec![StoredCrl {
        fetched_at: now() + Duration::seconds(1),
        ..stored(CRL_URL, pki.individual_crl())
    }]);
    // Downloaded from another point than the certificate names.
    none(vec![stored(
        "http://crl.staff-ca.invalid/other.crl",
        pki.individual_crl(),
    )]);
    // Not a list, or a row that keeps none.
    none(vec![stored(CRL_URL, b"not a revocation list".to_vec())]);
    none(vec![StoredCrl {
        der: None,
        ..stored(CRL_URL, vec![])
    }]);
    // Signed by someone else than the issuer.
    none(vec![stored(
        CRL_URL,
        crl_signed_by(
            &pki.individual_ca,
            &pki.ana.key,
            &[],
            pki_now() - DAY,
            Some(pki_now() + DAY),
        ),
    )]);
    // A delta list.
    none(vec![stored(
        CRL_URL,
        crl_with(
            &pki.individual_ca,
            &pki.individual_ca.key,
            &[],
            pki_now() - DAY,
            Some(pki_now() + DAY),
            &[delta_indicator(1)],
        ),
    )]);

    // The signer's list is current, the list of its issuer is not there.
    assert_eq!(
        operational_revocation_until(
            &path(official),
            &[stored(CRL_URL, pki.individual_crl())],
            now()
        ),
        None
    );
    // The proof is asked a week later than the lists reach.
    assert_eq!(
        operational_revocation_until(&path(official), &current(), now() + Duration::days(7)),
        None
    );
}

#[test]
fn a_path_without_an_issuer_has_no_proof() {
    let pki = Pki::get();
    // An anchor alone, a certificate alone, and nothing.
    assert_eq!(
        operational_revocation_until(&[pki.root.cert.clone()], &current(), now()),
        None
    );
    assert_eq!(
        operational_revocation_until(&[pki.jose.cert.clone()], &current(), now()),
        None
    );
    assert_eq!(operational_revocation_until(&[], &current(), now()), None);
    // A certificate directly under an issuer needs that issuer's list only.
    assert_eq!(
        operational_revocation_until(
            &[pki.jose.cert.clone(), pki.individual_ca.cert.clone()],
            &[stored(CRL_URL, pki.individual_crl())],
            now()
        ),
        Some(now() + Duration::seconds(30))
    );
}

#[test]
fn a_partitioned_list_is_no_proof_for_a_certificate_that_names_no_point() {
    let pki = Pki::get();
    // A signer whose certificate has no distribution point.
    let subject = person("Elena Navarro", "PH-0021");
    let unpointed = issued(
        &Spec {
            crl_url: None,
            ..Spec::signer(&subject, 1021)
        },
        ec_key(),
        Some(&pki.individual_ca),
    );
    let ca = &pki.individual_ca;
    let partitioned = crl_with(
        ca,
        &ca.key,
        &[],
        pki_now() - DAY,
        Some(pki_now() + DAY),
        &[issuing_distribution_point(&Idp {
            uri: Some(CRL_URL),
            ..Idp::default()
        })],
    );
    // The partition may not be the one the certificate is in.
    assert_eq!(
        proof_with(&unpointed, vec![stored(CRL_URL, partitioned.clone())]),
        None
    );
    // The issuer's whole list is a proof for it.
    assert_eq!(
        proof_with(&unpointed, vec![stored(CRL_URL, pki.individual_crl())]),
        Some(now() + Duration::seconds(30))
    );
    // And the partition is one for a certificate that names its point.
    assert_eq!(
        proof_with(&pki.jose, vec![stored(CRL_URL, partitioned)]),
        Some(now() + Duration::seconds(30))
    );
}
