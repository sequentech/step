// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Wire contract for the DB-free direct-approval signer. The authoritative
//! Harvest callers build these from a checked, immutable request, never from
//! browser-supplied action, actor, scope, key or artifact values.

use sequent_core::signing::{DocumentKind, SigningAction};
use serde::Serialize;
use uuid::Uuid;

const EVIDENCE_DOMAIN: &[u8] = b"sequent.local.operational.direct/v2\0";
const AUTHORIZATION_DOMAIN: &[u8] = b"sequent.local.operational.authorization/v2\0";
type Digest = [u8; 32];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DirectAnchor {
    pub installation: Uuid,
    pub tenant: Uuid,
    pub event: Uuid,
    pub post: Uuid,
    pub publication: Uuid,
}

/// The trusted transaction chooses how the header publication was bound.
/// The discriminator is not supplied by the browser and is not a wire field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PublicationBinding {
    /// The original installation anchors its first publication, which stays V1-only.
    InstalledAnchor,
    /// A fresh request froze the exact publication that it asks to initialize.
    FrozenRequest,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct DirectRequestScope {
    pub election: Option<Uuid>,
    pub area: Option<Uuid>,
    pub trustee: Option<Uuid>,
    pub publication: Option<Uuid>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DirectPurpose {
    ApprovalPayload,
    Pdf,
    Eml,
}

impl DirectPurpose {
    pub fn for_document(action: SigningAction) -> Option<Self> {
        match action.document() {
            DocumentKind::NoDocument => None,
            DocumentKind::Pdf => Some(Self::Pdf),
            DocumentKind::Eml => Some(Self::Eml),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::ApprovalPayload => "approval-payload",
            Self::Pdf => "pdf",
            Self::Eml => "eml",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirectOperation {
    anchor: DirectAnchor,
    request: Uuid,
    actor: Uuid,
    action: SigningAction,
    scope: DirectRequestScope,
    payload_digest: Digest,
    document_digest: Option<Digest>,
}

impl DirectOperation {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        anchor: DirectAnchor,
        publication_binding: PublicationBinding,
        request: Uuid,
        actor: Uuid,
        action: SigningAction,
        scope: DirectRequestScope,
        payload_sha256: &str,
        document_sha256: Option<&str>,
    ) -> Option<Self> {
        if [
            anchor.installation,
            anchor.tenant,
            anchor.event,
            anchor.post,
            anchor.publication,
            request,
            actor,
        ]
        .iter()
        .any(Uuid::is_nil)
            || [scope.election, scope.area, scope.trustee, scope.publication]
                .into_iter()
                .flatten()
                .any(|id| id.is_nil())
            || !scope_matches(anchor, publication_binding, action, scope)
        {
            return None;
        }
        let document_digest = match (DirectPurpose::for_document(action), document_sha256) {
            (Some(_), Some(value)) => Some(digest(value)?),
            (None, None) => None,
            _ => return None,
        };
        Some(Self {
            anchor,
            request,
            actor,
            action,
            scope,
            payload_digest: digest(payload_sha256)?,
            document_digest,
        })
    }

    pub fn anchor(&self) -> DirectAnchor {
        self.anchor
    }

    pub fn request(&self) -> Uuid {
        self.request
    }

    pub fn actor(&self) -> Uuid {
        self.actor
    }

    pub fn action(&self) -> SigningAction {
        self.action
    }

    pub fn scope(&self) -> DirectRequestScope {
        self.scope
    }

    pub fn payload_digest(&self) -> Digest {
        self.payload_digest
    }

    pub fn artifact_digest(&self, purpose: DirectPurpose) -> Option<Digest> {
        if purpose == DirectPurpose::ApprovalPayload {
            return Some(self.payload_digest);
        }
        (DirectPurpose::for_document(self.action) == Some(purpose))
            .then_some(self.document_digest)
            .flatten()
    }

    /// The HSM signs a SHA-256 prehash of these exact bytes. An artifact leg
    /// is a detached proof of review, not an X.509/PAdES/EML certification.
    pub fn evidence(&self, nonce: Uuid, purpose: DirectPurpose) -> Option<Vec<u8>> {
        if nonce.is_nil() {
            return None;
        }
        let artifact = self.artifact_digest(purpose)?;
        let action = self.action.to_string();
        let purpose = purpose.as_str();
        let mut bytes = Vec::with_capacity(300);
        bytes.extend_from_slice(EVIDENCE_DOMAIN);
        for id in [
            self.anchor.installation,
            self.anchor.tenant,
            self.anchor.event,
            self.anchor.post,
            self.anchor.publication,
            self.request,
            self.actor,
            nonce,
        ] {
            bytes.extend_from_slice(id.as_bytes());
        }
        for token in [action.as_bytes(), purpose.as_bytes()] {
            bytes.push(u8::try_from(token.len()).ok()?);
            bytes.extend_from_slice(token);
        }
        for id in [
            self.scope.election,
            self.scope.area,
            self.scope.trustee,
            self.scope.publication,
        ] {
            if let Some(id) = id {
                bytes.push(1);
                bytes.extend_from_slice(id.as_bytes());
            } else {
                bytes.push(0);
            }
        }
        bytes.extend_from_slice(&self.payload_digest);
        bytes.extend_from_slice(&artifact);
        Some(bytes)
    }

    /// Ed25519 authorization from the installed Harvest peer. A different
    /// reservation/fence or deadline does not authorize the same evidence.
    pub fn authorization(
        &self,
        reservation: Uuid,
        nonce: Uuid,
        expires_at_unix: i64,
        purpose: DirectPurpose,
    ) -> Option<Vec<u8>> {
        if reservation.is_nil() || expires_at_unix <= 0 {
            return None;
        }
        let evidence = self.evidence(nonce, purpose)?;
        let mut bytes = Vec::with_capacity(AUTHORIZATION_DOMAIN.len() + 24 + evidence.len());
        bytes.extend_from_slice(AUTHORIZATION_DOMAIN);
        bytes.extend_from_slice(reservation.as_bytes());
        bytes.extend_from_slice(&expires_at_unix.to_be_bytes());
        bytes.extend_from_slice(&evidence);
        Some(bytes)
    }
}

fn digest(text: &str) -> Option<Digest> {
    let mut bytes = [0; 32];
    hex::decode_to_slice(text, &mut bytes).ok()?;
    (hex::encode(bytes) == text).then_some(bytes)
}

fn scope_matches(
    anchor: DirectAnchor,
    publication_binding: PublicationBinding,
    action: SigningAction,
    scope: DirectRequestScope,
) -> bool {
    use SigningAction::*;
    let post = scope.election == Some(anchor.post);
    let no_area = scope.area.is_none();
    let no_trustee = scope.trustee.is_none();
    let anchor_publication = scope
        .publication
        .is_none_or(|publication| publication == anchor.publication);
    match action {
        InitializeVoting => {
            post && no_area
                && no_trustee
                && scope
                    .publication
                    .is_some_and(|published| match publication_binding {
                        PublicationBinding::InstalledAnchor => published != anchor.publication,
                        PublicationBinding::FrozenRequest => published == anchor.publication,
                    })
        }
        OpenVoting | CloseVoting | GenerateReports => {
            post && no_area && no_trustee && anchor_publication
        }
        AuthorizeManualVerification => post && no_area && no_trustee && scope.publication.is_none(),
        // The application approval route always resolves its Post *from* its area.
        // Harvest re-reads that exact application before authorizing this transcript.
        ApproveVoter => post && !no_area && no_trustee && scope.publication.is_none(),
        GenerateElectionReturns | TransmitResults => post && no_trustee && anchor_publication,
        // The request can authorize a new ballot publication. Its target is
        // checked by Harvest; it is not the installation's trust anchor.
        ApproveConfiguration => {
            (scope.election.is_none() || post)
                && no_area
                && no_trustee
                && scope.publication.is_some()
        }
        ConfirmCustodyProof | ContributeCustodyProof => {
            scope.election.is_none() && no_area && scope.trustee.is_some() && anchor_publication
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use SigningAction::*;

    fn ids(id: u128) -> Uuid {
        Uuid::from_u128(id)
    }

    fn anchor() -> DirectAnchor {
        DirectAnchor {
            installation: ids(1),
            tenant: ids(2),
            event: ids(3),
            post: ids(4),
            publication: ids(5),
        }
    }

    fn operation(action: SigningAction, scope: DirectRequestScope) -> Option<DirectOperation> {
        let document = "ab".repeat(32);
        DirectOperation::new(
            anchor(),
            PublicationBinding::InstalledAnchor,
            ids(6),
            ids(7),
            action,
            scope,
            &"cd".repeat(32),
            DirectPurpose::for_document(action).map(|_| document.as_str()),
        )
    }

    #[test]
    fn only_the_catalog_scopes_and_exact_document_legs_are_admitted() {
        let post = DirectRequestScope {
            election: Some(ids(4)),
            ..Default::default()
        };
        let initialization = DirectRequestScope {
            publication: Some(ids(50)),
            ..post
        };
        let configuration = DirectRequestScope {
            publication: Some(ids(17)),
            ..Default::default()
        };
        let trustee = DirectRequestScope {
            trustee: Some(ids(18)),
            ..Default::default()
        };
        let country = DirectRequestScope {
            area: Some(ids(19)),
            ..post
        };
        for (action, scope) in [
            (InitializeVoting, initialization),
            (OpenVoting, post),
            (CloseVoting, post),
            (GenerateElectionReturns, country),
            (GenerateReports, post),
            (TransmitResults, country),
            (ApproveVoter, country),
            (AuthorizeManualVerification, post),
            (ApproveConfiguration, configuration),
            (ConfirmCustodyProof, trustee),
            (ContributeCustodyProof, trustee),
        ] {
            let operation = operation(action, scope).expect("catalog shape");
            assert!(operation
                .evidence(ids(8), DirectPurpose::ApprovalPayload)
                .is_some());
            for purpose in [DirectPurpose::Pdf, DirectPurpose::Eml] {
                assert_eq!(
                    operation.evidence(ids(8), purpose).is_some(),
                    DirectPurpose::for_document(action) == Some(purpose)
                );
            }
            assert!(operation
                .evidence(Uuid::nil(), DirectPurpose::ApprovalPayload)
                .is_none());
        }
        assert!(operation(InitializeVoting, post).is_none());
        assert!(operation(
            InitializeVoting,
            DirectRequestScope {
                publication: Some(anchor().publication),
                ..post
            }
        )
        .is_none());
        assert!(operation(OpenVoting, country).is_none());
        assert!(operation(ApproveVoter, post).is_none());
        assert!(operation(ApproveVoter, trustee).is_none());
        assert!(operation(AuthorizeManualVerification, country).is_none());
        assert!(operation(
            AuthorizeManualVerification,
            DirectRequestScope {
                publication: Some(anchor().publication),
                ..post
            }
        )
        .is_none());
        assert!(operation(AuthorizeManualVerification, trustee).is_none());
        assert!(operation(ApproveConfiguration, post).is_none());
        assert!(operation(ContributeCustodyProof, post).is_none());
        assert!(operation(
            GenerateReports,
            DirectRequestScope {
                election: Some(ids(22)),
                ..post
            }
        )
        .is_none());
        assert!(operation(
            GenerateReports,
            DirectRequestScope {
                publication: Some(ids(22)),
                ..post
            }
        )
        .is_none());
    }

    #[test]
    fn initialization_publication_shape_depends_on_trusted_request_origin() {
        let anchor = anchor();
        let input = |binding, publication| {
            DirectOperation::new(
                anchor,
                binding,
                ids(6),
                ids(7),
                InitializeVoting,
                DirectRequestScope {
                    election: Some(anchor.post),
                    publication,
                    ..Default::default()
                },
                &"cd".repeat(32),
                None,
            )
        };
        assert!(input(PublicationBinding::InstalledAnchor, Some(ids(50))).is_some());
        assert!(input(
            PublicationBinding::InstalledAnchor,
            Some(anchor.publication)
        )
        .is_none());
        assert!(input(PublicationBinding::FrozenRequest, Some(ids(50))).is_none());
        assert!(input(PublicationBinding::FrozenRequest, None).is_none());
        let exact = input(PublicationBinding::FrozenRequest, Some(anchor.publication)).unwrap();
        assert!(exact
            .evidence(ids(8), DirectPurpose::ApprovalPayload)
            .is_some());
    }

    #[test]
    fn precise_domains_and_fields_cannot_replay_between_actions_or_purposes() {
        let post = DirectRequestScope {
            election: Some(ids(4)),
            ..Default::default()
        };
        let operation = operation(GenerateReports, post).unwrap();
        let payload = operation
            .evidence(ids(8), DirectPurpose::ApprovalPayload)
            .unwrap();
        let pdf = operation.evidence(ids(8), DirectPurpose::Pdf).unwrap();
        let mut want = EVIDENCE_DOMAIN.to_vec();
        for n in 1..=8 {
            want.extend_from_slice(ids(n).as_bytes());
        }
        want.extend_from_slice(b"\x10generate-reports\x03pdf\x01");
        want.extend_from_slice(ids(4).as_bytes());
        want.extend_from_slice(&[0, 0, 0]);
        want.extend_from_slice(&[0xcd; 32]);
        want.extend_from_slice(&[0xab; 32]);
        assert_eq!(want, pdf);
        assert_ne!(payload, pdf);
        assert_ne!(pdf, operation.evidence(ids(9), DirectPurpose::Pdf).unwrap());
        let mut authorized = AUTHORIZATION_DOMAIN.to_vec();
        authorized.extend_from_slice(ids(9).as_bytes());
        authorized.extend_from_slice(&10_i64.to_be_bytes());
        authorized.extend_from_slice(&pdf);
        assert_eq!(
            operation
                .authorization(ids(9), ids(8), 10, DirectPurpose::Pdf)
                .unwrap(),
            authorized
        );
        assert_ne!(
            operation
                .authorization(ids(10), ids(8), 10, DirectPurpose::Pdf)
                .unwrap(),
            authorized
        );
        assert_ne!(
            operation
                .authorization(ids(9), ids(8), 11, DirectPurpose::Pdf)
                .unwrap(),
            authorized
        );
    }
}
