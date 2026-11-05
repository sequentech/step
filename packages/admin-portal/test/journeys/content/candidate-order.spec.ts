// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Page} from "@playwright/test"
import type {PortalServices} from "@sequentech/ui-test-kit/adapters/playwright"
import {IDS} from "@sequentech/ui-test-kit/fixtures"
import {test, expect} from "../fixtures"
import {
    BASE_ROLES,
    CONTENT_IDS,
    candidateRow,
    contestRow,
    electionRow,
    eventPage,
    expectRole,
    names,
    notification,
    table,
} from "./data"

test.use({
    roles: [...BASE_ROLES, "contest-read", "contest-write", "candidate-read", "candidate-write"],
})

const presentation = {
    i18n: {en: {name: "Mayor", description: "Choose the mayor"}},
    candidates_order: "custom",
    invalid_vote_policy: "allowed",
    enable_checkable_lists: "allow-selecting-candidates-and-lists",
    collapsible_lists: "disabled",
    candidates_icon_checkbox_policy: "square-checkbox",
    under_vote_policy: "allowed",
    blank_vote_policy: "allowed",
    over_vote_policy: "allowed",
    duplicated_rank_policy: "allowed-warn-and-dialog",
    preference_gaps_policy: "allowed-warn-and-dialog",
    pagination_policy: "",
    allow_writeins: false,
}

function ballot(portal: PortalServices) {
    const election = electionRow()
    const contest = contestRow({
        presentation,
        tally_configuration: {tie_breaking_policy: "random"},
        annotations: {"ivr:i18n": '{"en":{"prompt":"Choose the mayor"}}'},
    })
    eventPage(portal, undefined, [election])
    table(portal, "sequent_backend_election", [election])
    table(portal, "sequent_backend_contest", [contest])
    const candidates = table(portal, "sequent_backend_candidate", [
        candidateRow({presentation: {...names("Alice Adams"), sort_order: 0}}),
        candidateRow({
            id: CONTENT_IDS.secondCandidate,
            presentation: {...names("Bob Brown"), sort_order: 1},
        }),
    ])
    table(portal, "sequent_backend_document", [])
    portal.graphql.on("contest_tree", () => ({data: {sequent_backend_contest: [contest]}}))
    portal.graphql.on("candidate_tree", () => ({data: {sequent_backend_candidate: candidates}}))
}

async function reorder(page: Page, portal: PortalServices) {
    await page.goto(`${portal.origin}/sequent_backend_contest/${IDS.contest}?lang=en`)
    await page.getByRole("textbox", {name: "Description", exact: true}).fill("Ordered ballot")
    await page.getByRole("button", {name: "Ballot Design", exact: true}).click()
    const rows = page.locator('[draggable="true"]')
    await expect(rows).toHaveText(["Alice Adams", "Bob Brown"])
    const source = rows.filter({hasText: "Bob Brown"})
    const target = rows.filter({hasText: "Alice Adams"})
    const dataTransfer = await page.evaluateHandle(() => new DataTransfer())
    try {
        // Native drag events avoid pointer hit tests racing the accordion animation.
        await source.dispatchEvent("dragstart", {dataTransfer})
        await target.dispatchEvent("dragover", {dataTransfer})
        await target.dispatchEvent("drop", {dataTransfer})
        await source.dispatchEvent("dragend", {dataTransfer})
    } finally {
        await dataTransfer.dispose()
    }
    await expect(rows).toHaveText(["Bob Brown", "Alice Adams"])
    await page.getByRole("button", {name: "Save", exact: true}).click()
}

test("persists a dragged candidate order before saving the contest", async ({page, portal}) => {
    ballot(portal)
    await reorder(page, portal)
    await expect(notification(page, "Element updated")).toBeVisible()
    await page.clock.runFor(5001)
    await expect.poll(() => portal.graphql.callsTo("update_sequent_backend_contest").length).toBe(1)
    expect(
        portal.graphql.callsTo("update_sequent_backend_candidate").map(({variables}) => variables)
    ).toEqual([
        {
            where: {id: {_eq: CONTENT_IDS.secondCandidate}},
            _set: {presentation: {i18n: {en: {name: "Bob Brown"}}, sort_order: 0}},
        },
        {
            where: {id: {_eq: CONTENT_IDS.candidate}},
            _set: {presentation: {i18n: {en: {name: "Alice Adams"}}, sort_order: 1}},
        },
    ])
    expect(
        portal.graphql.callsTo("update_sequent_backend_contest").map(({variables}) => variables)
    ).toEqual([
        {
            where: {id: {_eq: IDS.contest}},
            _set: {
                description: "Ordered ballot",
                presentation: {
                    ...presentation,
                    columns: null,
                    max_selections_per_type: null,
                    i18n: {en: {name: "Mayor", description: "Ordered ballot"}},
                },
            },
        },
    ])
    expectRole(portal, "update_sequent_backend_candidate", "candidate-write")
    expectRole(portal, "update_sequent_backend_contest", "contest-write")
    await page.reload()
    await page.getByRole("button", {name: "Ballot Design", exact: true}).click()
    await expect(page.locator('[draggable="true"]')).toHaveText(["Bob Brown", "Alice Adams"])
})

test("reports a rejected candidate reorder and does not save the contest", async ({
    page,
    portal,
}) => {
    const rejections: string[] = []
    page.on("pageerror", (error) => rejections.push(error.message))
    ballot(portal)
    portal.graphql.on("update_sequent_backend_candidate", () => ({
        errors: [{message: "ordering denied"}],
    }))
    await reorder(page, portal)
    await expect(notification(page, "ordering denied")).toBeVisible()
    expect(
        portal.graphql.callsTo("update_sequent_backend_candidate").map(({variables}) => variables)
    ).toEqual([
        {
            where: {id: {_eq: CONTENT_IDS.secondCandidate}},
            _set: {presentation: {i18n: {en: {name: "Bob Brown"}}, sort_order: 0}},
        },
    ])
    expect(portal.graphql.callsTo("update_sequent_backend_contest")).toEqual([])

    expect(rejections).toEqual([])
})
