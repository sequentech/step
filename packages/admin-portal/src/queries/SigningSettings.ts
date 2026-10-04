// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Election Event > Signatures. Reads select the signing tables, whose only
// select permission is the sub-tab's read permission, so each read sends that
// permission as its role. Writes are Hasura actions forwarding to the Harvest
// routes of the signing settings, which check the write permissions.
import {gql} from "@apollo/client"

export const GET_SIGNING_RULES = gql`
    query GetSigningRules($electionEventId: uuid!) {
        sequent_backend_signing_rule(where: {election_event_id: {_eq: $electionEventId}}) {
            action
            requirement
            signatures
            requester_signing
            expires_minutes
            revision
            updated_by
            updated_by_name
            updated_at
        }
    }
`

/**
 * `signingRuleCapacity` of every action, one alias per action (the action id
 * with underscores). `posts`, `posts_short` and `posts_short_without_requester`
 * are jsonb arrays of `{election_id, name, count}`; `roles` is a jsonb array of
 * the groups `{id, name, path}`.
 */
export const GET_SIGNING_RULE_CAPACITIES = gql`
    query GetSigningRuleCapacities($electionEventId: uuid!) {
        initialize_voting: signingRuleCapacity(
            election_event_id: $electionEventId
            action: "initialize-voting"
        ) {
            max
            posts
            posts_short
            posts_short_without_requester
            roles
            waiting
            config_version
        }
        open_voting: signingRuleCapacity(
            election_event_id: $electionEventId
            action: "open-voting"
        ) {
            max
            posts
            posts_short
            posts_short_without_requester
            roles
            waiting
            config_version
        }
        close_voting: signingRuleCapacity(
            election_event_id: $electionEventId
            action: "close-voting"
        ) {
            max
            posts
            posts_short
            posts_short_without_requester
            roles
            waiting
            config_version
        }
        generate_election_returns: signingRuleCapacity(
            election_event_id: $electionEventId
            action: "generate-election-returns"
        ) {
            max
            posts
            posts_short
            posts_short_without_requester
            roles
            waiting
            config_version
        }
        generate_reports: signingRuleCapacity(
            election_event_id: $electionEventId
            action: "generate-reports"
        ) {
            max
            posts
            posts_short
            posts_short_without_requester
            roles
            waiting
            config_version
        }
        transmit_results: signingRuleCapacity(
            election_event_id: $electionEventId
            action: "transmit-results"
        ) {
            max
            posts
            posts_short
            posts_short_without_requester
            roles
            waiting
            config_version
        }
        approve_voter: signingRuleCapacity(
            election_event_id: $electionEventId
            action: "approve-voter"
        ) {
            max
            posts
            posts_short
            posts_short_without_requester
            roles
            waiting
            config_version
        }
        approve_configuration: signingRuleCapacity(
            election_event_id: $electionEventId
            action: "approve-configuration"
        ) {
            max
            posts
            posts_short
            posts_short_without_requester
            roles
            waiting
            config_version
        }
        key_ceremony: signingRuleCapacity(
            election_event_id: $electionEventId
            action: "key-ceremony"
        ) {
            max
            posts
            posts_short
            posts_short_without_requester
            roles
            waiting
            config_version
        }
        tally_key: signingRuleCapacity(election_event_id: $electionEventId, action: "tally-key") {
            max
            posts
            posts_short
            posts_short_without_requester
            roles
            waiting
            config_version
        }
    }
`

export const GET_SIGNING_CERTIFICATES = gql`
    query GetSigningCertificates($electionEventId: uuid!) {
        checks: sequent_backend_signing_checks(
            where: {election_event_id: {_eq: $electionEventId}}
        ) {
            revocation_check
            crl_unavailable
            registration
            post_binding
            revision
            updated_at
            updated_by_name
        }
        issuers: sequent_backend_certificate_authority(
            where: {election_event_id: {_eq: $electionEventId}, purpose: {_eq: "staff-signatures"}}
            order_by: {created_at: asc}
        ) {
            id
            common_name
            subject
            issuer
            issuer_common_name
            not_after
            fingerprint_sha256
        }
        certificates: sequent_backend_staff_certificate(
            where: {election_event_id: {_eq: $electionEventId}}
            order_by: {registered_at: desc}
        ) {
            id
            user_id
            username
            election_id
            fingerprint_sha256
            spki_sha256
            holder_sha256
            serial
            subject
            issuer
            not_before
            not_after
            status
            registration
            registered_by
            registered_at
            revoked_by
            revoked_at
            revoke_reason
            user_display_name
            registered_by_name
            revoked_by_name
            linked_to
        }
        crls: sequent_backend_staff_crl(where: {election_event_id: {_eq: $electionEventId}}) {
            id
            issuer_fingerprint
            url
            fetched_at
            status
        }
    }
`

export const GET_SIGNING_REQUESTS = gql`
    query GetSigningRequests($electionEventId: uuid!) {
        sequent_backend_signing_request(
            where: {election_event_id: {_eq: $electionEventId}}
            order_by: {created_at: desc}
        ) {
            id
            action
            election_id
            area_id
            code
            required
            status
            cancel_reason
            requested_by_username
            requested_by_name
            created_at
            expires_at
            approvals(order_by: {signed_at: asc}) {
                id
                username
                display_name
                signed_at
            }
        }
    }
`

/**
 * The waiting requests of one action a signer may sign, in their Posts:
 * queried as that action's `sign-<action>` role, whose select permission
 * keeps to its action and the user's Post labels.
 */
export const GET_WAITING_SIGNING_REQUESTS = gql`
    query GetWaitingSigningRequests($electionEventId: uuid!) {
        sequent_backend_signing_request(
            where: {election_event_id: {_eq: $electionEventId}, status: {_eq: "waiting"}}
            order_by: {created_at: asc}
        ) {
            id
            action
            election_id
            area_id
            code
            required
            created_at
            expires_at
            approvals(order_by: {signed_at: asc}) {
                id
                user_id
                signed_at
            }
        }
    }
`

/**
 * `POST /signing-event-info`: the event's time zone (IANA) and, for a
 * reader of the certificates, the signers' titles by user id.
 */
export const SIGNING_EVENT_INFO = gql`
    query SigningEventInfo($electionEventId: uuid!) {
        signingEventInfo(election_event_id: $electionEventId) {
            time_zone
            titles
        }
    }
`

/** `PUT /signing-rules/<action>`; `roles` (group ids) needs role-read and role-write. */
export const SIGNING_PUT_RULE = gql`
    mutation SigningPutRule(
        $election_event_id: uuid!
        $action: String!
        $requirement: String!
        $signatures: Int!
        $requester_signing: String!
        $expires_minutes: Int
        $roles: SigningRoleChangesInput
        $expected_revision: Int!
    ) {
        signingPutRule(
            election_event_id: $election_event_id
            action: $action
            requirement: $requirement
            signatures: $signatures
            requester_signing: $requester_signing
            expires_minutes: $expires_minutes
            roles: $roles
            expected_revision: $expected_revision
        ) {
            revision
            cancelled
            rule
            short_posts
            warnings
        }
    }
`

/** `POST /signing-issuers`: PEM text, or a .cer/.der file's bytes in base64. */
export const SIGNING_IMPORT_ISSUERS = gql`
    mutation SigningImportIssuers($election_event_id: uuid!, $pem: String, $der_base64: String) {
        signingImportIssuers(
            election_event_id: $election_event_id
            pem: $pem
            der_base64: $der_base64
        ) {
            imported
            skipped
            errors
        }
    }
`

/** `DELETE /signing-issuers/<id>`. */
export const SIGNING_DELETE_ISSUER = gql`
    mutation SigningDeleteIssuer($election_event_id: uuid!, $issuer_id: uuid!) {
        signingDeleteIssuer(election_event_id: $election_event_id, issuer_id: $issuer_id) {
            issuer_id
        }
    }
`

/** `PUT /signing-checks`. */
export const SIGNING_PUT_CHECKS = gql`
    mutation SigningPutChecks(
        $election_event_id: uuid!
        $revocation_check: String!
        $crl_unavailable: String!
        $registration: String!
        $post_binding: String!
        $expected_revision: Int!
    ) {
        signingPutChecks(
            election_event_id: $election_event_id
            revocation_check: $revocation_check
            crl_unavailable: $crl_unavailable
            registration: $registration
            post_binding: $post_binding
            expected_revision: $expected_revision
        ) {
            revision
        }
    }
`

/** `POST /staff-certificates`; `linked_to` links the same person's second account (O1). */
export const SIGNING_REGISTER_CERTIFICATE = gql`
    mutation SigningRegisterCertificate(
        $election_event_id: uuid!
        $user_id: String!
        $election_id: uuid
        $pem: String!
        $linked_to: String
    ) {
        signingRegisterCertificate(
            election_event_id: $election_event_id
            user_id: $user_id
            election_id: $election_id
            pem: $pem
            linked_to: $linked_to
        ) {
            certificate_id
        }
    }
`

/** `POST /staff-certificates/<id>/revoke`. */
export const SIGNING_REVOKE_CERTIFICATE = gql`
    mutation SigningRevokeCertificate(
        $election_event_id: uuid!
        $certificate_id: uuid!
        $reason: String!
    ) {
        signingRevokeCertificate(
            election_event_id: $election_event_id
            certificate_id: $certificate_id
            reason: $reason
        ) {
            certificate_id
        }
    }
`

/** `GET /signing-requests/export`: stores the CSV as a document to download. */
export const SIGNING_EXPORT_REQUESTS = gql`
    mutation SigningExportRequests($election_event_id: uuid!) {
        signingExportRequests(election_event_id: $election_event_id) {
            document_id
            sha256
            rows
            url
        }
    }
`
