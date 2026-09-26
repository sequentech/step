// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {FIXED_TIME, IDS} from "@sequentech/ui-test-kit/fixtures"
import {test, expect, TENANT_ID} from "../fixtures"
import {
    ALICE_ID,
    AREA_ID,
    VOTER_TAB_ROLES,
    attribute,
    expectRole,
    mockVoters,
    openVoters,
    rowAction,
    user,
} from "./data"

const ATTRIBUTES = [
    attribute("username", {display_name: "Username"}),
    attribute("email", {display_name: "Email"}),
    attribute("nickname", {display_name: "Nickname", validations: {length: {min: 3, max: 10}}}),
    attribute("country", {
        display_name: "Country",
        annotations: {inputType: "select"},
        validations: {options: {options: ["PT", "ES", "FR"]}},
    }),
    attribute("channels", {
        display_name: "Channels",
        multivalued: true,
        annotations: {
            inputType: "multiselect-checkboxes",
            inputOptionLabels: {email: "By email", sms: "By SMS"},
        },
    }),
    attribute("reference", {display_name: "Reference", annotations: {"sequent.secret": "true"}}),
]

const alice = user(ALICE_ID, "alice", {
    area: {id: AREA_ID, name: "North"},
    attributes: {
        "area-id": [AREA_ID],
        "nickname": ["ally"],
        "country": ["ES"],
        "channels": ["email"],
        "reference": ["stored"],
    },
})

const castVote = {
    id: "15151515-1515-4515-8515-151515151501",
    tenant_id: TENANT_ID,
    election_event_id: IDS.event,
    election_id: IDS.election,
    area_id: AREA_ID,
    voter_id_string: ALICE_ID,
    status: "CAST",
    created_at: FIXED_TIME,
    last_updated_at: FIXED_TIME,
}

test.describe("voter editor with secret field access", () => {
    test.use({
        roles: [
            ...VOTER_TAB_ROLES,
            "voter-write",
            "voter-secret-attribute-read",
            "voter-secret-attribute-write",
        ],
    })

    test("validates, reveals and submits a reviewed voter profile edit", async ({page, portal}) => {
        mockVoters(portal, {attributes: ATTRIBUTES, users: [alice]})
        portal.graphql.on("RevealVoterSecretAttribute", () => ({
            data: {
                reveal_voter_secret_attribute: {attribute_name: "reference", values: ["REF-123"]},
            },
        }))
        portal.graphql.on("EditUser", () => ({
            data: {edit_user: {user: {id: ALICE_ID}, task_execution: null}},
        }))
        await openVoters(page, portal)
        await rowAction(page, "Edit")
        const drawer = page.getByRole("dialog")
        const nickname = drawer.getByRole("textbox", {name: "Nickname"})
        await expect(nickname).toHaveValue("ally")
        await expect(drawer.getByText("Between 3 and 10 characters", {exact: true})).toBeVisible()
        await nickname.fill("al")
        await nickname.blur()
        await expect(
            drawer.getByText('"Nickname" must be between 3 and 10 characters', {exact: true})
        ).toBeVisible()
        await expect(drawer.getByRole("button", {name: "Save", exact: true})).toBeDisabled()
        await nickname.fill("allie")
        await nickname.blur()
        await expect(drawer.getByRole("button", {name: "Save", exact: true})).toBeEnabled()

        await drawer.getByRole("combobox", {name: "Country"}).click()
        await page.getByRole("option", {name: "PT", exact: true}).click()
        await drawer.getByRole("checkbox", {name: "By SMS"}).check()

        expect(portal.graphql.callsTo("RevealVoterSecretAttribute")).toHaveLength(0)
        await drawer.getByRole("button", {name: "Reveal", exact: true}).click()
        await expect(drawer.getByRole("textbox", {name: "Reference"})).toHaveValue("REF-123")

        await drawer.getByRole("button", {name: "Save", exact: true}).click()
        const confirm = drawer.getByRole("button", {name: "Confirm changes", exact: true})
        await expect(confirm).toBeVisible()
        for (const value of ["ally", "allie", "ES", "PT"])
            await expect(drawer.getByText(value, {exact: true})).toBeVisible()
        await confirm.click()
        await expect(drawer).toHaveCount(0)
        await expect(page.getByText("Voter edited", {exact: true})).toBeVisible()
        expect(
            portal.graphql.callsTo("RevealVoterSecretAttribute").map((call) => call.variables)
        ).toEqual([
            {
                tenantId: TENANT_ID,
                electionEventId: IDS.event,
                userId: ALICE_ID,
                attributeName: "reference",
            },
        ])
        expectRole(portal, "RevealVoterSecretAttribute", "voter-read")
        expect(portal.graphql.callsTo("EditUser").map((call) => call.variables)).toEqual([
            {
                body: {
                    user_id: ALICE_ID,
                    tenant_id: TENANT_ID,
                    election_event_id: IDS.event,
                    first_name: null,
                    last_name: null,
                    enabled: true,
                    email: "alice@example.test",
                    temporary: true,
                    attributes: {
                        "area-id": [AREA_ID],
                        "nickname": ["allie"],
                        "country": ["PT"],
                        "channels": ["email", "sms"],
                    },
                    secret_attributes: {},
                },
            },
        ])
    })

    test("keeps the review open with the reason when the edit is refused", async ({
        page,
        portal,
    }) => {
        mockVoters(portal, {attributes: ATTRIBUTES, users: [alice]})
        portal.graphql.on("EditUser", () => ({
            errors: [
                {
                    message: "Can't edit a voter that has already cast its ballot",
                    extensions: {code: "Unauthorized"},
                },
            ],
        }))
        await openVoters(page, portal)
        await rowAction(page, "Edit")
        const drawer = page.getByRole("dialog")
        await drawer.getByRole("textbox", {name: "Email", exact: true}).fill("new@example.test")
        await drawer.getByRole("button", {name: "Save", exact: true}).click()
        await drawer.getByRole("button", {name: "Confirm changes", exact: true}).click()
        await expect(drawer.getByRole("alert")).toHaveText(
            "Error editing voter: Can't edit a voter that has already cast its ballot"
        )
        await expect(
            drawer.getByRole("button", {name: "Confirm changes", exact: true})
        ).toBeEnabled()
        await drawer.getByRole("button", {name: "Edit", exact: true}).click()
        await expect(drawer.getByRole("textbox", {name: "Email", exact: true})).toHaveValue(
            "new@example.test"
        )
    })
})

test.describe("voter editor without the voted-voter permission", () => {
    test.use({roles: [...VOTER_TAB_ROLES, "voter-write"]})

    test("keeps a voter who has already voted read-only", async ({page, portal}) => {
        mockVoters(portal, {attributes: ATTRIBUTES, users: [alice], castVotes: [castVote]})
        await openVoters(page, portal)
        await rowAction(page, "Edit")
        const drawer = page.getByRole("dialog")
        await expect(drawer.getByRole("textbox", {name: "Nickname"})).toHaveValue("ally")
        await expect
            .poll(() => portal.graphql.callsTo("sequent_backend_cast_vote").length)
            .toBeGreaterThan(0)
        test.fail(
            true,
            "EditUserForm enables fields through canEditVoters before the has-voted check; Harvest refuses the edit"
        )
        await expect(drawer.getByRole("textbox", {name: "Email", exact: true})).toBeDisabled({
            timeout: 2000,
        })
    })
})

test.describe("voter contact editor", () => {
    test.use({roles: [...VOTER_TAB_ROLES, "voter-email-tlf-edit"]})

    test("may change only the email of a voter who has not voted", async ({page, portal}) => {
        mockVoters(portal, {attributes: ATTRIBUTES, users: [alice]})
        await openVoters(page, portal)
        await expect(page.getByRole("cell", {name: "alice", exact: true})).toBeVisible()
        test.fail(
            true,
            "ListUsers hides the Actions column unless voter-write or another action permission is held, so voter-email-tlf-edit never reaches Edit"
        )
        expect(await page.getByRole("button", {name: "Actions", exact: true}).count()).toBe(1)
        await rowAction(page, "Edit")
        const drawer = page.getByRole("dialog")
        await expect(drawer.getByRole("textbox", {name: "Email", exact: true})).toBeEnabled()
        await expect(drawer.getByRole("textbox", {name: "Nickname"})).toBeDisabled()
        await expect(drawer.getByRole("combobox", {name: "Country"})).toBeDisabled()
        await expect(drawer.getByRole("checkbox", {name: "By SMS"})).toBeDisabled()
        await expect(drawer.getByRole("button", {name: "Reveal", exact: true})).toHaveCount(0)
    })
})
