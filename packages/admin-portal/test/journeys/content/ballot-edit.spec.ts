// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {PortalServices} from "@sequentech/ui-test-kit/adapters/playwright"
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

const editorRoles = [
    ...BASE_ROLES,
    "election-write",
    "election-data-tab",
    "contest-read",
    "contest-write",
    "candidate-read",
    "candidate-write",
]
test.use({roles: editorRoles})

function ballot(portal: PortalServices) {
    const election = electionRow()
    const contest = contestRow()
    eventPage(portal, undefined, [election])
    table(portal, "sequent_backend_election", [election])
    table(portal, "sequent_backend_contest", [contest])
    const candidates = table(portal, "sequent_backend_candidate", [
        candidateRow({
            external_id: "alice-original",
            presentation: {
                ...names("Alice Adams", {alias: "Alice", description: "Independent"}),
                invalid_vote_position: "bottom",
            },
        }),
    ])
    table(portal, "sequent_backend_document", [])
    portal.graphql.on("contest_tree", () => ({data: {sequent_backend_contest: [contest]}}))
    portal.graphql.on("candidate_tree", () => ({data: {sequent_backend_candidate: candidates}}))
    return candidates
}

test("saves candidate labels and disabled status through the data form", async ({page, portal}) => {
    ballot(portal)
    await page.goto(`${portal.origin}/sequent_backend_candidate/${CONTENT_IDS.candidate}?lang=en`)
    await expect(page.getByRole("textbox", {name: "Name", exact: true})).toHaveValue("Alice Adams")
    await page.getByRole("textbox", {name: "Name", exact: true}).fill("Alice Updated")
    await page.getByRole("textbox", {name: "Alias", exact: true}).fill("A. Updated")
    await page.getByRole("textbox", {name: "Description", exact: true}).fill("Community list")
    await page.getByRole("textbox", {name: "External ID", exact: true}).fill("alice-updated")
    await page.getByRole("textbox", {name: "IVR prompt", exact: true}).fill("Candidate Alice")
    await page.getByRole("switch", {name: "Disabled", exact: true}).check()
    await page.getByRole("button", {name: "Save", exact: true}).click()
    await expect(notification(page, "Element updated")).toBeVisible()
    await page.clock.runFor(5001)
    await expect
        .poll(() => portal.graphql.callsTo("update_sequent_backend_candidate").length)
        .toBe(1)
    expect(
        portal.graphql.callsTo("update_sequent_backend_candidate").map(({variables}) => variables)
    ).toEqual([
        {
            where: {id: {_eq: CONTENT_IDS.candidate}},
            _set: {
                description: "Community list",
                external_id: "alice-updated",
                annotations: {"ivr:i18n": '{"en":{"prompt":"Candidate Alice"}}'},
                presentation: {
                    i18n: {
                        en: {
                            name: "Alice Updated",
                            alias: "A. Updated",
                            description: "Community list",
                        },
                    },
                    invalid_vote_position: "bottom",
                    is_disabled: true,
                    is_category_list: false,
                    is_explicit_blank: false,
                    is_explicit_invalid: false,
                    is_write_in: false,
                    language_conf: {enabled_language_codes: []},
                },
            },
        },
    ])
    expectRole(portal, "update_sequent_backend_candidate", "candidate-write")
    await page.reload()
    await expect(page.getByRole("textbox", {name: "Name", exact: true})).toHaveValue(
        "Alice Updated"
    )
    await expect(page.getByRole("switch", {name: "Disabled", exact: true})).toBeChecked()
})

test.describe("with candidate write but no contest write permission", () => {
    test.use({roles: editorRoles.filter((role) => role !== "contest-write")})

    test("allows saving candidate changes independently of contest permissions", async ({
        page,
        portal,
    }) => {
        ballot(portal)
        await page.goto(
            `${portal.origin}/sequent_backend_candidate/${CONTENT_IDS.candidate}?lang=en`
        )
        await expect(page.getByRole("textbox", {name: "Name", exact: true})).toHaveValue(
            "Alice Adams"
        )
        await page.getByRole("textbox", {name: "Name", exact: true}).fill("Alice Updated")
        expect(portal.graphql.callsTo("update_sequent_backend_candidate")).toEqual([])
        test.fail(true, "CandidateDataForm gates Save on contest-write instead of candidate-write")
        await expect(page.getByRole("button", {name: "Save", exact: true})).toBeVisible({
            timeout: 2000,
        })
    })
})
