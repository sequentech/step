// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Revocation lists of staff issuers (design §7). A list is downloaded from
//! a CRL distribution point, checked against the staff issuer that signed
//! it and stored in `staff_crl`; certificate checks read it from there.
//!
//! The hourly job refreshes the lists named by the staff issuers and by the
//! event's registered certificates, and the ones stored before. The
//! check-certificate dry run also downloads the missing or stale lists of
//! the chain it checks, so a certificate's first signature finds its list.
//!
//! Downloads run outside any transaction: a refresh reads what to download
//! in one short transaction, downloads, and stores each outcome in another.
//! No signing step writes `staff_crl`, so these writes don't take the
//! event's signing lock. A failed download is retried by the dry run only
//! after [`CRL_RETRY_AFTER_MINUTES`]; one download of a list runs at a time
//! in a process.
//!
//! Downloads only reach public addresses (unless the host is allowed by
//! `SIGNING_CRL_ALLOWED_HOSTS`, for on-premises deployments), don't follow
//! redirects, and are limited in size and time. Stored errors are generic.

use crate::postgres::certificate_authority::StaffIssuerRow;
use crate::postgres::signing_certificates::{
    list_active_staff_certificates, list_staff_crls, upsert_staff_crl, CrlDownload, StaffCrlRow,
};
use crate::services::signing::certificates::{
    asn1_time_to_utc, load_staff_anchors, parse_chain, verified_path,
};
use anyhow::{anyhow, bail, Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Duration as Span, TimeZone, Utc};
use deadpool_postgres::{Client, Transaction};
use openssl::x509::{X509Crl, X509Ref, X509};
use sequent_core::signing::CrlStatus;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use tracing::{info, instrument, warn};
use uuid::Uuid;
use x509_parser::der_parser::num_bigint::BigUint;
use x509_parser::extensions::{DistributionPointName, GeneralName, ParsedExtension};
use x509_parser::oid_registry::OID_X509_EXT_DELTA_CRL_INDICATOR;
use x509_parser::prelude::FromDer;
use x509_parser::revocation_list::CertificateRevocationList;

/// The largest list the server downloads.
pub const MAX_CRL_BYTES: usize = 20 * 1024 * 1024;
/// How long one download may take.
pub const CRL_FETCH_TIMEOUT: Duration = Duration::from_secs(15);
/// How long a list without a next update stays current after its this
/// update.
pub const CRL_MAX_AGE_DAYS: i64 = 7;
/// How far in the future a list's this update may be (clock skew).
pub const CRL_CLOCK_SKEW_MINUTES: i64 = 5;
/// How long the dry run waits before trying a failed download again.
pub const CRL_RETRY_AFTER_MINUTES: i64 = 5;
/// Hosts (comma separated) downloaded from even at a private address.
pub const ALLOWED_HOSTS_ENV: &str = "SIGNING_CRL_ALLOWED_HOSTS";

// Downloads

/// Downloads revocation lists; tests use fakes.
#[async_trait]
pub trait CrlFetcher: Send + Sync {
    async fn fetch(&self, url: &str) -> Result<Vec<u8>>;
}

/// Downloads over HTTP(S) from public addresses, without redirects, within
/// a size and a time limit.
#[derive(Debug, Clone)]
pub struct HttpCrlFetcher {
    /// Hosts downloaded from whatever their address (lowercase).
    pub allowed_hosts: Vec<String>,
    pub max_bytes: usize,
    pub timeout: Duration,
}

impl Default for HttpCrlFetcher {
    /// Public addresses, plus the hosts of `SIGNING_CRL_ALLOWED_HOSTS`.
    fn default() -> Self {
        HttpCrlFetcher {
            allowed_hosts: std::env::var(ALLOWED_HOSTS_ENV)
                .unwrap_or_default()
                .split(',')
                .map(|host| host.trim().to_lowercase())
                .filter(|host| !host.is_empty())
                .collect(),
            max_bytes: MAX_CRL_BYTES,
            timeout: CRL_FETCH_TIMEOUT,
        }
    }
}

/// Whether `ip` is a public unicast address: not loopback, private,
/// link-local (which holds cloud metadata services), unique-local, shared,
/// reserved, documentation, multicast or unspecified.
pub fn is_public_address(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => is_public_v4(ip),
        IpAddr::V6(ip) => match ip.to_ipv4_mapped() {
            Some(mapped) => is_public_v4(mapped),
            None => is_public_v6(ip),
        },
    }
}

fn is_public_v4(ip: Ipv4Addr) -> bool {
    let [a, b, c, _] = ip.octets();
    !(ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_unspecified()
        || ip.is_broadcast()
        || ip.is_multicast()
        || ip.is_documentation()
        || a == 0
        || (a == 100 && (64..128).contains(&b))
        || (a == 192 && b == 0 && c == 0)
        || (a == 198 && (18..20).contains(&b))
        || a >= 240)
}

fn is_public_v6(ip: Ipv6Addr) -> bool {
    let [first, second, ..] = ip.segments();
    !(ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_multicast()
        || (first & 0xfe00) == 0xfc00
        || (first & 0xffc0) == 0xfe80
        || (first == 0x2001 && second == 0x0db8))
}

#[async_trait]
impl CrlFetcher for HttpCrlFetcher {
    async fn fetch(&self, url: &str) -> Result<Vec<u8>> {
        let parsed = reqwest::Url::parse(url).map_err(|_| anyhow!("not a valid URL"))?;
        if !matches!(parsed.scheme(), "http" | "https") {
            bail!("only http and https distribution points are downloaded");
        }
        let host = parsed
            .host_str()
            .ok_or_else(|| anyhow!("not a valid URL"))?
            .to_lowercase();
        let port = parsed
            .port_or_known_default()
            .ok_or_else(|| anyhow!("not a valid URL"))?;
        let bare_host = host
            .trim_start_matches('[')
            .trim_end_matches(']')
            .to_owned();
        let addresses: Vec<SocketAddr> = tokio::net::lookup_host((bare_host.as_str(), port))
            .await
            .map_err(|_| anyhow!("unreachable"))?
            .collect();
        let allowed = self
            .allowed_hosts
            .iter()
            .any(|allowed| *allowed == bare_host);
        if addresses.is_empty()
            || (!allowed
                && addresses
                    .iter()
                    .any(|address| !is_public_address(address.ip())))
        {
            bail!("refused: the list's address is not public");
        }
        // Connect to the address checked above, not to a second lookup.
        let client = reqwest::Client::builder()
            .timeout(self.timeout)
            .redirect(reqwest::redirect::Policy::none())
            .resolve(&bare_host, addresses[0])
            .build()?;
        let mut response = client
            .get(parsed)
            .send()
            .await
            .map_err(|_| anyhow!("unreachable"))?;
        let status = response.status();
        if !status.is_success() {
            bail!("download failed: HTTP {}xx", status.as_u16() / 100);
        }
        let mut body = vec![];
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| anyhow!("download failed: the connection broke"))?
        {
            body.extend_from_slice(&chunk);
            if body.len() > self.max_bytes {
                bail!("the list is larger than {} bytes", self.max_bytes);
            }
        }
        Ok(body)
    }
}

// Lists

/// The URIs of a certificate's CRL distribution points.
pub fn distribution_points(certificate: &X509Ref) -> Vec<String> {
    let Some(points) = certificate.crl_distribution_points() else {
        return vec![];
    };
    points
        .iter()
        .filter_map(|point| point.distpoint())
        .filter_map(|name| name.fullname())
        .flat_map(|names| {
            names
                .iter()
                .filter_map(|name| name.uri().map(str::to_owned))
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Parses a list in DER or PEM.
pub fn parse_crl(bytes: &[u8]) -> Result<X509Crl> {
    X509Crl::from_der(bytes)
        .or_else(|_| X509Crl::from_pem(bytes))
        .map_err(|_| anyhow!("not a revocation list"))
}

/// What a list covers and when it is current.
#[derive(Debug, Clone, PartialEq)]
pub struct ListScope {
    /// A delta list: never proof that a certificate isn't revoked.
    pub delta: bool,
    /// A critical extension the server doesn't know: the list isn't used.
    pub unknown_critical: bool,
    pub only_user_certs: bool,
    pub only_ca_certs: bool,
    /// Only some reasons, an indirect list or attribute certificates only:
    /// not a complete list of the issuer's certificates.
    pub partial: bool,
    /// The issuing distribution point's URIs, if it names any.
    pub points: Vec<String>,
    pub this_update: DateTime<Utc>,
    pub next_update: Option<DateTime<Utc>>,
    pub number: Option<BigUint>,
}

fn utc(seconds: i64) -> Result<DateTime<Utc>> {
    Utc.timestamp_opt(seconds, 0)
        .single()
        .ok_or_else(|| anyhow!("time out of range"))
}

/// Reads a list's scope from its DER.
pub fn list_scope(der: &[u8]) -> Result<ListScope> {
    let (_, crl) =
        CertificateRevocationList::from_der(der).map_err(|_| anyhow!("not a revocation list"))?;
    let mut scope = ListScope {
        delta: false,
        unknown_critical: false,
        only_user_certs: false,
        only_ca_certs: false,
        partial: false,
        points: vec![],
        this_update: utc(crl.last_update().timestamp())?,
        next_update: crl
            .next_update()
            .map(|next| utc(next.timestamp()))
            .transpose()?,
        number: crl.crl_number().cloned(),
    };
    for extension in crl.extensions() {
        if extension.oid == OID_X509_EXT_DELTA_CRL_INDICATOR {
            scope.delta = true;
            continue;
        }
        match extension.parsed_extension() {
            ParsedExtension::IssuingDistributionPoint(point) => {
                scope.only_user_certs = point.only_contains_user_certs;
                scope.only_ca_certs = point.only_contains_ca_certs;
                scope.partial = point.only_some_reasons.is_some()
                    || point.indirect_crl
                    || point.only_contains_attribute_certs;
                if let Some(DistributionPointName::FullName(names)) = &point.distribution_point {
                    scope.points = names
                        .iter()
                        .filter_map(|name| match name {
                            GeneralName::URI(uri) => Some((*uri).to_owned()),
                            _ => None,
                        })
                        .collect();
                }
            }
            ParsedExtension::CRLNumber(_)
            | ParsedExtension::AuthorityKeyIdentifier(_)
            | ParsedExtension::IssuerAlternativeName(_) => {}
            _ if extension.critical => scope.unknown_critical = true,
            _ => {}
        }
    }
    Ok(scope)
}

impl ListScope {
    /// Whether the list speaks for every revocation of a certificate that
    /// is (or isn't) a CA and names the distribution `points`.
    pub fn covers(&self, certificate_is_ca: bool, points: &[String]) -> bool {
        !(self.delta
            || self.unknown_critical
            || self.partial
            || (self.only_user_certs && certificate_is_ca)
            || (self.only_ca_certs && !certificate_is_ca)
            || (!self.points.is_empty()
                && !points.is_empty()
                && !self.points.iter().any(|point| points.contains(point))))
    }

    /// Whether the list is current at `now`: issued (within the clock skew)
    /// and not past its next update, or, without one, younger than
    /// [`CRL_MAX_AGE_DAYS`].
    pub fn is_current(&self, now: DateTime<Utc>) -> bool {
        self.this_update <= now + Span::minutes(CRL_CLOCK_SKEW_MINUTES)
            && match self.next_update {
                Some(next_update) => next_update > now,
                None => self.this_update + Span::days(CRL_MAX_AGE_DAYS) > now,
            }
    }
}

fn names_equal(a: &openssl::x509::X509NameRef, b: &openssl::x509::X509NameRef) -> bool {
    matches!((a.to_der(), b.to_der()), (Ok(a), Ok(b)) if a == b)
}

/// Whether `issuer` issued `certificate` (by name and signature).
pub fn issued_by(certificate: &X509Ref, issuer: &X509Ref) -> bool {
    names_equal(certificate.issuer_name(), issuer.subject_name())
        && issuer
            .public_key()
            .and_then(|key| certificate.verify(&key))
            .unwrap_or(false)
}

/// Accepts `bytes` as `issuer`'s list if it is a complete (not delta) list
/// it signed, dated no later than `now` (within the clock skew), and not
/// older than the `previous` one, by this update or by number.
pub fn check_crl(
    bytes: &[u8],
    issuer: &X509Ref,
    previous: Option<&StaffCrlRow>,
    now: DateTime<Utc>,
) -> Result<CrlDownload> {
    let crl = parse_crl(bytes)?;
    if !names_equal(crl.issuer_name(), issuer.subject_name()) {
        bail!("the list is not issued by its issuer");
    }
    let key = issuer.public_key()?;
    if !crl.verify(&key).unwrap_or(false) {
        bail!("the list's signature doesn't verify with its issuer's key");
    }
    let der = crl.to_der()?;
    let scope = list_scope(&der)?;
    if scope.delta {
        bail!("a delta list is not accepted");
    }
    if scope.this_update > now + Span::minutes(CRL_CLOCK_SKEW_MINUTES) {
        bail!("the list is dated in the future");
    }
    if let Some(previous) = previous
        .and_then(|row| row.der.as_deref())
        .and_then(|der| list_scope(der).ok())
    {
        if scope.this_update < previous.this_update {
            bail!("the list is older than the stored one");
        }
        if let (Some(number), Some(previous)) = (&scope.number, &previous.number) {
            if number < previous {
                bail!("the list's number is lower than the stored one's");
            }
        }
    }
    Ok(CrlDownload::Ok {
        der,
        this_update: Some(asn1_time_to_utc(crl.last_update())?),
        next_update: crl.next_update().map(asn1_time_to_utc).transpose()?,
    })
}

/// SHA-256 of the issuer's DER, lowercase hex, as the staff tables write
/// hashes (certificate_authority keeps its own uppercase, colon form).
fn issuer_fingerprint(issuer: &X509Ref) -> Result<String> {
    Ok(hex::encode(Sha256::digest(issuer.to_der()?)))
}

// Refreshes

/// A list to download: its point, the staff issuer that signs it, and what
/// is stored of it.
#[derive(Clone)]
pub struct CrlTarget {
    pub url: String,
    pub issuer: StaffIssuerRow,
    pub issuer_certificate: X509,
    pub stored: Option<StaffCrlRow>,
}

/// Downloads `target`'s list and checks it. No database access.
pub async fn download(
    target: &CrlTarget,
    fetcher: &dyn CrlFetcher,
    now: DateTime<Utc>,
) -> CrlDownload {
    let outcome = match fetcher.fetch(&target.url).await {
        Ok(bytes) => check_crl(
            &bytes,
            &target.issuer_certificate,
            target.stored.as_ref(),
            now,
        ),
        Err(err) => Err(err),
    };
    outcome.unwrap_or_else(|err| {
        warn!(url = %target.url, error = %err, "revocation list unavailable");
        // The error's own line only: it never echoes a response body.
        CrlDownload::Unavailable {
            error: err.to_string(),
        }
    })
}

/// Stores a download of `target`.
pub async fn store_download(
    hasura_transaction: &Transaction<'_>,
    target: &CrlTarget,
    download: &CrlDownload,
) -> Result<StaffCrlRow> {
    upsert_staff_crl(
        hasura_transaction,
        target.issuer.tenant_id,
        target.issuer.election_event_id,
        target.issuer.id,
        &issuer_fingerprint(&target.issuer_certificate)?,
        &target.url,
        download,
    )
    .await
}

/// The staff issuer among `anchors` that issued `certificate`.
fn issuer_of<'a>(
    certificate: &X509Ref,
    anchors: &'a [(StaffIssuerRow, X509)],
) -> Option<&'a (StaffIssuerRow, X509)> {
    anchors
        .iter()
        .find(|(_, anchor)| issued_by(certificate, anchor))
}

fn target(
    url: String,
    (issuer, certificate): &(StaffIssuerRow, X509),
    stored: &[StaffCrlRow],
) -> CrlTarget {
    CrlTarget {
        stored: stored.iter().find(|row| row.url == url).cloned(),
        url,
        issuer: issuer.clone(),
        issuer_certificate: certificate.clone(),
    }
}

/// Every list of the event: the distribution points of its staff issuers
/// and registered certificates, and the lists stored before. A point whose
/// certificate no staff issuer issued is skipped: its list couldn't be
/// checked.
#[instrument(skip(hasura_transaction), err)]
pub async fn plan_event_crls(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
) -> Result<Vec<CrlTarget>> {
    let anchors = load_staff_anchors(hasura_transaction, tenant_id, election_event_id).await?;
    let stored = list_staff_crls(hasura_transaction, tenant_id, election_event_id).await?;
    let mut targets: BTreeMap<String, usize> = BTreeMap::new();
    let mut add = |certificate: &X509Ref| {
        let Some(issuer) = issuer_of(certificate, &anchors) else {
            return;
        };
        let index = anchors
            .iter()
            .position(|anchor| anchor.0.id == issuer.0.id)
            .unwrap_or_default();
        for url in distribution_points(certificate) {
            targets.entry(url).or_insert(index);
        }
    };
    for (_, anchor) in &anchors {
        add(anchor);
    }
    for row in
        list_active_staff_certificates(hasura_transaction, tenant_id, election_event_id).await?
    {
        match X509::from_pem(row.pem.as_bytes()) {
            Ok(certificate) => add(&certificate),
            Err(err) => warn!(id = %row.id, error = %err, "unreadable staff certificate"),
        }
    }
    for row in &stored {
        if let Some(index) = anchors
            .iter()
            .position(|anchor| anchor.0.id == row.issuer_id)
        {
            targets.entry(row.url.clone()).or_insert(index);
        }
    }
    Ok(targets
        .into_iter()
        .map(|(url, index)| target(url, &anchors[index], &stored))
        .collect())
}

/// Whether the stored list of a point is good and current at `now`.
fn is_fresh(stored: Option<&StaffCrlRow>, now: DateTime<Utc>) -> bool {
    stored
        .and_then(|row| row.der.as_deref())
        .and_then(|der| list_scope(der).ok())
        .is_some_and(|scope| scope.is_current(now))
}

/// Whether a failed download of the point is too recent to try again.
fn backing_off(stored: Option<&StaffCrlRow>, now: DateTime<Utc>) -> bool {
    stored.is_some_and(|row| {
        row.status == CrlStatus::Unavailable
            && row.fetched_at > now - Span::minutes(CRL_RETRY_AFTER_MINUTES)
    })
}

/// The missing or stale lists of the chain's verified path, for the
/// certificates a staff issuer issued, except the ones that failed within
/// [`CRL_RETRY_AFTER_MINUTES`]. An untrusted chain needs none.
#[instrument(skip(hasura_transaction, chain_pem), err)]
pub async fn plan_chain_crls(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    chain_pem: &[String],
    now: DateTime<Utc>,
) -> Result<Vec<CrlTarget>> {
    let Ok(chain) = parse_chain(chain_pem) else {
        return Ok(vec![]);
    };
    let anchors = load_staff_anchors(hasura_transaction, tenant_id, election_event_id).await?;
    let certificates: Vec<X509> = anchors.iter().map(|(_, anchor)| anchor.clone()).collect();
    let Ok(path) = verified_path(&chain[0], &chain[1..], &certificates)
        .context("Error verifying the chain")?
    else {
        return Ok(vec![]);
    };
    let stored = list_staff_crls(hasura_transaction, tenant_id, election_event_id).await?;
    let mut targets = vec![];
    for certificate in &path[..path.len() - 1] {
        let Some(issuer) = issuer_of(certificate, &anchors) else {
            continue;
        };
        for url in distribution_points(certificate) {
            let target = target(url, issuer, &stored);
            if !is_fresh(target.stored.as_ref(), now) && !backing_off(target.stored.as_ref(), now) {
                targets.push(target);
            }
        }
    }
    Ok(targets)
}

/// The lists this process is downloading now.
fn in_flight() -> &'static Mutex<HashSet<String>> {
    static IN_FLIGHT: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    IN_FLIGHT.get_or_init(Default::default)
}

/// Marks a list as being downloaded until dropped; `None` when it already
/// is.
struct InFlight(String);

impl InFlight {
    fn claim(target: &CrlTarget) -> Option<Self> {
        let key = format!(
            "{}:{}:{}",
            target.issuer.tenant_id, target.issuer.election_event_id, target.url
        );
        let mut set = in_flight()
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        set.insert(key.clone()).then_some(InFlight(key))
    }
}

impl Drop for InFlight {
    fn drop(&mut self) {
        in_flight()
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .remove(&self.0);
    }
}

/// What a refresh did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CrlRefresh {
    pub ok: Vec<String>,
    pub unavailable: Vec<String>,
    /// Lists another task of this process was downloading.
    pub skipped: Vec<String>,
}

/// Downloads `targets`, outside any transaction, and stores each outcome in
/// a short transaction of its own. A list another task of this process is
/// downloading is skipped.
pub async fn refresh_targets(
    client: &mut Client,
    targets: Vec<CrlTarget>,
    fetcher: &dyn CrlFetcher,
    now: DateTime<Utc>,
) -> Result<CrlRefresh> {
    let mut refresh = CrlRefresh::default();
    for target in targets {
        let Some(_claim) = InFlight::claim(&target) else {
            refresh.skipped.push(target.url);
            continue;
        };
        let download = download(&target, fetcher, now).await;
        let transaction = client.transaction().await?;
        let row = store_download(&transaction, &target, &download).await?;
        transaction.commit().await?;
        match download {
            CrlDownload::Ok { .. } => refresh.ok.push(row.url),
            CrlDownload::Unavailable { .. } => refresh.unavailable.push(row.url),
        }
    }
    Ok(refresh)
}

/// Refreshes every list of the event.
#[instrument(skip(client, fetcher), err)]
pub async fn refresh_event_crls(
    client: &mut Client,
    tenant_id: Uuid,
    election_event_id: Uuid,
    fetcher: &dyn CrlFetcher,
    now: DateTime<Utc>,
) -> Result<CrlRefresh> {
    let targets = {
        let transaction = client.transaction().await?;
        plan_event_crls(&transaction, tenant_id, election_event_id).await?
    };
    let refresh = refresh_targets(client, targets, fetcher, now).await?;
    info!(
        ok = refresh.ok.len(),
        unavailable = refresh.unavailable.len(),
        "staff CRLs refreshed"
    );
    Ok(refresh)
}

/// Downloads the missing or stale lists of the chain's verified path.
#[instrument(skip(client, chain_pem, fetcher), err)]
pub async fn refresh_chain_crls(
    client: &mut Client,
    tenant_id: Uuid,
    election_event_id: Uuid,
    chain_pem: &[String],
    fetcher: &dyn CrlFetcher,
    now: DateTime<Utc>,
) -> Result<CrlRefresh> {
    let targets = {
        let transaction = client.transaction().await?;
        plan_chain_crls(&transaction, tenant_id, election_event_id, chain_pem, now).await?
    };
    refresh_targets(client, targets, fetcher, now).await
}
