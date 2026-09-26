// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {FIXED_TIME, IDS} from "@sequentech/ui-test-kit/fixtures"
import {test, expect, TENANT_ID} from "../fixtures"
import {
    ALICE_ID,
    AREA_ID,
    BOB_ID,
    DOCUMENT_ID,
    VOTER_TAB_ROLES,
    attribute,
    electionEvent,
    mockVoters,
    openVoters,
    user,
} from "./data"

const VIEWER_ROLES = [
    ...VOTER_TAB_ROLES,
    "ee-voters-filters",
    "ee-voters-columns",
    "election-event-voter-list-reconciliation",
]

const datafixEvent = {
    annotations: {"datafix:id": "datafix-county-01"},
    presentation: {
        ...electionEvent().presentation,
        materials: {policy: "mandatory_for_voting"},
        custom_filters: [
            {label: {name: "notVoted", i18n: {en: "Not voted yet"}}, filter: {has_voted: false}},
        ],
    },
}

const ATTRIBUTES = [
    attribute("username", {display_name: "Username"}),
    attribute("birthdate", {display_name: "Birth date", annotations: {inputType: "html5-date"}}),
    attribute("channels", {display_name: "Channels", multivalued: true}),
    attribute("voted-channel", {display_name: "Voted channel"}),
    attribute("disable-comment", {display_name: "Disable comment"}),
    attribute("support-materials-acknowledged", {multivalued: true}),
]

const voters = [
    user(ALICE_ID, "alice", {
        area: {id: AREA_ID, name: "North"},
        votes_info: [{election_id: IDS.election, num_votes: 1, last_voted_at: FIXED_TIME}],
        attributes: {
            "birthdate": ["1990-05-01"],
            "channels": ["email", "sms"],
            "voted-channel": ["NONE", "PAPER"],
            "disable-comment": ["Voted on paper"],
            "support-materials-acknowledged": [DOCUMENT_ID],
        },
    }),
    user(BOB_ID, "bob", {attributes: {"voted-channel": ["NONE"]}}),
]

test.describe("voter list reader of a Datafix event", () => {
    test.use({roles: VIEWER_ROLES})

    test("shows voting, sync and materials columns and applies a custom filter", async ({
        page,
        portal,
    }) => {
        mockVoters(portal, {event: datafixEvent, attributes: ATTRIBUTES, users: voters})
        portal.graphql.on("sequent_backend_support_material", () => ({
            data: {
                sequent_backend_support_material: [
                    {
                        id: "16161616-1616-4616-8616-161616161601",
                        tenant_id: TENANT_ID,
                        election_event_id: IDS.event,
                        document_id: DOCUMENT_ID,
                        kind: "application/pdf",
                        data: {},
                        labels: {},
                        annotations: {},
                        is_hidden: false,
                        created_at: FIXED_TIME,
                        last_updated_at: FIXED_TIME,
                    },
                ],
                sequent_backend_support_material_aggregate: {aggregate: {count: 1}},
            },
        }))
        await openVoters(page, portal)
        await page.getByRole("button", {name: "Columns", exact: true}).click()
        for (const name of ["Birth date", "Channels", "Voted Channel", "Disable Comment"])
            await page.getByRole("switch", {name, exact: true}).check()
        await page.keyboard.press("Escape")
        for (const name of [
            "Voted Channel",
            "Disable Comment",
            "Voted",
            "Support Materials Viewed",
        ])
            await expect(
                page.getByRole("columnheader").filter({has: page.getByText(name, {exact: true})})
            ).toBeVisible()
        const alice = page.getByRole("row", {name: /alice/})
        await expect(alice.getByRole("cell", {name: "PAPER", exact: true})).toBeVisible()
        await expect(alice.getByRole("cell", {name: "Voted on paper", exact: true})).toBeVisible()
        await expect(alice.getByText("North", {exact: true})).toBeVisible()
        await expect(
            page.getByRole("row", {name: /bob/}).getByRole("cell", {name: "-", exact: true})
        ).toHaveCount(5)

        await expect(alice.getByText("May 1, 1990", {exact: true})).toBeVisible()
        await expect(alice.getByText("sms", {exact: true})).toBeVisible()

        const before = portal.graphql.callsTo("getUsers").length
        await page.getByRole("button", {name: "Custom Filters", exact: true}).click()
        await page.getByRole("menuitem", {name: "Not voted yet"}).click()
        await expect
            .poll(() => portal.graphql.callsTo("getUsers").slice(before).at(-1)?.variables)
            .toMatchObject({tenant_id: TENANT_ID, election_event_id: IDS.event, has_voted: false})
        expect(
            portal.graphql.callsTo("sequent_backend_support_material")[0].variables
        ).toMatchObject({limit: 9999})
    })
})
