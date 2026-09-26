// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {test, expect, eventPath, realm, resultsIds} from "./fixtures"
import {scanPage} from "@sequentech/ui-test-kit/adapters/axe"

test("public index, manifest and SQLite render literal totals and area/election selections", async ({
    page,
    portal,
}) => {
    await page.goto(portal.origin + eventPath)
    await expect(page.getByRole("heading", {name: "Community Election Results"})).toBeVisible()
    await expect(page.getByRole("row", {name: /Alice Example/})).toContainText("45")
    await expect(page.getByRole("row", {name: /Bob Example/})).toContainText("30")
    await page.getByRole("tab", {name: "North district", exact: true}).click()
    await expect(page.getByRole("row", {name: /Alice Example/})).toContainText("18")
    await expect(page.getByRole("row", {name: /Bob Example/})).toContainText("12")
    await page.getByRole("tab", {name: "South district", exact: true}).click()
    await expect(page.getByText("Not published yet", {exact: true}).first()).toBeVisible()
    await expect(page.getByRole("row", {name: /Alice Example/})).toHaveCount(0)
    await page.getByRole("tab", {name: "School Board", exact: true}).click()
    await expect(page.getByRole("row", {name: /Charlie Example/})).toContainText("20")
    expect(portal.graphql.calls).toEqual([])
    expect(portal.oidc.authorizations).toEqual([])
    expect(portal.s3.requestsFor("results/manifest.json")).toHaveLength(1)
    expect(portal.s3.requestsFor("results/full.sqlite")).toHaveLength(1)
})

test("election deep link selects only the requested election", async ({page, portal}) => {
    await page.goto(`${portal.origin}/${resultsIds.event}/elections/school?lang=en`)
    await expect(page.getByRole("tab", {name: "School Board", exact: true})).toHaveAttribute(
        "aria-selected",
        "true"
    )
    await expect(page.getByRole("row", {name: /Charlie Example/})).toContainText("20")
    await expect(page.getByRole("row", {name: /Alice Example/})).toHaveCount(0)
})

test("404 publication index renders not published without signing in", async ({page, portal}) => {
    await page.goto(portal.origin + eventPath)
    await expect(page.getByRole("row", {name: /Alice Example/})).toContainText("45")
    portal.s3.override(({key}) => key.startsWith("results-index/"), {status: 404})
    await page.reload()
    await expect(page.getByRole("heading", {name: "Results not published yet"})).toBeVisible()
    expect(portal.graphql.calls).toEqual([])
    expect(portal.oidc.authorizations).toEqual([])
    expect(portal.s3.requestsFor("results/full.sqlite")).toHaveLength(1)
})

for (const key of ["results/manifest.json", "results/full.sqlite"]) {
    test(`failed ${key} shows load error and refresh recovers`, async ({page, portal}) => {
        await page.goto(portal.origin + eventPath)
        await expect(page.getByRole("row", {name: /Alice Example/})).toContainText("45")
        portal.s3.override((request) => request.key === key, {status: 500})
        await page.reload()
        await expect(page.getByRole("heading", {name: "Unexpected error"})).toBeVisible()
        await expect(
            page.getByText(
                "We could not load results right now. Please try again in a few minutes."
            )
        ).toBeVisible()
        await expect(page.getByRole("row", {name: /Alice Example/})).toHaveCount(0)
        portal.s3.clearOverrides()
        await page.reload()
        await expect(page.getByRole("row", {name: /Alice Example/})).toContainText("45")
    })
}

test("authenticated publication uses PKCE and bearer queries before a signed SQLite download", async ({
    page,
    portal,
}) => {
    portal.data.index.publications![0].access = "authenticated"
    portal.data.manifest.access = "authenticated"
    portal.data.manifest.artifacts = {full_sqlite: {document_id: "private-sqlite"}}
    await portal.publish()
    await page.goto(`${portal.origin}/${resultsIds.event}/elections/council?lang=en`)
    await expect(page.getByRole("row", {name: /Alice Example/})).toContainText("45")
    await expect(page.getByText("Signed-in access", {exact: true})).toBeVisible()
    expect(portal.oidc.authorizations).toHaveLength(1)
    expect(portal.oidc.authorizations[0]).toMatchObject({
        realm,
        params: {client_id: "results-portal"},
    })
    expect(portal.oidc.authorizations[0].params.code_challenge_method).toBe("S256")
    expect(portal.graphql.callsTo("ResolveResultsPublication")[0].variables).toEqual({
        eeId: "event-results",
        electionId: "council",
    })
    expect(portal.graphql.callsTo("FetchResultsArtifact")[0].variables).toEqual({
        electionEventId: "event-results",
        electionId: "council",
        publicationId: "publication-results",
    })
    expect(portal.s3.requestsFor("results/area.sqlite")[0].query["X-Amz-Signature"]).toBe(
        "results-signature"
    )
    expect(portal.s3.requestsFor("results/full.sqlite")).toHaveLength(0)
})

test("area-based authenticated data has no global or other-area tab", async ({page, portal}) => {
    portal.data.index.publications![0].access = "authenticated"
    portal.data.manifest.access = "authenticated"
    portal.data.manifest.visibility_scope = "area_based"
    portal.data.manifest.election_ids = ["council"]
    portal.data.manifest.contests = [portal.data.manifest.contests[1]]
    portal.data.manifest.artifacts = {areas: {north: {document_id: "private-area"}}}
    // The artifact contains only the permitted area's rows, as the resolver's scope promises.
    portal.data.dataset.results_contest = []
    portal.data.dataset.results_contest_candidate = []
    portal.data.dataset.results_election = []
    portal.data.dataset.election = portal.data.dataset.election.filter(({id}) => id === "council")
    portal.data.dataset.contest = portal.data.dataset.contest.filter(
        ({id}) => id === "representative"
    )
    portal.data.dataset.candidate = portal.data.dataset.candidate.filter(
        ({contest_id}) => contest_id === "representative"
    )
    portal.data.dataset.area = portal.data.dataset.area.filter(({id}) => id === "north")
    await portal.publish()
    await page.goto(portal.origin + eventPath)
    await expect(page.getByRole("row", {name: /Alice Example/})).toContainText("18")
    await expect(page.getByRole("tab", {name: "North district", exact: true})).toHaveAttribute(
        "aria-selected",
        "true"
    )
    await expect(page.getByRole("tab", {name: "Global", exact: true})).toHaveCount(0)
    await expect(page.getByRole("tab", {name: "South district", exact: true})).toHaveCount(0)
    await expect(page.getByRole("tab", {name: "School Board", exact: true})).toHaveCount(0)
})

for (const failure of ["no-publication", "no-artifact", "token"] as const) {
    test(`authenticated ${failure} failure hides the previous results`, async ({page, portal}) => {
        portal.data.index.publications![0].access = "authenticated"
        portal.data.manifest.access = "authenticated"
        portal.data.manifest.artifacts = {full_sqlite: {document_id: "private-sqlite"}}
        await portal.publish()
        await page.goto(portal.origin + eventPath)
        await expect(page.getByRole("row", {name: /Alice Example/})).toContainText("45")
        if (failure === "no-publication")
            portal.graphql.on("ResolveResultsPublication", () => ({
                data: {resolveResultsPublication: null},
            }))
        else if (failure === "no-artifact")
            portal.graphql.on("FetchResultsArtifact", () => ({
                data: {fetchResultsArtifact: {urls: []}},
            }))
        else portal.oidc.tokenFailure = {status: 500, body: '{"error":"temporarily_unavailable"}'}
        await page.reload()
        await expect(
            page.getByRole("heading", {
                name:
                    failure === "no-publication" ? "Results not published yet" : "Unexpected error",
            })
        ).toBeVisible()
        if (failure === "token")
            await expect(
                page.getByText(
                    "We could not complete sign-in for results right now. Please try again in a few minutes."
                )
            ).toBeVisible()
        await expect(page.getByRole("row", {name: /Alice Example/})).toHaveCount(0)
        // A settled failure must stop fetching; clearing auth identity used to restart discovery.
        await page.waitForLoadState("networkidle", {timeout: 3000})
        expect(portal.graphql.callsTo("ResolveResultsPublication")).toHaveLength(
            failure === "token" ? 1 : 2
        )
        expect(portal.graphql.callsTo("FetchResultsArtifact")).toHaveLength(
            failure === "no-artifact" ? 2 : 1
        )
    })
}

test("published results meet accessibility rules", async ({page, portal}) => {
    await page.goto(portal.origin + eventPath)
    await expect(page.getByRole("row", {name: /Alice Example/})).toContainText("45")
    const violations = await scanPage(page)
    if (violations.length)
        expect(violations).toEqual([
            {
                id: "color-contrast",
                impact: "serious",
                targets: [
                    [".seq-results-selector__election-tab--council"],
                    [".seq-results-selector__contest-row__tab"],
                    [".seq-results-selector__area-tab--global"],
                ],
            },
        ])
    expect(violations).toEqual([])
})

test("mobile results remain selectable across areas and elections", async ({page, portal}) => {
    await page.setViewportSize({width: 390, height: 844})
    await page.goto(portal.origin + eventPath)
    await expect(page.getByRole("row", {name: /Alice Example/})).toContainText("45")
    await page.getByRole("tab", {name: "North district", exact: true}).click()
    await expect(page.getByRole("row", {name: /Alice Example/})).toContainText("18")
    await page.getByRole("tab", {name: "School Board", exact: true}).click()
    await expect(page.getByRole("row", {name: /Charlie Example/})).toContainText("20")
})

test("changing event in the mounted router never resolves with the previous event token", async ({
    page,
    portal,
}) => {
    portal.data.index.publications![0].access = "authenticated"
    portal.data.manifest.access = "authenticated"
    portal.data.manifest.artifacts = {full_sqlite: {document_id: "private-sqlite"}}
    await portal.publish()
    await page.goto(portal.origin + eventPath)
    await expect(page.getByRole("row", {name: /Alice Example/})).toContainText("45")
    const nextManifest = {
        ...portal.data.manifest,
        election_event_id: "other-event",
        title: {en: "Second event results"},
    }
    portal.s3.putJson("public", "results-index/other-event.json", {
        ...portal.data.index,
        election_event_id: "other-event",
        publications: [{...portal.data.index.publications![0], route: "/other-event"}],
    })
    portal.graphql.on("ResolveResultsPublication", ({variables, headers}) => {
        expect(variables.eeId).toBe("other-event")
        const claims = portal.oidc.verifyAccessToken(headers.authorization.replace(/^Bearer /, ""))
        expect(claims?.iss).toBe(
            `${portal.oidc.serverUrl()}realms/tenant-tenant-results-event-other-event`
        )
        return {
            data: {
                resolveResultsPublication: {
                    tenant_id: resultsIds.tenant,
                    election_event_id: "other-event",
                    access: "authenticated",
                    route_scope: "event",
                    election_ids: nextManifest.election_ids,
                    publication_id: nextManifest.publication_id,
                    manifest: nextManifest,
                },
            },
        }
    })
    // Drive the browser router's history input without reloading its in-memory auth state.
    await page.evaluate(() => {
        history.pushState(history.state, "", "/other-event?lang=en")
        dispatchEvent(new PopStateEvent("popstate", {state: history.state}))
    })
    await expect(page.getByRole("heading", {name: "Second event results"})).toBeVisible()
    await expect(page.getByRole("heading", {name: "Community Election Results"})).toHaveCount(0)
    expect(portal.oidc.authorizations.map(({realm}) => realm)).toEqual([
        "tenant-tenant-results-event-event-results",
        "tenant-tenant-results-event-other-event",
    ])
    expect(portal.graphql.callsTo("ResolveResultsPublication")).toHaveLength(2)
})
