// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {test, expect, eventPath, IDS, FIXED_TIME, type Portal} from "./fixtures"

const locatorPath = `${eventPath}/election/${IDS.election}/ballot-locator`
function lookup(portal: Portal) {
    portal.graphql.on("GetElectionEvent", () => ({
        data: {sequent_backend_election_event: [portal.data.event]},
    }))
    portal.graphql.on("GetElections", () => ({
        data: {sequent_backend_election: [portal.data.election]},
    }))
    portal.graphql.on("GetCastVote", () => ({data: {sequent_backend_cast_vote: []}}))
}

test("locator rejects empty, odd-length and non-hex IDs before requesting a lookup", async ({
    page,
    portal,
}) => {
    lookup(portal)
    await page.goto(`${portal.origin}${locatorPath}?lang=en`)
    const input = page.getByRole("textbox")
    const locate = page.getByRole("button", {name: "Find your Ballot", exact: true})
    await input.fill("ab12")
    await expect(locate).toBeEnabled()
    for (const invalid of ["", "a", "abc", "zz12"]) {
        await input.fill(invalid)
        await expect(locate).toBeDisabled()
    }
    expect(portal.graphql.callsTo("GetCastVote")).toEqual([])
})

for (const [count, message] of [
    [0, "was not found"],
    [1, "has been found"],
    [2, "More than one of your ballots matches"],
] as const) {
    test(`locator renders ${count} matching ballots without disclosing ambiguous content`, async ({
        page,
        portal,
    }) => {
        lookup(portal)
        const ballotId = "abcdef12".repeat(8)
        portal.graphql.on("GetCastVote", () => ({
            data: {
                sequent_backend_cast_vote: Array.from({length: count}, (_, index) => ({
                    ballot_id: ballotId,
                    content: `encrypted-content-${index}`,
                })),
            },
        }))
        await page.goto(`${portal.origin}${locatorPath}/${ballotId}?lang=en`)
        await expect(page.getByRole("status").filter({hasText: message})).toBeVisible()
        if (count === 1)
            await expect(page.getByText("encrypted-content-0", {exact: true})).toBeVisible()
        else await expect(page.getByText(/encrypted-content-/)).toHaveCount(0)
        expect(portal.graphql.callsTo("GetCastVote")[0].variables).toEqual({
            tenantId: IDS.tenant,
            electionEventId: IDS.event,
            electionId: IDS.election,
            ballotIdPattern: ballotId,
        })
    })
}

for (const telephone of [false, true]) {
    test(`four-character lookup uses a prefix only for telephone voting: ${telephone}`, async ({
        page,
        portal,
    }) => {
        lookup(portal)
        portal.data.election.voting_channels.telephone = telephone
        await page.goto(`${portal.origin}${locatorPath}/AB12?lang=en`)
        await expect(page.getByRole("status").filter({hasText: "was not found"})).toBeVisible()
        expect(portal.graphql.callsTo("GetCastVote")[0].variables.ballotIdPattern).toBe(
            telephone ? "ab12%" : "ab12"
        )
    })
}

test("cast log sorting and paging send scoped variables and display returned rows", async ({
    page,
    portal,
}) => {
    lookup(portal)
    Object.assign(portal.data.event.presentation, {show_cast_vote_logs: "show-logs-tab"})
    portal.publish()
    portal.graphql.on("listCastVoteMessages", ({variables}) => ({
        data: {
            list_cast_vote_messages: {
                total: 11,
                list: [
                    {
                        statement_timestamp: Date.parse(FIXED_TIME) / 1000,
                        statement_kind: "CAST_VOTE",
                        ballot_id: "ab12",
                        username: `voter-${variables.offset ?? 0}`,
                        message: "Ballot accepted",
                    },
                ],
            },
        },
    }))
    await page.goto(`${portal.origin}${locatorPath}?lang=en`)
    await page.getByRole("tab", {name: "Logs", exact: true}).click()
    await expect(page.getByRole("cell", {name: "voter-0", exact: true})).toBeVisible()
    expect(portal.graphql.callsTo("listCastVoteMessages")[0].variables).toMatchObject({
        tenantId: IDS.tenant,
        electionEventId: IDS.event,
        electionId: IDS.election,
        ballotId: "",
        limit: 5,
        offset: 0,
        orderBy: {id: "desc"},
    })
    await page.clock.runFor(600)
    await page.getByRole("button", {name: "Username", exact: true}).click()
    await expect
        .poll(() => portal.graphql.callsTo("listCastVoteMessages").at(-1)?.variables.orderBy)
        .toEqual({username: "asc"})
    await page.clock.runFor(600)
    await page.getByRole("button", {name: "Go to next page"}).click()
    await expect(page.getByRole("cell", {name: "voter-5", exact: true})).toBeVisible()
    expect(portal.graphql.callsTo("listCastVoteMessages").at(-1)?.variables).toMatchObject({
        offset: 5,
        limit: 5,
    })
    await page.clock.runFor(600)
    await page.getByRole("combobox", {name: "Rows per page:"}).click()
    await page.getByRole("option", {name: "10", exact: true}).click()
    await expect
        .poll(() => portal.graphql.callsTo("listCastVoteMessages").at(-1)?.variables)
        .toMatchObject({offset: 0, limit: 10})
})
