// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {
    MonitoringDashboard,
    MonitoringSelector,
    MonitoringWidget,
    MonitoringWidgetEntry,
} from "../types"
import type {ParsedWidgetEntry} from "./layout"

// The server sends definitions it has validated, as JSON. They are still
// checked for the fields the view reads, so a definition from a newer or older
// server shows as one invalid widget instead of breaking the dashboard.

export type Parsed<T> = {ok: true; value: T} | {ok: false; problem: string}

class ShapeError extends Error {}

type Json = Record<string, unknown>

const isObject = (value: unknown): value is Json =>
    typeof value === "object" && value !== null && !Array.isArray(value)

function object(value: unknown, path: string): Json {
    if (!isObject(value)) throw new ShapeError(`${path || "definition"} is not an object`)
    return value
}

function text(value: unknown, path: string): string {
    if (typeof value !== "string") throw new ShapeError(`${path} is not text`)
    return value
}

function optional<T>(value: unknown, read: (value: unknown) => T): T | undefined {
    return value === undefined || value === null ? undefined : read(value)
}

function texts(value: unknown, path: string): string[] {
    if (!Array.isArray(value)) throw new ShapeError(`${path} is not a list`)
    return value.map((item, index) => text(item, `${path}[${index}]`))
}

function textMap(value: unknown, path: string): Record<string, string> {
    return Object.fromEntries(
        Object.entries(object(value, path)).map(([key, item]) => [
            key,
            text(item, `${path}.${key}`),
        ])
    )
}

function positive(value: unknown, path: string): number {
    if (typeof value !== "number" || !Number.isFinite(value) || value <= 0) {
        throw new ShapeError(`${path} is not a positive number`)
    }
    return value
}

function decode(raw: unknown): unknown {
    if (typeof raw !== "string") return raw
    try {
        return JSON.parse(raw)
    } catch {
        throw new ShapeError("definition is not JSON")
    }
}

function attempt<T>(read: () => T): Parsed<T> {
    try {
        return {ok: true, value: read()}
    } catch (error) {
        if (error instanceof ShapeError) return {ok: false, problem: error.message}
        throw error
    }
}

function selector(raw: unknown, path: string): MonitoringSelector {
    const value = object(raw, path)
    text(value.label, `${path}.label`)
    optional(value.options, (options) => textMap(options, `${path}.options`))
    optional(value.default, (fallback) => text(fallback, `${path}.default`))
    optional(value.when, (when) => {
        const condition = object(when, `${path}.when`)
        text(condition.selector, `${path}.when.selector`)
        texts(condition.in, `${path}.when.in`)
    })
    return value as unknown as MonitoringSelector
}

export function parseDashboard(raw: unknown): Parsed<MonitoringDashboard> {
    return attempt(() => {
        const value = object(decode(raw), "")
        text(value.id, "id")
        text(value.title, "title")
        optional(value.description, (description) => text(description, "description"))
        optional(value.section, (section) => text(section, "section"))
        optional(value.requirements, (requirements) => texts(requirements, "requirements"))
        optional(value.selectors, (selectors) => texts(selectors, "selectors"))
        if (!Array.isArray(value.layout)) throw new ShapeError("layout is not a list")
        value.layout.forEach((item, index) => {
            const path = `layout[${index}]`
            const cell = object(item, path)
            text(cell.widget, `${path}.widget`)
            positive(cell.width, `${path}.width`)
            optional(cell.values, (values) => textMap(values, `${path}.values`))
        })
        return value as unknown as MonitoringDashboard
    })
}

export function parseWidget(raw: unknown): Parsed<MonitoringWidget> {
    return attempt(() => {
        const value = object(decode(raw), "")
        text(value.id, "id")
        text(value.title, "title")
        optional(value.description, (description) => text(description, "description"))
        text(value.source, "source")
        optional(value.requirements, (requirements) => texts(requirements, "requirements"))
        optional(value.follows, (follows) => texts(follows, "follows"))
        optional(value.height, (height) => positive(height, "height"))
        optional(value.selectors, (selectors) =>
            Object.entries(object(selectors, "selectors")).forEach(([name, item]) =>
                selector(item, `selectors.${name}`)
            )
        )
        return value as unknown as MonitoringWidget
    })
}

export function parseWidgets(
    entries: Record<string, MonitoringWidgetEntry> | null | undefined
): Record<string, ParsedWidgetEntry> {
    return Object.fromEntries(
        Object.entries(entries ?? {}).map(([id, entry]) => {
            const parsed = parseWidget(entry?.definition)
            const revision = entry?.revision ?? 0
            return [
                id,
                parsed.ok
                    ? {widget: parsed.value, revision}
                    : {widget: null, revision, problem: parsed.problem},
            ]
        })
    )
}
