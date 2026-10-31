// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

// The Transparency tab's actions (step-integration.md, section 2.8). The types
// are written by hand until the schema with these actions is merged and
// codegen runs again.

export const GET_TRANSPARENCY = gql`
    query GetTransparency($electionEventId: uuid!) {
        get_transparency(election_event_id: $electionEventId) {
            mode
            refusal
            locked
            logs
            status
            draft_policy
            policy
            policy_id
            enrolments
            explorer_url
        }
    }
`

export const ENABLE_TRANSPARENCY = gql`
    mutation EnableTransparency($electionEventId: uuid!, $mode: String!) {
        enable_transparency(election_event_id: $electionEventId, mode: $mode) {
            mode
            refusal
            locked
            logs
            status
            draft_policy
            policy
            policy_id
            enrolments
            explorer_url
        }
    }
`

export const UPLOAD_TRANSPARENCY_POLICY = gql`
    mutation UploadTransparencyPolicy($electionEventId: uuid!, $signedPolicy: String!) {
        upload_transparency_policy(
            election_event_id: $electionEventId
            signed_policy: $signedPolicy
        ) {
            mode
            refusal
            locked
            logs
            status
            draft_policy
            policy
            policy_id
            enrolments
            explorer_url
        }
    }
`

/** An event's transparency, as Harvest answers each of the three actions. */
export interface ITransparencyOutput {
    /** disabled | public | private */
    mode: string
    /** Why `public` is refused, as a code, if it is. */
    refusal?: string | null
    /** Voting opened: the mode and the policy can't change. */
    locked: boolean
    /** jsonb: the sidecar's `GET /admin/logs` entries for the event's logs. */
    logs?: unknown
    /** jsonb: the sidecar's `GET <log>/status` of each log, by log name. */
    status?: unknown
    draft_policy?: string | null
    policy?: string | null
    /** Hex, as the sidecar returned it. */
    policy_id?: string | null
    /** jsonb: [{log, note}], each log's witness enrolment note. */
    enrolments?: unknown
    explorer_url?: string | null
}

export interface GetTransparencyQueryVariables {
    electionEventId: string
}

export interface GetTransparencyQuery {
    get_transparency?: ITransparencyOutput | null
}

export interface EnableTransparencyMutationVariables {
    electionEventId: string
    mode: string
}

export interface EnableTransparencyMutation {
    enable_transparency?: ITransparencyOutput | null
}

export interface UploadTransparencyPolicyMutationVariables {
    electionEventId: string
    signedPolicy: string
}

export interface UploadTransparencyPolicyMutation {
    upload_transparency_policy?: ITransparencyOutput | null
}
