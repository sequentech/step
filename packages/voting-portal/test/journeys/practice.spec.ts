// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {test, expect, eventPath, IDS} from "./fixtures"

for (const legacyName of [undefined, "Practice without legacy marker"]) {
    for (const completedPractice of [false, true]) {
        test(`TEST election gates the actual ballot until a practice vote exists: ${completedPractice}, legacy=${legacyName}`, async ({
            page,
            portal,
        }) => {
            const practiceElectionId = "30000000-0000-4000-8000-000000000002"
            const practiceStyleId = "50000000-0000-4000-8000-000000000002"
            const practiceElection = {
                ...portal.data.election,
                id: practiceElectionId,
                election_id: practiceElectionId,
                presentation: {
                    ...portal.data.election.presentation,
                    i18n: {en: {name: "TEST Practice", description: "Practice before voting"}},
                },
            }
            if (legacyName) Object.assign(practiceElection, {name: legacyName})
            portal.s3.putJson("private", "practice-election.json", practiceElection)
            portal.s3.putJson("private", "practice-summary.json", {
                ...portal.data.summary,
                id: practiceStyleId,
            })
            portal.graphql.on("GetVoterStatus", () => ({
                data: {
                    get_ballot_files_urls: {
                        event_id: IDS.event,
                        status: portal.data.event.status,
                        files: [
                            {
                                id: IDS.style,
                                election_id: IDS.election,
                                version: "1",
                                status: portal.data.election.status,
                                num_allowed_revotes: 0,
                                voting_channels: portal.data.election.voting_channels,
                                urls: {
                                    event_url: portal.s3.presign("event.json", "test"),
                                    election_url: portal.s3.presign("election.json", "test"),
                                    summary_url: portal.s3.presign("summary.json", "test"),
                                    style_url: portal.s3.presign("style.json", "test"),
                                },
                            },
                            {
                                id: practiceStyleId,
                                election_id: practiceElectionId,
                                version: "1",
                                status: practiceElection.status,
                                num_allowed_revotes: 0,
                                voting_channels: practiceElection.voting_channels,
                                urls: {
                                    event_url: portal.s3.presign("event.json", "test"),
                                    election_url: portal.s3.presign(
                                        "practice-election.json",
                                        "test"
                                    ),
                                    summary_url: portal.s3.presign("practice-summary.json", "test"),
                                    style_url: portal.s3.presign(
                                        "unused-practice-style.json",
                                        "test"
                                    ),
                                },
                            },
                        ],
                    },
                    sequent_backend_cast_vote: completedPractice
                        ? [
                              {
                                  id: "90000000-0000-4000-8000-000000000002",
                                  tenant_id: IDS.tenant,
                                  election_id: practiceElectionId,
                                  election_event_id: IDS.event,
                                  status: "valid",
                              },
                          ]
                        : [],
                },
            }))
            await page.goto(`${portal.origin}${eventPath}?lang=en`)
            const community = page
                .getByRole("heading", {name: "Community Council"})
                .locator("..")
                .locator("..")
            const practice = page
                .getByRole("heading", {name: "TEST Practice"})
                .locator("..")
                .locator("..")
            await expect(practice.getByRole("button", {name: /click to vote/i})).toBeEnabled()
            if (completedPractice)
                await expect(community.getByRole("button", {name: /click to vote/i})).toBeEnabled()
            else
                await expect(community.getByRole("button", {name: /click to vote/i})).toBeDisabled()
            expect(portal.graphql.callsTo("InsertCastVote")).toEqual([])
        })
    }
}
