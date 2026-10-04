// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Boundaries of the election event's Data tab: the event, its tenant and
// elections, and the Keycloak and results website services the save updates.
import {GraphQLError} from "graphql"
import {ElectionsOrder} from "@sequentech/ui-core"
import {EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {electionRecord, eventRecord, storyId, tenantRecord} from "@/__stories__/fixtures"
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import type {RealmPasswordPolicy} from "@/queries/RealmPasswordPolicy"

export const SECOND_ELECTION_ID = storyId(4, 2)

export const REALM_ATTRIBUTES = {voter_certificate_policy: "disabled"}

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
}

export const eventDataArgs: EventDataScenario = {
    reads: "records",
    customOrder: false,
    realmAttributesFail: false,
}

export const eventDataEvent = (customOrder = false) => {
    const event = eventRecord()
    return customOrder
        ? eventRecord(undefined, {
              presentation: {...event.presentation, elections_order: ElectionsOrder.CUSTOM},
          })
        : event
}

export function eventDataBoundaries({reads, customOrder, realmAttributesFail}: EventDataScenario) {
    const data = resourceBoundary(
        {
            sequent_backend_election_event: [eventDataEvent(customOrder)],
            sequent_backend_tenant: [tenantRecord],
            sequent_backend_election: [
                electionRecord(undefined, {
                    presentation: {...electionRecord().presentation, sort_order: 5},
                }),
                electionRecord(undefined, {
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
