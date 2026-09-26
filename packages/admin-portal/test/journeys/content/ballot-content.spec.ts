// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Page} from "@playwright/test"
import type {PortalServices} from "@sequentech/ui-test-kit/adapters/playwright"
import {IDS} from "@sequentech/ui-test-kit/fixtures"
import {test, expect, TENANT_ID} from "../fixtures"
import {
    BASE_ROLES,
    CONTENT_IDS,
    type Row,
    candidateRow,
    contestRow,
    electionRow,
    eventPage,
    eventRow,
    everyLanguage,
    expectRole,
    answerInvalid,
    names,
    notification,
    table,
} from "./data"

const editorRoles = [
    ...BASE_ROLES,
    "election-create",
    "election-write",
    "election-data-tab",
    "contest-read",
    "contest-create",
    "contest-write",
    "candidate-read",
    "candidate-create",
    "candidate-write",
]
const NEW_ELECTION_ID = "30000000-0000-4000-8000-000000000009"
const NEW_CONTEST_ID = "60000000-0000-4000-8000-000000000009"
const NEW_CANDIDATE_ID = "90000000-0000-4000-8000-000000000009"

function ballot(portal: PortalServices, electionOverrides: Row = {}) {
    const election = electionRow({presentation: names("Mayor election"), ...electionOverrides})
    const contests = [
        contestRow({presentation: {...names("Mayor"), sort_order: 0}}),
        contestRow({
            id: CONTENT_IDS.secondContest,
            presentation: {...names("Council seats"), sort_order: 1},
        }),
    ]
    const candidates = [
        candidateRow(),
        candidateRow({id: CONTENT_IDS.secondCandidate, presentation: names("Bob Brown")}),
    ]
    eventPage(portal, eventRow(), [election])
    const elections = table(portal, "sequent_backend_election", [election], () => NEW_ELECTION_ID)
    const contestRows = table(portal, "sequent_backend_contest", contests, () => NEW_CONTEST_ID)
    const candidateRows = table(
        portal,
        "sequent_backend_candidate",
        candidates,
        () => NEW_CANDIDATE_ID
    )
    table(portal, "sequent_backend_document", [])
    portal.graphql.on("contest_tree", () => ({data: {sequent_backend_contest: contestRows}}))
    portal.graphql.on("candidate_tree", () => ({
        data: {sequent_backend_candidate: candidateRows},
    }))
    portal.graphql.on("CreateElection", ({variables}) => {
        elections.push(
            electionRow({
                id: NEW_ELECTION_ID,
                external_id: variables.externalId,
                description: variables.description,
                presentation: variables.presentation,
            })
        )
        return {data: {create_election: {id: NEW_ELECTION_ID}}}
    })
    return {elections, contests: contestRows, candidates: candidateRows}
}

async function open(page: Page, portal: PortalServices, path: string) {
    await page.goto(`${portal.origin}${path}${path.includes("?") ? "&" : "?"}lang=en`)
}

test.describe("ballot content editor", () => {
    test.use({roles: editorRoles})

    test("creates an election in the event and opens its data", async ({page, portal}) => {
        const {elections} = ballot(portal)
        await open(page, portal, `/sequent_backend_election/create?electionEventId=${IDS.event}`)
        await page.getByRole("textbox", {name: "Name"}).fill("Council election")
        await page.getByRole("textbox", {name: "External ID"}).fill("council-2026")
        await page.getByRole("textbox", {name: "Description"}).fill("Council seats")
        await page.getByRole("button", {name: "Save", exact: true}).click()
        await expect(page).toHaveURL(new RegExp(`/sequent_backend_election/${NEW_ELECTION_ID}`))
        expect(portal.graphql.callsTo("CreateElection")[0].variables).toEqual({
            electionEventId: IDS.event,
            externalId: "council-2026",
            description: "Council seats",
            presentation: {
                i18n: everyLanguage({name: "Council election", description: "Council seats"}),
                language_conf: {enabled_language_codes: ["en"], default_language_code: "en"},
            },
        })
        expect(elections.map((row) => row.id)).toEqual([IDS.election, NEW_ELECTION_ID])
        await expect(page.getByRole("tab", {name: "Data"})).toBeVisible()
    })

    test("creates a contest in an election and opens it", async ({page, portal}) => {
        test.fail(
            true,
            'CreateContest sends the default vote limits as strings (Contest/CreateContest.tsx:103-104: defaultValue="0"/"1")'
        )
        const {contests} = ballot(portal)
        // Hasura rejects string Int variables; answer that way so the defect stays visible.
        answerInvalid(
            portal,
            "insert_sequent_backend_contest",
            (variables) => typeof (variables.objects as Row).min_votes === "string",
            'expected a 32-bit integer for type "Int", but found a string'
        )
        await open(
            page,
            portal,
            `/sequent_backend_contest/create?electionEventId=${IDS.event}&electionId=${IDS.election}`
        )
        await page.getByRole("textbox", {name: "Name"}).fill("Referendum")
        await page.getByRole("textbox", {name: "Description"}).fill("Yes or no")
        const request = page.waitForRequest(
            (candidate) =>
                candidate.postDataJSON()?.operationName === "insert_sequent_backend_contest"
        )
        await page.getByRole("button", {name: "Save", exact: true}).click()
        const insert = (await request).postDataJSON().variables
        expect(insert).toEqual({
            objects: {
                description: "Yes or no",
                is_acclaimed: false,
                is_active: true,
                min_votes: 0,
                max_votes: 1,
                winning_candidates_num: 1,
                counting_algorithm: "plurality-at-large",
                is_encrypted: true,
                tenant_id: TENANT_ID,
                election_event_id: IDS.event,
                election_id: IDS.election,
                presentation: {
                    allow_writeins: true,
                    candidates_order: "alphabetical",
                    i18n: everyLanguage({name: "Referendum", description: "Yes or no"}),
                },
            },
        })
        await expect(page).toHaveURL(new RegExp(`/sequent_backend_contest/${NEW_CONTEST_ID}`), {
            timeout: 3000,
        })
        expectRole(portal, "insert_sequent_backend_contest", "contest-create")
        expect(contests.map((row) => row.id)).toContain(NEW_CONTEST_ID)
    })

    test("creates a candidate in a contest and opens it", async ({page, portal}) => {
        const {candidates} = ballot(portal)
        await open(
            page,
            portal,
            `/sequent_backend_candidate/create?electionEventId=${IDS.event}&contestId=${IDS.contest}`
        )
        await page.getByRole("textbox", {name: "Name"}).fill("Carol Clark")
        await page.getByRole("textbox", {name: "Description"}).fill("Green list")
        await page.getByRole("button", {name: "Save", exact: true}).click()
        await expect(page).toHaveURL(new RegExp(`/sequent_backend_candidate/${NEW_CANDIDATE_ID}`))
        const insert = portal.graphql.callsTo("insert_sequent_backend_candidate")[0].variables
        expect(insert).toEqual({
            objects: {
                is_public: false,
                description: "Green list",
                tenant_id: TENANT_ID,
                election_event_id: IDS.event,
                contest_id: IDS.contest,
                presentation: {
                    i18n: everyLanguage({name: "Carol Clark", description: "Green list"}),
                },
            },
        })
        expectRole(portal, "insert_sequent_backend_candidate", "candidate-create")
        expect(candidates.map((row) => row.id)).toContain(NEW_CANDIDATE_ID)
        await expect(page.getByRole("tab", {name: "Data"})).toBeVisible()
    })
})
