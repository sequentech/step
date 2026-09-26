// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
// @ts-check

// Lists the package's authored UI components with the Storybook sections and
// stories that render them, and checks that every widget has one.
import {resolve} from "node:path"
import {pathToFileURL} from "node:url"
import {inventory} from "./widgetInventory.mjs"

const USAGE = `Usage: yarn stories:inventory [--check] [--json | --markdown] [--url=<Storybook URL>] [<source path | component name> ...]

Without a filter, prints the widget counts per feature and the widgets without
a story. With filters, prints each matching widget's section, story IDs, direct
links and focused test command. --markdown prints the complete source-to-story
table; --json prints every entry. --check fails when a widget has no story, a
section is misnamed or a story names a component its module does not define.`

/** @param {string[]} argv */
function parseArguments(argv) {
    /** @type {{check: boolean, format: "text" | "json" | "markdown", url?: string, help: boolean, filters: string[]}} */
    const options = {check: false, format: "text", help: false, filters: []}
    for (const argument of argv) {
        if (argument === "--check") options.check = true
        else if (argument === "--json") options.format = "json"
        else if (argument === "--markdown") options.format = "markdown"
        else if (argument.startsWith("--url=")) options.url = argument.slice("--url=".length)
        else if (argument === "--help" || argument === "-h") options.help = true
        else if (argument.startsWith("-")) throw new Error(`Unknown option ${argument}\n${USAGE}`)
        else options.filters.push(argument.replace(/^\.\//, ""))
    }
    return options
}

/**
 * @param {import("./widgetInventory.mjs").WidgetEntry} widget
 * @param {string} filter
 */
const matches = (widget, filter) =>
    widget.component.file === filter ||
    widget.component.file.startsWith(filter.endsWith("/") ? filter : `${filter}/`) ||
    widget.component.name === filter ||
    widget.component.local === filter ||
    widget.feature.toLowerCase() === filter.toLowerCase()

/** @param {import("./widgetInventory.mjs").WidgetEntry} widget */
const storiesOf = (widget) => [
    ...widget.sections.flatMap((section) => section.stories),
    ...widget.stories,
]

async function main() {
    const options = parseArguments(process.argv.slice(2))
    if (options.help) {
        console.log(USAGE)
        return
    }
    const packageDir = process.cwd()
    const configPath = resolve(packageDir, ".storybook/widgets.mjs")
    /** @type {import("./widgetInventory.mjs").WidgetConfig} */
    const config = (await import(pathToFileURL(configPath).href)).default
    const url = options.url ?? config.storybookUrl
    const {widgets, problems} = inventory(packageDir, config)
    const selected = options.filters.length
        ? widgets.filter((widget) => options.filters.some((filter) => matches(widget, filter)))
        : widgets
    const counted = selected.filter((widget) => widget.status !== "excluded")
    const covered = counted.filter((widget) => widget.status === "covered")
    const remaining = counted.filter((widget) => widget.status === "remaining")

    if (options.format === "json") {
        console.log(
            JSON.stringify(
                {
                    storybookUrl: url,
                    counts: {
                        components: selected.length,
                        excluded: selected.length - counted.length,
                        widgets: counted.length,
                        covered: covered.length,
                        remaining: remaining.length,
                    },
                    widgets: selected.map((widget) => ({
                        file: widget.component.file,
                        name: widget.component.name,
                        local: widget.component.local,
                        exported: widget.component.exportNames.length > 0,
                        kind: widget.component.kind,
                        line: widget.component.line,
                        feature: widget.feature,
                        status: widget.status,
                        reason: widget.reason,
                        sections: widget.sections.map(({title, id, file}) => ({title, id, file})),
                        stories: storiesOf(widget).map(({id}) => id),
                    })),
                    problems,
                },
                null,
                2
            )
        )
    } else if (options.format === "markdown") {
        console.log("| Feature | Source | Widget | Section | Stories |")
        console.log("| --- | --- | --- | --- | ---: |")
        for (const widget of counted) {
            const section = widget.sections[0]
            const embedding = widget.stories[0]
            const link = section
                ? `[${section.title}](${url}/?path=/docs/${section.id}--docs)`
                : embedding
                  ? `in [${embedding.id}](${url}/?path=/story/${embedding.id})`
                  : "**none**"
            console.log(
                `| ${widget.feature} | \`${widget.component.file}\` | ${widget.component.name} | ` +
                    `${link} | ${storiesOf(widget).length} |`
            )
        }
    } else if (options.filters.length) {
        for (const widget of selected) {
            const {file, name, line} = widget.component
            console.log(`${file}:${line} ${name} (${widget.status})`)
            if (widget.reason) console.log(`  ${widget.reason}`)
            for (const section of widget.sections) {
                console.log(`  ${section.title}  ${url}/?path=/docs/${section.id}--docs`)
                console.log(`  yarn test:story ${section.file}`)
            }
            for (const story of storiesOf(widget)) {
                console.log(`    ${story.id}  ${url}/?path=/story/${story.id}`)
            }
            if (widget.status === "remaining") console.log(`  expected: ${widget.expectedTitle}`)
        }
    } else {
        /** @type {Map<string, {widgets: number, covered: number, stories: number}>} */
        const features = new Map()
        for (const widget of counted) {
            const entry = features.get(widget.feature) ?? {widgets: 0, covered: 0, stories: 0}
            entry.widgets += 1
            if (widget.status === "covered") entry.covered += 1
            entry.stories += storiesOf(widget).length
            features.set(widget.feature, entry)
        }
        const excluded = selected.filter((widget) => widget.status === "excluded")
        const styled = excluded.filter((widget) => widget.component.kind === "styled").length
        console.log(
            `${selected.length} components: ${counted.length} widgets, ` +
                `${excluded.length} excluded (${styled} styled primitives, ` +
                `${excluded.length - styled} listed in .storybook/widgets.mjs)`
        )
        console.log(`${covered.length} covered, ${remaining.length} remaining\n`)
        const width = Math.max(...[...features.keys()].map((feature) => feature.length), 7)
        console.log(`${"Feature".padEnd(width)}  Widgets  Covered  Stories`)
        for (const [feature, entry] of [...features].sort(([a], [b]) => a.localeCompare(b))) {
            console.log(
                `${feature.padEnd(width)}  ${String(entry.widgets).padStart(7)}  ` +
                    `${String(entry.covered).padStart(7)}  ${String(entry.stories).padStart(7)}`
            )
        }
        if (remaining.length) {
            console.log("\nWidgets without a story:")
            for (const widget of remaining) {
                console.log(
                    `  ${widget.component.file}#${widget.component.name}  ${widget.expectedTitle}`
                )
            }
        }
    }
    if (problems.length && options.format !== "json") {
        console.error(`\n${problems.length} problems:`)
        for (const problem of problems) console.error(`  ${problem}`)
    }
    if (options.check && (problems.length || remaining.length)) process.exitCode = 1
}

main().catch((error) => {
    console.error(error instanceof Error ? error.message : error)
    process.exitCode = 1
})
