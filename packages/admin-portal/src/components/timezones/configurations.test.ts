// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The overseas fixture the timezone stories and tests use is the janitor's
// COMELEC preset, the configuration the 104-Post replay runs under
// (`windmill/tests/postgres_post_replay.rs`).
import {readFileSync} from "fs"
import {join} from "path"
// The package Jest configuration maps ui-core to VotingChannel; these fixtures
// need the real lifecycle enums from ElectionEventPresentation.
jest.mock("@sequentech/ui-core", () =>
    jest.requireActual("../../../../ui-core/src/types/ElectionEventPresentation")
)

import {OVERSEAS_ZONES, overseasConfiguration} from "./__fixtures__/configurations"

interface IPreset {
    timezones: {configured: Array<string>; primary: string; logs: string}
    lifecycle_policies: {initialization_scope: string; unsigned_scheduled_close: string}
    posts: Array<{post: string; timezone: string}>
}

const preset = (): IPreset =>
    JSON.parse(
        readFileSync(
            join(
                __dirname,
                "../../../../windmill/external-bin/janitor/templates/COMELEC/lifecycle.json"
            ),
            "utf8"
        )
    )

describe("the overseas fixture", () => {
    it("configures the preset's zones, primary, log zone and policies", () => {
        const {timezones, lifecycle_policies} = preset()
        const fixture = overseasConfiguration().presentation
        expect([...OVERSEAS_ZONES].sort()).toEqual([...timezones.configured].sort())
        expect(fixture.timezones?.configured).toEqual(OVERSEAS_ZONES)
        expect(fixture.timezones?.primary).toBe(timezones.primary)
        expect(fixture.timezones?.logs).toBe(timezones.logs)
        expect(fixture.lifecycle_policies).toEqual(lifecycle_policies)
    })

    it("gives each Post the preset's zone", () => {
        const zones = new Map(
            preset().posts.map(({post, timezone}) => [post.toUpperCase(), timezone])
        )
        for (const post of overseasConfiguration().elections) {
            expect([post.name, post.timezone]).toEqual([
                post.name,
                zones.get(post.name.toUpperCase()),
            ])
        }
    })
})
