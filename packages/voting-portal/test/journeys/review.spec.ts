// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {test, expect, eventPath} from "./fixtures"
import {review} from "./flow"

for (const policy of ["show", "show-in-help", "not-show"]) {
    test(`audit policy ${policy} controls direct and help actions`, async ({page, portal}) => {
        portal.data.election.presentation.audit_button_cfg = policy
        portal.data.style.ballot_eml = JSON.stringify(portal.data.ballot)
        portal.publish()
        await review(page, portal)
        await expect(page.getByRole("button", {name: "Audit ballot", exact: true})).toHaveCount(
            policy === "show" ? 1 : 0
        )
        if (policy !== "not-show") {
            await page.getByRole("button", {name: "Your vote has not been cast"}).click()
            const dialog = page.getByRole("dialog")
            await expect(
                dialog.getByRole("button", {name: "Audit ballot", exact: true})
            ).toHaveCount(policy === "show-in-help" ? 1 : 0)
            if (policy === "show-in-help") {
                await dialog.getByRole("button", {name: "Audit ballot", exact: true}).click()
                await expect(
                    page.getByRole("dialog", {name: "Would you like to audit your ballot"})
                ).toBeVisible()
            }
        }
        expect(portal.graphql.callsTo("InsertCastVote")).toHaveLength(0)
    })
}

test("cast confirmation cancels safely and a rapid confirmation sends exactly one mutation", async ({
    page,
    portal,
}) => {
    portal.data.election.presentation.cast_vote_confirm = true
    portal.data.style.ballot_eml = JSON.stringify(portal.data.ballot)
    portal.publish()
    await review(page, portal)
    await page.getByRole("button", {name: "Cast ballot", exact: true}).click()
    let dialog = page.getByRole("dialog", {name: "Are you sure you want to cast your vote?"})
    await dialog.getByRole("button", {name: "Cancel"}).click()
    expect(portal.graphql.callsTo("InsertCastVote")).toHaveLength(0)
    await expect(page).toHaveURL(/\/review/)
    await page.getByRole("button", {name: "Cast ballot", exact: true}).click()
    dialog = page.getByRole("dialog", {name: "Are you sure you want to cast your vote?"})
    await dialog
        .getByRole("button", {name: "Yes, I want to cast my vote"})
        .evaluate((button: HTMLButtonElement) => {
            button.click()
            button.click()
        })
    await expect(page).toHaveURL(/\/confirmation/)
    expect(portal.graphql.callsTo("InsertCastVote")).toHaveLength(1)
})

for (const [code, message] of [
    ["InternalServerError", "An internal error occurred while casting your vote"],
    ["QueueError", "There was a problem processing your vote"],
    ["Unauthorized", "You are not authorized to cast a vote"],
    ["ElectionEventNotFound", "The election event could not be found"],
    ["ElectoralLogNotFound", "Your voting record could not be found"],
    ["CheckPreviousVotesFailed", "An error occurred while checking your voting status"],
    ["GetClientCredentialsFailed", "Failed to verify your credentials"],
    ["GetAreaIdFailed", "An error occurred while verifying your voting area"],
    ["GetTransactionFailed", "An error occurred while processing your vote"],
    ["DeserializeBallotFailed", "An error occurred while loading your ballot"],
    ["DeserializeContestsFailed", "An error occurred while loading your selections"],
    ["PokValidationFailed", "Failed to validate your vote"],
    ["UuidParseFailed", "An error occurred while processing your request"],
    ["CheckRevotesFailed", "You have exceeded the allowed number of revotes"],
    ["CheckVotesInOtherAreasFailed", "You have already voted in another area"],
    ["UnknownError", "An unknown error occurred while casting your vote"],
    ["BallotIdMismatch", "The ballot id does not match with the cast vote"],
    ["unexpected", "An unknown error occurred while casting your vote"],
    ["timeout", "A timeout occurred while casting your vote"],
    ["internal", "There was an internal error while casting the vote"],
    ["uncoded", "An unknown error occurred while casting your vote"],
]) {
    test(`cast ${code} renders its message and permits recovery`, async ({page, portal}) => {
        await review(page, portal)
        portal.graphql.once("InsertCastVote", () => ({
            errors: [
                {
                    message: code === "internal" ? "internal error" : "Synthetic action failure",
                    ...(code === "uncoded" || code === "internal"
                        ? {}
                        : {
                              extensions:
                                  code === "timeout"
                                      ? {
                                            code: "unexpected",
                                            internal: {error: {message: "Response timeout"}},
                                        }
                                      : {code},
                          }),
                },
            ],
        }))
        await page.getByRole("button", {name: "Cast ballot", exact: true}).click()
        await expect(page.getByRole("alert")).toContainText(message)
        await expect(page.getByRole("button", {name: "Cast ballot", exact: true})).toBeEnabled()
        await page.getByRole("button", {name: "Cast ballot", exact: true}).click()
        await expect(page).toHaveURL(/\/confirmation/)
        const calls = portal.graphql.callsTo("InsertCastVote")
        expect(calls).toHaveLength(2)
        expect(calls[1].variables).toEqual(calls[0].variables)
    })
}

test("fully acclaimed election reviews its winners without encrypting or casting", async ({
    page,
    portal,
}) => {
    Object.assign(portal.data.ballot.contests[0], {is_acclaimed: true})
    portal.data.style.ballot_eml = JSON.stringify(portal.data.ballot)
    portal.publish()
    await page.goto(`${portal.origin}${eventPath}?lang=en`)
    await page.getByRole("button", {name: /click to vote/i}).click()
    await page.getByRole("button", {name: "Start Voting", exact: true}).click()
    await expect(page.getByRole("checkbox", {name: /Alice Example/})).toBeDisabled()
    await page.getByRole("button", {name: "Next", exact: true}).click()
    await expect(page.getByRole("heading", {name: /Decided by acclamation/})).toBeVisible()
    await page.getByRole("button", {name: "Finish", exact: true}).click()
    await expect(page).toHaveURL(/\/confirmation/)
    await expect(page.getByTestId("ballot-id")).toHaveCount(0)
    await expect(page.getByRole("button", {name: "Print"})).toHaveCount(0)
    expect(portal.graphql.callsTo("InsertCastVote")).toEqual([])
})

for (const [fault, message] of [
    ["expired", "Your session has expired. Please start again"],
    ["malformed", "There was an error parsing the ballot data"],
    ["incomplete", "The ballot data is invalid"],
    ["absent", "Session storage is not available"],
]) {
    test(`gold return rejects ${fault} saved ballot data without casting`, async ({
        page,
        portal,
    }) => {
        await page.clock.setFixedTime(portal.now)
        portal.data.election.presentation.cast_vote_gold_level = "gold-level"
        portal.data.style.ballot_eml = JSON.stringify(portal.data.ballot)
        portal.publish()
        await review(page, portal)
        portal.oidc.signedIn = false
        await page.getByRole("button", {name: "Cast ballot", exact: true}).click()
        await expect(page.getByRole("button", {name: "Sign in", exact: true})).toBeVisible()
        const stored = await page.evaluate(() => ({
            data: sessionStorage.getItem("ballotData"),
            expiration: sessionStorage.getItem("ballotDataExpiration"),
        }))
        expect(JSON.parse(stored.data!)).toMatchObject({
            electionId: portal.data.election.id,
            isDemo: false,
        })
        expect(Number(stored.expiration)).toBe(portal.now + 300000)
        await page.evaluate(
            ({fault, now}) => {
                if (fault === "expired")
                    sessionStorage.setItem("ballotDataExpiration", String(now - 1))
                else if (fault === "malformed") sessionStorage.setItem("ballotData", "{")
                else if (fault === "incomplete")
                    sessionStorage.setItem(
                        "ballotData",
                        JSON.stringify({electionId: "missing-ballot"})
                    )
                else sessionStorage.removeItem("ballotData")
            },
            {fault, now: portal.now}
        )
        await page.getByRole("button", {name: "Sign in", exact: true}).click()
        await expect(page.getByRole("alert")).toContainText(message)
        await expect(page.getByRole("button", {name: "Cast ballot", exact: true})).toHaveCount(0)
        expect(portal.oidc.authorizations.at(-1)?.requestedAcr).toBe("gold")
        expect(portal.graphql.callsTo("InsertCastVote")).toEqual([])
        expect(await page.evaluate(() => sessionStorage.getItem("ballotData"))).toBeNull()
    })
}
