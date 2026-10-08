// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Boundaries of the election event's Data tab: the event, its tenant and
// elections, and the Keycloak and results website services the save updates.
import {GraphQLError} from "graphql"
import {EBallotBoxSealPolicy, ElectionsOrder} from "@sequentech/ui-core"
import {EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {electionRecord, eventRecord, storyId, tenantRecord} from "@/__stories__/fixtures"
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import type {RealmPasswordPolicy} from "@/queries/RealmPasswordPolicy"
import {EStoryWorkflow} from "../../../../../ui-essentials/.storybook/globals"

export const SECOND_ELECTION_ID = storyId(4, 2)

export const REALM_ATTRIBUTES = {
    voter_certificate_policy: "disabled",
    enrollment_windows: "null",
    enrollment_registration_restore: "enabled",
}

export const PASSWORD_POLICY: RealmPasswordPolicy = {
    configured: true,
    minimum_length: 12,
    maximum_length: 72,
    include_uppercase: true,
    include_lowercase: true,
    include_digits: true,
    include_special_characters: false,
}

export interface EventDataScenario {
    /** What reading the election event does. */
    reads: ReadState
    /** The event lists its elections in the order the administrator set. */
    customOrder: boolean
    /** Whether reading the realm attributes fails. */
    realmAttributesFail: boolean
    /** Voting has never opened in the event, so the ballot box seal policy can still change. */
    neverOpened?: boolean
    /** The event seals its ballot boxes at close (VOTE-FREEZE). */
    sealAtClose?: boolean
}

export const eventDataArgs: EventDataScenario = {
    reads: "records",
    customOrder: false,
    realmAttributesFail: false,
}

export const eventDataEvent = (customOrder = false, neverOpened = false, sealAtClose = false) => {
    const workflow = neverOpened ? EStoryWorkflow.KEYS : undefined
    const event = eventRecord(workflow)
    return eventRecord(workflow, {
        presentation: {
            ...event.presentation,
            ...(customOrder ? {elections_order: ElectionsOrder.CUSTOM} : {}),
            ...(sealAtClose ? {ballot_box_seal_policy: EBallotBoxSealPolicy.SEAL_AT_CLOSE} : {}),
        },
    })
}

export function eventDataBoundaries({
    reads,
    customOrder,
    realmAttributesFail,
    neverOpened = false,
    sealAtClose = false,
}: EventDataScenario) {
    const workflow = neverOpened ? EStoryWorkflow.KEYS : undefined
    const data = resourceBoundary(
        {
            sequent_backend_election_event: [eventDataEvent(customOrder, neverOpened, sealAtClose)],
            sequent_backend_tenant: [tenantRecord],
            sequent_backend_election: [
                electionRecord(workflow, {
                    presentation: {...electionRecord().presentation, sort_order: 5},
                }),
                electionRecord(workflow, {
                    id: SECOND_ELECTION_ID,
                    presentation: {...electionRecord().presentation, sort_order: 2},
                }),
            ],
            sequent_backend_support_material: [],
        },
        {reads: {sequent_backend_election_event: reads}}
    )
    const graphql = graphqlBoundary(
        {
            GetRealmAttributes: () =>
                realmAttributesFail
                    ? {errors: [new GraphQLError("Synthetic Keycloak failure")]}
                    : {data: {get_realm_attributes: {attributes: REALM_ATTRIBUTES}}},
            UpdateRealmAttributes: () => ({data: {update_realm_attributes: {updated: true}}}),
            GetRealmPasswordPolicy: () => ({data: {get_realm_password_policy: PASSWORD_POLICY}}),
            // The published configuration the Voting lifecycle section compares with: none yet.
            GetLifecycleSnapshots: () => ({data: {get_lifecycle_snapshots: {snapshots: []}}}),
            SetCustomUrls: () => ({data: {set_custom_urls: {success: true, message: ""}}}),
            SetVoterAuthentication: () => ({
                data: {set_voter_authentication: {success: true, message: ""}},
            }),
            ConfigureResultsWebsitePolicy: ({variables}) => ({
                data: {
                    configureResultsWebsitePolicy: {
                        election_event_id: EVENT_ID,
                        status: variables.status,
                        access: variables.access,
                        visibility_scope: variables.visibility_scope,
                    },
                },
            }),
        },
        // get_lifecycle_snapshots isn't in the generated schema yet: no schema check.
        {schema: false}
    )
    return {data, graphql}
}
