// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
// @ts-check

/**
 * A story of the package's Storybook, as its Vitest project names it.
 * @typedef {object} StoryEntry
 * @property {string} id Story ID, for example `screens-admin-tally-ceremony--loading`.
 * @property {string} componentId ID of the story's title, shared by the stories of one file.
 * @property {string} name Display name, which is also the name of the story's Vitest test.
 * @property {string} exportName
 * @property {string} file Story file relative to the package directory, with `/` separators.
 */

/**
 * @typedef {object} StorySelection
 * @property {string[]} files Story files given to Vitest.
 * @property {StoryEntry[]} stories Stories that run.
 * @property {boolean} byName Whether Vitest filters the files' tests by story name.
 */

/**
 * @typedef {object} TestStoryArguments
 * @property {string | undefined} target Story file, story ID, title ID or Storybook URL.
 * @property {"run" | "watch"} mode
 * @property {boolean} help
 * @property {string[]} vitestOptions Other options, passed to Vitest unchanged.
 */

export class StorySelectionError extends Error {}

/**
 * @param {string[]} argv
 * @returns {TestStoryArguments}
 */
export function parseArguments(argv) {
    /** @type {string[]} */
    const targets = []
    /** @type {string[]} */
    const vitestOptions = []
    /** @type {"run" | "watch"} */
    let mode = "run"
    let help = false
    for (const argument of argv) {
        if (argument === "--watch" || argument === "-w") mode = "watch"
        else if (argument === "--help" || argument === "-h") help = true
        else if (argument.startsWith("-")) vitestOptions.push(argument)
        else targets.push(argument)
    }
    if (targets.length > 1) {
        throw new StorySelectionError(
            `Expected one story file or story ID, got: ${targets.join(" ")}. ` +
                "Pass Vitest options with values as --option=value."
        )
    }
    return {target: targets[0], mode, help, vitestOptions}
}

/**
 * Reads the story ID from a Storybook manager or iframe URL, and returns other
 * targets unchanged.
 * @param {string} target
 * @returns {string}
 */
export function storyIdFromTarget(target) {
    if (!/^https?:\/\//.test(target)) return target
    const url = new URL(target)
    const path = url.searchParams.get("path")
    const id = path?.match(/^\/(?:story|docs)\/([^/]+)$/)?.[1] ?? url.searchParams.get("id")
    if (!id) throw new StorySelectionError(`${target} does not name a story`)
    return id
}

/**
 * @param {string} target Story ID or title ID; a story file is selected with `file`.
 * @param {StoryEntry[]} entries
 * @param {string} [file] Package-relative path when `target` names an existing file.
 * @returns {StorySelection}
 */
export function selectStories(target, entries, file) {
    if (file) {
        const stories = entries.filter((entry) => entry.file === file)
        if (stories.length === 0) {
            throw new StorySelectionError(`${file} is not a story file of this Storybook`)
        }
        return {files: [file], stories, byName: false}
    }
    const id = storyIdFromTarget(target)
    const story = entries.find((entry) => entry.id === id)
    if (story) return {files: [story.file], stories: [story], byName: true}
    const component = entries.filter((entry) => entry.componentId === id)
    if (component.length > 0) {
        return {
            files: [...new Set(component.map((entry) => entry.file))],
            stories: component,
            byName: false,
        }
    }
    const similar = entries
        .filter((entry) => entry.id.includes(id.toLowerCase()))
        .slice(0, 5)
        .map((entry) => `\n  ${entry.id}`)
        .join("")
    throw new StorySelectionError(
        `No story file, story ID or title ID matches ${target}` +
            (similar ? `. Similar story IDs:${similar}` : "")
    )
}

/**
 * @param {string} text
 * @returns {string}
 */
export const escapeRegExp = (text) => text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")

/**
 * Matches the stories' tests. Storybook nests the tests of a story's `.test()`
 * cases in a suite named after the story followed by two spaces.
 * @param {string[]} names
 * @returns {string}
 */
export const testNamePattern = (names) => `^(?:${names.map(escapeRegExp).join("|")})(?: {3}.+)?$`

/**
 * @param {StorySelection} selection
 * @param {Pick<TestStoryArguments, "mode" | "vitestOptions">} options
 * @returns {string[]}
 */
export function vitestArguments(selection, {mode, vitestOptions}) {
    return [
        mode,
        "--project=storybook",
        ...selection.files,
        ...(selection.byName
            ? [`--testNamePattern=${testNamePattern(selection.stories.map(({name}) => name))}`]
            : []),
        ...vitestOptions,
    ]
}
