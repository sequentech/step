// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The normal application's operator-installed, single-publication SoftHSM scope.

use super::InstalledDevice;
use crate::postgres::signing::SigningRequestRow;
use crate::services::signing::actions::subject_of;
use crate::services::signing::operational_policy::{read_tenant_mode, OperationalSigningMode};
use anyhow::{ensure, Context, Result};
use deadpool_postgres::{GenericClient, Transaction};
use sequent_core::signing::{InitializeVotingSubject, SigningAction};
use serde::{Deserialize, Deserializer};
use std::sync::OnceLock;
use uuid::Uuid;

const DIRECTORY: &str = "sequent-felix3-local-initialize";
const FILENAME: &str = "installation.json";
const MAX_BYTES: usize = 4096;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalInitializeInstallation {
    pub tenant: Uuid,
    pub event: Uuid,
    pub post: Uuid,
    pub publication: Uuid,
    pub installation: Uuid,
    #[serde(deserialize_with = "software_profile")]
    profile: InstalledDevice,
}

fn software_profile<'de, D: Deserializer<'de>>(
    input: D,
) -> std::result::Result<InstalledDevice, D::Error> {
    let device = InstalledDevice::DisposableSoftHsm;
    if String::deserialize(input)? == device.as_str() {
        Ok(device)
    } else {
        Err(serde::de::Error::custom(
            "expected the softhsm-test PKCS#11 profile",
        ))
    }
}

impl LocalInitializeInstallation {
    pub fn device(&self) -> InstalledDevice {
        self.profile
    }

    fn parse(bytes: &[u8]) -> Result<Self> {
        ensure!(
            (1..=MAX_BYTES).contains(&bytes.len()),
            "invalid local Initialize descriptor size"
        );
        // Serde can also deserialize a struct from a positional sequence.
        ensure!(
            bytes.iter().find(|byte| !byte.is_ascii_whitespace()) == Some(&b'{'),
            "local Initialize descriptor must be a JSON object"
        );
        let descriptor: Self =
            serde_json::from_slice(bytes).context("invalid local Initialize descriptor")?;
        let ids = [
            descriptor.tenant,
            descriptor.event,
            descriptor.post,
            descriptor.publication,
            descriptor.installation,
        ];
        ensure!(
            !ids.iter().any(Uuid::is_nil),
            "local Initialize descriptor contains a nil UUID"
        );
        Ok(descriptor)
    }

    pub(super) fn names_event(&self, tenant: Uuid, event: Uuid, action: SigningAction) -> bool {
        self.tenant == tenant && self.event == event && action == SigningAction::InitializeVoting
    }

    fn matches_request(&self, row: &SigningRequestRow) -> Result<bool> {
        if !self.names_event(row.tenant_id, row.election_event_id, row.action) {
            return Ok(false);
        }
        let post = row
            .election_id
            .filter(|id| !id.is_nil())
            .context("Initialize request has no Post")?;
        if post != self.post {
            return Ok(false);
        }
        ensure!(
            row.subject.is_object(),
            "invalid Initialize request subject"
        );
        let subject: InitializeVotingSubject = subject_of(row)?;
        let signed_id = subject
            .publication_id
            .context("Initialize request has no signed publication")?;
        let publication =
            Uuid::parse_str(&signed_id).context("invalid signed Initialize publication")?;
        ensure!(
            !publication.is_nil() && signed_id == publication.to_string(),
            "invalid signed Initialize publication"
        );
        Ok(publication == self.publication)
    }

    fn check_id(&self, approved: Option<Uuid>) -> Result<()> {
        ensure!(
            approved == Some(self.installation),
            "the named local Initialize installation lacks matching database approval"
        );
        Ok(())
    }

    async fn verify(&self, client: &impl GenericClient) -> Result<()> {
        // Disabling or replacing approval must serialize with this lookup in
        // the database. Use the caller's connection through receipt/effect commit.
        let approved: Option<Uuid> = client
            .query_one(
                "SELECT sequent_backend.operational_signing_local_initialize_installation_id(
                     $1::uuid, $2::uuid, $3::uuid, $4::uuid, $5::text)",
                &[
                    &self.tenant,
                    &self.event,
                    &self.post,
                    &self.publication,
                    &self.profile.as_str(),
                ],
            )
            .await
            .context("cannot read the local Initialize installation approval")?
            .try_get(0)
            .context("invalid local Initialize installation approval result")?;
        self.check_id(approved)
    }
}

#[cfg(target_os = "linux")]
mod descriptor {
    use super::*;
    use rustix::fs::{fstatvfs, open, openat, Mode, OFlags, StatVfsMountFlags};
    use rustix::io::Errno;
    use std::{fs::File, io::Read, os::unix::fs::MetadataExt};

    const DIRECTORY_FLAGS: OFlags = OFlags::RDONLY
        .union(OFlags::DIRECTORY)
        .union(OFlags::NOFOLLOW)
        .union(OFlags::CLOEXEC);

    pub(super) fn read() -> Result<Option<LocalInitializeInstallation>> {
        match open("/run", DIRECTORY_FLAGS, Mode::empty()) {
            Ok(fd) => read_at(&File::from(fd), 0),
            Err(Errno::NOENT) => Ok(None),
            Err(error) => Err(error).context("open /run"),
        }
    }

    // A host-owned file on a read-only bind mount may have a different UID
    // inside the container. On a writable mount only the operator may own it.
    fn protected_owner(file: &File, uid: u32, operator: u32) -> Result<bool> {
        Ok(uid == operator || fstatvfs(file)?.f_flag.contains(StatVfsMountFlags::RDONLY))
    }

    // Each lookup is relative to the already opened directory. Check the
    // opened file itself, and open nonblocking so a FIFO cannot stall startup.
    pub(super) fn read_at(
        run: &File,
        operator: u32,
    ) -> Result<Option<LocalInitializeInstallation>> {
        let directory = match openat(run, DIRECTORY, DIRECTORY_FLAGS, Mode::empty()) {
            Ok(fd) => File::from(fd),
            Err(Errno::NOENT) => return Ok(None),
            Err(error) => {
                return Err(error).context("open local Initialize installation directory")
            }
        };
        let root = run.metadata()?;
        ensure!(
            root.mode() & 0o022 == 0 && protected_owner(run, root.uid(), operator)?,
            "unsafe /run permissions or owner"
        );
        let parent = directory.metadata()?;
        ensure!(
            parent.mode() & 0o022 == 0 && protected_owner(&directory, parent.uid(), operator)?,
            "unsafe local Initialize directory permissions or owner"
        );
        let flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC;
        let file = match openat(&directory, FILENAME, flags, Mode::empty()) {
            Ok(fd) => File::from(fd),
            Err(Errno::NOENT) => return Ok(None),
            Err(error) => return Err(error).context("open local Initialize installation.json"),
        };
        let metadata = file.metadata()?;
        ensure!(
            metadata.is_file()
                && metadata.mode() & 0o222 == 0
                && protected_owner(&file, metadata.uid(), operator)?,
            "local Initialize descriptor must be an operator-protected read-only regular file"
        );
        ensure!(
            (1..=MAX_BYTES as u64).contains(&metadata.len()),
            "invalid local Initialize descriptor size"
        );
        let mut bytes = Vec::new();
        file.take(MAX_BYTES as u64 + 1).read_to_end(&mut bytes)?;
        Ok(Some(LocalInitializeInstallation::parse(&bytes)?))
    }
}

#[cfg(not(target_os = "linux"))]
mod descriptor {
    use super::*;

    pub(super) fn read() -> Result<Option<LocalInitializeInstallation>> {
        anyhow::bail!("the local Initialize installation requires a Linux operator mount")
    }
}

/// Pin the descriptor, including genuine absence or a read error, until restart.
/// Database authorization is read again in each consequential transaction.
pub(super) fn installed() -> Result<Option<&'static LocalInitializeInstallation>> {
    static INSTALL: OnceLock<Result<Option<LocalInitializeInstallation>>> = OnceLock::new();
    INSTALL
        .get_or_init(descriptor::read)
        .as_ref()
        .map(Option::as_ref)
        .map_err(|error| anyhow::anyhow!("{error:#}"))
}

pub(crate) async fn assert_startup(client: &impl GenericClient) -> Result<()> {
    if let Some(installation) = installed()? {
        installation.verify(client).await?;
        ensure!(
            read_tenant_mode(client, installation.tenant).await?
                == OperationalSigningMode::HsmRequired,
            "the local Initialize installation requires a tenant with hsm-required signing policy"
        );
    }
    Ok(())
}

/// Both callers have reconstructed and compared the persisted canonical payload.
pub(crate) async fn for_canonical_request(
    tx: &Transaction<'_>,
    request: &SigningRequestRow,
) -> Result<Option<&'static LocalInitializeInstallation>> {
    let Some(installation) = installed()? else {
        return Ok(None);
    };
    if !installation.matches_request(request)? {
        return Ok(None);
    }
    installation.verify(tx).await?;
    Ok(Some(installation))
}

#[cfg(test)]
#[path = "local_initialize_tests.rs"]
mod tests;
