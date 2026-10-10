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
    portal.graphql.on("LocateBallot", () => ({data: {locate_ballot: located("not-found")}}))
}

function located(status: string, ballot: {ballot_id?: string; content?: string} = {}) {
    return {
        status,
        ballot_id: ballot.ballot_id ?? null,
        content: ballot.content ?? null,
        cast_at: ballot.content ? FIXED_TIME : null,
        checks_available_until: null,
    }
}

function checksUntil(portal: Portal, until: string) {
    Object.assign(portal.data.event.presentation, {
        receipts: {checks_period_policy: "until-date", checks_available_until: until},
    })
    portal.publish()
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
    expect(portal.graphql.callsTo("LocateBallot")).toEqual([])
})

for (const [status, message] of [
    ["not-found", "was not found"],
    ["found", "has been found"],
    ["ambiguous", "More than one of your ballots matches"],
] as const) {
    test(`locator renders a ${status} lookup without disclosing other content`, async ({
        page,
        portal,
    }) => {
        lookup(portal)
        const ballotId = "abcdef12".repeat(8)
        portal.graphql.on("LocateBallot", () => ({
            data: {
                locate_ballot:
                    status === "found"
                        ? located(status, {ballot_id: ballotId, content: "encrypted-content-0"})
                        : located(status),
            },
        }))
        await page.goto(`${portal.origin}${locatorPath}/${ballotId}?lang=en`)
        await expect(page.getByRole("status").filter({hasText: message})).toBeVisible()
        if (status === "found") {
            await expect(page.getByText("encrypted-content-0", {exact: true})).toBeVisible()
            await expect(page.getByText(/^Cast on /)).toBeVisible()
        } else {
            await expect(page.getByText(/encrypted-content-/)).toHaveCount(0)
            await expect(page.getByText(/^Cast on /)).toHaveCount(0)
        }
        expect(portal.graphql.callsTo("LocateBallot")[0].variables).toEqual({
            electionEventId: IDS.event,
            electionId: IDS.election,
            ballotId,
        })
    })
}

test("the lookup sends the typed ID in lower case and leaves matching to the API", async ({
    page,
    portal,
}) => {
    lookup(portal)
    portal.data.election.voting_channels.telephone = true
    await page.goto(`${portal.origin}${locatorPath}/AB12?lang=en`)
    await expect(page.getByRole("status").filter({hasText: "was not found"})).toBeVisible()
    expect(portal.graphql.callsTo("LocateBallot")[0].variables.ballotId).toBe("ab12")
})

test("a failed lookup is not reported as a ballot that was not found", async ({page, portal}) => {
    lookup(portal)
    portal.graphql.on("LocateBallot", () => ({
        errors: [{message: "Unable to check the ballot"}],
    }))
    await page.goto(`${portal.origin}${locatorPath}/ab12?lang=en`)
    await expect(page.getByRole("status").filter({hasText: "Something went wrong"})).toBeVisible()
    await expect(page.getByText("was not found")).toHaveCount(0)
})

test("while checks are open the locator says until when", async ({page, portal}) => {
    lookup(portal)
    checksUntil(portal, "2999-06-07T23:59:00+08:00")
    await page.goto(`${portal.origin}${locatorPath}?lang=en`)
    await expect(page.getByText(/You can check your ballot until .*2999/)).toBeVisible()
    await expect(page.getByRole("textbox")).toBeVisible()
})

test("after the checks period the locator offers no lookup and no logs", async ({page, portal}) => {
    lookup(portal)
    Object.assign(portal.data.event.presentation, {show_cast_vote_logs: "show-logs-tab"})
    checksUntil(portal, "2000-06-15T12:00:00Z")
    await page.goto(`${portal.origin}${locatorPath}/ab12?lang=en`)
    await expect(page.getByRole("status").filter({hasText: /Checks ended on .*2000/})).toBeVisible()
    await expect(page.getByRole("textbox")).toHaveCount(0)
    await expect(page.getByRole("tab", {name: "Logs", exact: true})).toHaveCount(0)
    await expect(page.getByRole("button", {name: "Find another Ballot"})).toHaveCount(0)
    expect(portal.graphql.callsTo("LocateBallot")).toEqual([])
    expect(portal.graphql.callsTo("listCastVoteMessages")).toEqual([])
})

test("the API's answer that checks ended is shown even when this device's clock disagrees", async ({
    page,
    portal,
}) => {
    lookup(portal)
    portal.graphql.on("LocateBallot", () => ({
        data: {
            locate_ballot: {
                ...located("checks-ended"),
                checks_available_until: "2000-06-15T12:00:00+00:00",
            },
        },
    }))
    await page.goto(`${portal.origin}${locatorPath}/ab12?lang=en`)
    await expect(page.getByRole("status").filter({hasText: /Checks ended on .*2000/})).toBeVisible()
    await expect(page.getByText("was not found")).toHaveCount(0)
})

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
