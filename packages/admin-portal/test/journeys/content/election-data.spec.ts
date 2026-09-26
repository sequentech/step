// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Page} from "@playwright/test"
import type {PortalServices} from "@sequentech/ui-test-kit/adapters/playwright"
import {IDS} from "@sequentech/ui-test-kit/fixtures"
import {test, expect} from "../fixtures"
import {
    BASE_ROLES,
    CONTENT_IDS,
    catchRejections,
    contestRow,
    electionRow,
    eventPage,
    expectRole,
    names,
    notification,
    table,
} from "./data"

test.use({
    roles: [...BASE_ROLES, "election-write", "election-data-tab", "contest-read", "contest-write"],
})
const presentation = {
    i18n: {en: {name: "Mayor election", alias: "Mayor", description: "Election of the mayor"}},
    language_conf: {enabled_language_codes: ["en"], default_language_code: "en"},
    dates: {},
    contests_order: "custom",
    audit_button_cfg: "show",
    cast_vote_gold_level: "no-gold-level",
    start_screen_title_policy: "election",
    security_confirmation_policy: "none",
    initialization_report_policy: "not-required",
    grace_period_policy: "no-grace-period",
    consolidated_report_policy: "do-not-generate",
    decline_to_vote_policy: "disabled",
    blank_ballots_policy: "disabled",
    voting_screen_back_policy: "election-selection-screen",
}

function ballot(portal: PortalServices) {
    const election = electionRow({presentation})
    eventPage(portal, undefined, [election])
    table(portal, "sequent_backend_election", [election])
    const contests = table(portal, "sequent_backend_contest", [
        contestRow({presentation: {...names("Mayor"), sort_order: 0}}),
        contestRow({
            id: CONTENT_IDS.secondContest,
            name: "Council seats",
            presentation: {...names("Council seats"), sort_order: 1},
        }),
    ])
    table(portal, "sequent_backend_document", [])
    portal.graphql.on("contest_tree", () => ({data: {sequent_backend_contest: contests}}))
}

async function reorder(page: Page, portal: PortalServices) {
    await page.goto(`${portal.origin}/sequent_backend_election/${IDS.election}?lang=en`)
    await page.getByRole("textbox", {name: "Name", exact: true}).fill("City election")
    await page.getByRole("textbox", {name: "Description", exact: true}).fill("Ordered city ballot")
    await page.getByRole("textbox", {name: "IVR prompt", exact: true}).fill("City voting")
    await page.getByRole("button", {name: "Ballot Design", exact: true}).click()
    const rows = page.locator('[draggable="true"]')
    await expect(rows).toHaveText(["Mayor", "Council seats"])
    const dataTransfer = await page.evaluateHandle(() => new DataTransfer())
    try {
        await rows.filter({hasText: "Council seats"}).dispatchEvent("dragstart", {dataTransfer})
        await rows.filter({hasText: "Mayor"}).dispatchEvent("dragover", {dataTransfer})
        await rows.filter({hasText: "Mayor"}).dispatchEvent("drop", {dataTransfer})
        await rows.filter({hasText: "Council seats"}).dispatchEvent("dragend", {dataTransfer})
    } finally {
        await dataTransfer.dispose()
    }
    await expect(rows).toHaveText(["Council seats", "Mayor"])
    await page.getByRole("button", {name: "Save", exact: true}).click()
}

const contestWrites = [
    {
        where: {id: {_eq: CONTENT_IDS.secondContest}},
        _set: {presentation: {i18n: {en: {name: "Council seats"}}, sort_order: 0}},
    },
    {
        where: {id: {_eq: IDS.contest}},
        _set: {presentation: {i18n: {en: {name: "Mayor"}}, sort_order: 1}},
    },
]

test("saves election text and a custom contest order with complete payloads", async ({
    page,
    portal,
}) => {
    ballot(portal)
    await reorder(page, portal)
    await expect(notification(page, "Element updated")).toBeVisible()
    await page.clock.runFor(5001)
    await expect
        .poll(() => portal.graphql.callsTo("update_sequent_backend_election").length)
        .toBe(1)
    expect(
        portal.graphql.callsTo("update_sequent_backend_contest").map(({variables}) => variables)
    ).toEqual(contestWrites)
    expect(
        portal.graphql.callsTo("update_sequent_backend_election").map(({variables}) => variables)
    ).toEqual([
        {
            where: {id: {_eq: IDS.election}},
            _set: {
                description: "Ordered city ballot",
                annotations: {"ivr:i18n": '{"en":{"prompt":"City voting"}}'},
                presentation: {
                    ...presentation,
                    cast_vote_confirm: false,
                    i18n: {
                        en: {
                            name: "City election",
                            alias: "Mayor",
                            description: "Ordered city ballot",
                        },
                    },
                },
                status: {allow_tally: "allowed"},
                receipts: {
                    EMAIL: {allowed: false, template: null},
                    SMS: {allowed: false, template: null},
                    DOCUMENT: {allowed: false, template: null},
                },
            },
        },
    ])
    expectRole(portal, "update_sequent_backend_contest", "contest-write")
    expectRole(portal, "update_sequent_backend_election", "election-write")
    await page.reload()
    await expect(page.getByRole("textbox", {name: "Name", exact: true})).toHaveValue(
        "City election"
    )
    await page.getByRole("button", {name: "Ballot Design", exact: true}).click()
    await expect(page.locator('[draggable="true"]')).toHaveText(["Council seats", "Mayor"])
})

test("reports contest reordering rejection without saving its parent election", async ({
    page,
    portal,
}) => {
    ballot(portal)
    const rejections = await catchRejections(page, "contest ordering denied")
    portal.graphql.on("update_sequent_backend_contest", () => ({
        errors: [{message: "contest ordering denied"}],
    }))
    await reorder(page, portal)
    await expect(notification(page, "contest ordering denied")).toBeVisible()
    expect(
        portal.graphql.callsTo("update_sequent_backend_contest").map(({variables}) => variables)
    ).toEqual([contestWrites[0]])
    expect(portal.graphql.callsTo("update_sequent_backend_election")).toEqual([])
    test.fail(
        true,
        "EditElectionData rethrows the rejected async transform without handling its promise"
    )
    expect(await rejections()).toEqual([])
})
