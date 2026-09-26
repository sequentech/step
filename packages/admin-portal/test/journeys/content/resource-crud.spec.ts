// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {PortalServices} from "@sequentech/ui-test-kit/adapters/playwright"
import {FIXED_TIME, IDS} from "@sequentech/ui-test-kit/fixtures"
import {test, expect} from "../fixtures"
import {
    BASE_ROLES,
    EVENT_SCOPE,
    areaRow,
    contestRow,
    electionRow,
    eventPage,
    expectRole,
    table,
} from "./data"

const AREA_CONTEST_ID = "a1000000-0000-4000-8000-000000000001"
const BALLOT_STYLE_ID = "b1000000-0000-4000-8000-000000000001"
const DOCUMENT_ID = "d1000000-0000-4000-8000-000000000001"
const LABELS = {print_code: "North"}
const AREA_CONTEST = {
    ...EVENT_SCOPE,
    area_id: IDS.area,
    contest_id: IDS.contest,
    labels: LABELS,
    annotations: {},
}
const BALLOT_STYLE = {
    ...EVENT_SCOPE,
    ballot_publication_id: "b2000000-0000-4000-8000-000000000001",
    area_id: IDS.area,
    election_id: IDS.election,
    ballot_eml: "<Ballot>North</Ballot>",
    status: "DRAFT",
    labels: {},
    annotations: {},
}
const DOCUMENT = {
    ...EVENT_SCOPE,
    name: "guide.txt",
    media_type: "text/plain",
    size: 16,
    is_public: false,
    labels: {},
    annotations: {},
}

test.use({
    roles: [
        ...BASE_ROLES,
        "area-read",
        "area-write",
        "contest-read",
        "publish-read",
        "publish-write",
        "document-read",
        "document-write",
        "tenant-read",
    ],
})

function resources(portal: PortalServices) {
    eventPage(portal)
    table(portal, "sequent_backend_area", [areaRow()])
    table(portal, "sequent_backend_election", [electionRow()])
    table(portal, "sequent_backend_contest", [contestRow()])
}

const CASES = [
    {
        resource: "area_contest",
        title: "Area Contest",
        id: AREA_CONTEST_ID,
        fields: AREA_CONTEST,
        role: "area-write",
    },
    {
        resource: "ballot_style",
        title: "Ballot Style",
        id: BALLOT_STYLE_ID,
        fields: BALLOT_STYLE,
        role: "publish-write",
    },
    {
        resource: "document",
        title: "Document",
        id: DOCUMENT_ID,
        fields: DOCUMENT,
        role: "document-write",
    },
] as const

for (const entry of CASES) {
    test(`creates ${entry.title.toLowerCase()} records and preserves the form after rejection`, async ({
        page,
        portal,
    }) => {
        resources(portal)
        const resource = `sequent_backend_${entry.resource}`
        const rows = table(portal, resource, [], () => entry.id)
        // React Admin's source query supplies the caller's tenant/publication context.
        // The legacy tenant selector displays username although tenants expose slug.
        const query = new URLSearchParams({lang: "en", source: JSON.stringify(entry.fields)})
        await page.goto(`${portal.origin}/${resource}/create?${query}`)
        await expect(page.getByRole("heading", {name: entry.title, exact: true})).toBeVisible()
        await page.getByRole("button", {name: "Save", exact: true}).click()
        await expect.poll(() => portal.graphql.callsTo(`insert_${resource}`).length).toBe(1)
        expect(portal.graphql.callsTo(`insert_${resource}`)[0].variables).toEqual({
            objects: entry.fields,
        })
        expectRole(portal, `insert_${resource}`, entry.role)
        await expect(page.getByRole("alert")).toContainText("Element created")
        expect(rows).toHaveLength(1)

        portal.graphql.once(`insert_${resource}`, () => ({
            errors: [{message: "Creation rejected"}],
        }))
        await page.goto(`${portal.origin}/${resource}/create?${query}`)
        await expect(page.getByRole("heading", {name: entry.title, exact: true})).toBeVisible()
        await page.getByRole("button", {name: "Save", exact: true}).click()
        await expect(page.getByRole("alert")).toContainText("Creation rejected")
        expect(
            portal.graphql.callsTo(`insert_${resource}`).map(({variables}) => variables)
        ).toEqual([{objects: entry.fields}, {objects: entry.fields}])
        await expect(page.getByRole("button", {name: "Save", exact: true})).toBeEnabled()
        await expect(page).toHaveURL((url) => url.pathname === `/${resource}/create`)
        expect(rows).toHaveLength(1)
    })
}

test("edits area-contest print labels", async ({page, portal}) => {
    resources(portal)
    table(portal, "sequent_backend_area_contest", [
        {id: AREA_CONTEST_ID, ...AREA_CONTEST, created_at: FIXED_TIME, last_updated_at: FIXED_TIME},
    ])
    await page.goto(`${portal.origin}/sequent_backend_area_contest/${AREA_CONTEST_ID}?lang=en`)
    await expect(page.getByRole("heading", {name: "Area", exact: true, level: 4})).toBeVisible()
    await page.getByText("...", {exact: true}).click()
    await page.getByText('"North"', {exact: true}).click({modifiers: ["Control"]})
    await page.getByPlaceholder("update this value").fill("North annex")
    await page.getByPlaceholder("update this value").press("Control+Enter")
    await page.getByRole("button", {name: "Save", exact: true}).click()
    await page.clock.runFor(6000)
    await expect
        .poll(() => portal.graphql.callsTo("update_sequent_backend_area_contest").length)
        .toBe(1)
    expect(portal.graphql.callsTo("update_sequent_backend_area_contest")[0].variables).toEqual({
        where: {id: {_eq: AREA_CONTEST_ID}},
        _set: {labels: {print_code: "North annex"}},
    })
    expectRole(portal, "update_sequent_backend_area_contest", "area-write")
    await page.goto(`${portal.origin}/sequent_backend_area_contest/${AREA_CONTEST_ID}?lang=en`)
    await page.getByText("...", {exact: true}).click()
    await expect(page.getByText('"North annex"', {exact: true})).toBeVisible()
})

test("edits a ballot style's printable ballot", async ({page, portal}) => {
    resources(portal)
    table(portal, "sequent_backend_ballot_style", [
        {id: BALLOT_STYLE_ID, ...BALLOT_STYLE, created_at: FIXED_TIME, last_updated_at: FIXED_TIME},
    ])
    await page.goto(`${portal.origin}/sequent_backend_ballot_style/${BALLOT_STYLE_ID}?lang=en`)
    await expect(page.getByRole("textbox", {name: "Ballot eml", exact: true})).toHaveValue(
        "<Ballot>North</Ballot>"
    )
    await page
        .getByRole("textbox", {name: "Ballot eml", exact: true})
        .fill("<Ballot>North annex</Ballot>")
    await page.getByRole("button", {name: "Save", exact: true}).click()
    await page.clock.runFor(6000)
    await expect
        .poll(() => portal.graphql.callsTo("update_sequent_backend_ballot_style").length)
        .toBe(1)
    expect(portal.graphql.callsTo("update_sequent_backend_ballot_style")[0].variables).toEqual({
        where: {id: {_eq: BALLOT_STYLE_ID}},
        _set: {ballot_eml: "<Ballot>North annex</Ballot>"},
    })
    expectRole(portal, "update_sequent_backend_ballot_style", "publish-write")
    await page.goto(`${portal.origin}/sequent_backend_ballot_style/${BALLOT_STYLE_ID}?lang=en`)
    await expect(page.getByRole("textbox", {name: "Ballot eml", exact: true})).toHaveValue(
        "<Ballot>North annex</Ballot>"
    )
})

for (const entry of CASES.filter((entry) => entry.resource !== "document")) {
    test(`deletes ${entry.title.toLowerCase()} records after its undo window`, async ({
        page,
        portal,
    }) => {
        resources(portal)
        const resource = `sequent_backend_${entry.resource}`
        table(portal, resource, [
            {id: entry.id, ...entry.fields, created_at: FIXED_TIME, last_updated_at: FIXED_TIME},
        ])
        await page.goto(`${portal.origin}/${resource}/${entry.id}?lang=en`)
        const remove = page.getByRole("toolbar").getByRole("button", {name: "Delete", exact: true})
        await expect(remove).toBeVisible()
        await remove.click()
        await expect(page.getByRole("button", {name: "Undo", exact: true})).toBeVisible()
        await page.getByRole("button", {name: "Undo", exact: true}).click()
        expect(portal.graphql.callsTo(`delete_${resource}`)).toEqual([])
        await page.goto(`${portal.origin}/${resource}/${entry.id}?lang=en`)
        await remove.click()
        await page.clock.runFor(6000)
        await expect.poll(() => portal.graphql.callsTo(`delete_${resource}`).length).toBe(1)
        expect(portal.graphql.callsTo(`delete_${resource}`)[0].variables).toEqual({
            where: {id: {_eq: entry.id}},
        })
        expectRole(portal, `delete_${resource}`, entry.role)
        await expect(page).toHaveURL((url) => url.pathname === `/${resource}`)
        await expect(page.getByRole("cell", {name: "North", exact: true})).toHaveCount(0)
    })
}
