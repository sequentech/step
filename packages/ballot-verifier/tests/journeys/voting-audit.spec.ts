// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {readFile} from "node:fs/promises"
import {
    test as votingTest,
    expect,
    eventPath as votingEventPath,
    IDS,
} from "../../../voting-portal/test/journeys/fixtures"
import {serveDist} from "@sequentech/ui-test-kit/server/static"
import {loadCore} from "@sequentech/ui-test-kit/wasm/node"
import type {IDecodedVoteContest} from "sequent-core"
import {verifierServices, eventPath, serveVerifier, routeVerifier} from "./fixtures"

const test = votingTest.extend<{}, {verifierDist: Awaited<ReturnType<typeof serveDist>>}>({
    verifierDist: [
        // Playwright requires destructuring even without fixture dependencies.
        // eslint-disable-next-line no-empty-pattern
        async ({}, use) => {
            const dist = await serveVerifier()
            try {
                await use(dist)
            } finally {
                await dist.close()
            }
        },
        {scope: "worker"},
    ],
})

test("voting portal audit download verifies unchanged in the production verifier", async ({
    page,
    context,
    portal,
    verifierDist,
}) => {
    await page.goto(`${portal.origin}${votingEventPath}?lang=en`)
    await page.getByRole("button", {name: /click to vote/i}).click()
    await page.getByRole("button", {name: "Start Voting", exact: true}).click()
    await page.getByRole("checkbox", {name: /Alice Example/}).check()
    await page.getByRole("button", {name: "Next", exact: true}).click()
    const hash = (await page.getByText(/^Your Ballot ID:/).textContent())?.match(
        /[0-9a-f]{64}/
    )?.[0]
    expect(hash).toMatch(/^[0-9a-f]{64}$/)
    await page.getByRole("button", {name: "Audit ballot", exact: true}).click()
    await page.getByRole("button", {name: "Yes, discard my ballot to audit"}).click()
    const downloading = page.waitForEvent("download")
    await page.getByRole("button", {name: "Download", exact: true}).click()
    const download = await downloading
    const bytes = await readFile((await download.path())!)
    const core = await loadCore()
    const decoded = core.decode_auditable_ballot_js(
        JSON.parse(bytes.toString())
    ) as IDecodedVoteContest[]
    expect(decoded[0].choices.filter(({selected}) => selected >= 0)).toEqual([
        {id: IDS.alice, selected: 0, write_in_text: null},
    ])
    expect(portal.graphql.callsTo("InsertCastVote")).toEqual([])

    // A second application origin in this context starts a separate OIDC session.
    const verifier = verifierServices(verifierDist.origin)
    const unroute = await routeVerifier(context, verifier)
    try {
        await page.goto(`${verifier.origin}${eventPath}`)
        await page.getByTestId("drop-input-file").setInputFiles({
            name: download.suggestedFilename(),
            mimeType: "application/json",
            buffer: bytes,
        })
        await page.getByRole("textbox", {name: "Ballot ID", exact: true}).fill(hash!)
        await page.getByRole("button", {name: "Next", exact: true}).click()
        await expect(
            page.getByRole("heading", {name: "Verify your ballot selections"})
        ).toBeVisible()
        await expect(page.getByText("Alice Example", {exact: true})).toBeVisible()
        await expect(page.getByText("Bob Example", {exact: true})).toHaveCount(0)
        await expect(page.getByText(hash!, {exact: true})).toHaveCount(2)
        expect(verifier.oidc.tokenRequests[0]).toMatchObject({status: 200, pkce: "valid"})
    } finally {
        await unroute()
        expect(verifier.violations.list(), "unexpected verifier requests").toEqual([])
    }
})
