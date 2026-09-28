// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {test, expect} from "@playwright/test"
import {
    parseArguments,
    selectStories,
    storyIdFromTarget,
    testNamePattern,
    vitestArguments,
    type StoryEntry,
} from "../../test-support/storybook/storySelection.mjs"

const TALLY = "src/resources/Tally/TallyCeremony.stories.tsx"
const KEYS = "src/resources/ElectionEvent/EditElectionEventKeys.stories.tsx"
const entries: StoryEntry[] = [
    {
        id: "screens-admin-tally-ceremony--loading",
        componentId: "screens-admin-tally-ceremony",
        name: "Loading",
        exportName: "Loading",
        file: TALLY,
    },
    {
        id: "screens-admin-tally-ceremony--loading-failure",
        componentId: "screens-admin-tally-ceremony",
        name: "Loading Failure",
        exportName: "LoadingFailure",
        file: TALLY,
    },
    {
        id: "screens-admin-keys-ceremony--loading",
        componentId: "screens-admin-keys-ceremony",
        name: "Loading",
        exportName: "Loading",
        file: KEYS,
    },
]

test.describe("focused story test arguments", () => {
    test("reads the target, watch mode and Vitest options", () => {
        expect(parseArguments(["story-id"])).toEqual({
            target: "story-id",
            mode: "run",
            help: false,
            vitestOptions: [],
        })
        expect(parseArguments(["--watch", "story-id", "--reporter=dot"])).toEqual({
            target: "story-id",
            mode: "watch",
            help: false,
            vitestOptions: ["--reporter=dot"],
        })
        expect(parseArguments(["-w", "-h"])).toEqual({
            target: undefined,
            mode: "watch",
            help: true,
            vitestOptions: [],
        })
    })

    test("rejects a second target, such as an option value given apart", () => {
        expect(() => parseArguments(["story-id", "--reporter", "dot"])).toThrow(
            "Expected one story file or story ID, got: story-id dot. " +
                "Pass Vitest options with values as --option=value."
        )
    })

    test("reads story IDs from manager, docs and iframe URLs", () => {
        const id = "screens-admin-tally-ceremony--loading"
        expect(storyIdFromTarget(`http://localhost:6008/?path=/story/${id}`)).toBe(id)
        expect(
            storyIdFromTarget(`http://localhost:6008/?path=/story/${id}&globals=locale:es`)
        ).toBe(id)
        expect(storyIdFromTarget("http://localhost:6008/?path=/docs/screens-admin--docs")).toBe(
            "screens-admin--docs"
        )
        expect(
            storyIdFromTarget(`http://127.0.0.1:43008/iframe.html?id=${id}&viewMode=story`)
        ).toBe(id)
        expect(storyIdFromTarget(id)).toBe(id)
        expect(() => storyIdFromTarget("http://localhost:6008/?path=/settings/about")).toThrow(
            "http://localhost:6008/?path=/settings/about does not name a story"
        )
    })
})

test.describe("focused story selection", () => {
    test("runs every story of a story file without a name filter", () => {
        expect(selectStories(TALLY, entries, TALLY)).toEqual({
            files: [TALLY],
            stories: [entries[0], entries[1]],
            byName: false,
        })
        expect(() => selectStories("src/App.tsx", entries, "src/App.tsx")).toThrow(
            "src/App.tsx is not a story file of this Storybook"
        )
    })

    test("runs one story by ID and all stories of a title by title ID", () => {
        expect(selectStories("screens-admin-keys-ceremony--loading", entries)).toEqual({
            files: [KEYS],
            stories: [entries[2]],
            byName: true,
        })
        expect(selectStories("screens-admin-tally-ceremony", entries)).toEqual({
            files: [TALLY],
            stories: [entries[0], entries[1]],
            byName: false,
        })
    })

    test("suggests similar story IDs for an unknown target", () => {
        expect(() => selectStories("tally-ceremony--load", entries)).toThrow(
            "No story file, story ID or title ID matches tally-ceremony--load. Similar story IDs:" +
                "\n  screens-admin-tally-ceremony--loading" +
                "\n  screens-admin-tally-ceremony--loading-failure"
        )
        expect(() => selectStories("results", entries)).toThrow(
            /^No story file, story ID or title ID matches results$/
        )
    })
})

test.describe("focused story Vitest filter", () => {
    test("matches the story test and its nested cases, not longer names", () => {
        const pattern = new RegExp(testNamePattern(["Loading"]))
        expect(pattern.test("Loading")).toBe(true)
        // Storybook names a story's .test() cases "<story>  " + " " + "<case>".
        expect(pattern.test("Loading   base story")).toBe(true)
        expect(pattern.test("Loading Failure")).toBe(false)
        expect(pattern.test("Slow Loading")).toBe(false)
    })

    test("escapes regular expression characters in story names", () => {
        expect(testNamePattern(["Results (2+ contests) $5?"])).toBe(
            "^(?:Results \\(2\\+ contests\\) \\$5\\?)(?: {3}.+)?$"
        )
    })

    test("builds the Vitest command line", () => {
        expect(
            vitestArguments(
                {files: [KEYS], stories: [entries[2]], byName: true},
                {mode: "watch", vitestOptions: ["--coverage"]}
            )
        ).toEqual([
            "watch",
            "--project=storybook",
            KEYS,
            "--testNamePattern=^(?:Loading)(?: {3}.+)?$",
            "--coverage",
        ])
        expect(
            vitestArguments(
                {files: [TALLY], stories: [entries[0], entries[1]], byName: false},
                {mode: "run", vitestOptions: []}
            )
        ).toEqual(["run", "--project=storybook", TALLY])
    })
})
