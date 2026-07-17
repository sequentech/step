// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {translateFromPresentation} from "@sequentech/ui-core"
import {ResultsJson, ResultsRow, ResultsSerializedJson} from "@/types/results"

const isResultsJson = (value: unknown): value is ResultsJson => {
    if (
        value === null ||
        typeof value === "string" ||
        typeof value === "number" ||
        typeof value === "boolean"
    ) {
        return true
    }

    if (Array.isArray(value)) {
        return value.every(isResultsJson)
    }

    return typeof value === "object" && Object.values(value).every(isResultsJson)
}

export const parseMaybeJson = (
    value: ResultsSerializedJson | undefined
): ResultsSerializedJson | undefined => {
    if (typeof value !== "string") {
        return value
    }

    try {
        const parsed: unknown = JSON.parse(value)
        return isResultsJson(parsed) ? parsed : value
    } catch {
        return value
    }
}

export const translatedLabel = (
    row: ResultsRow | undefined,
    locale: string,
    fallback = "-"
): string => {
    if (!row) {
        return fallback
    }

    const presentation = parseMaybeJson(row.presentation)
    const translated =
        translateFromPresentation({presentation, ...row}, "name", locale) ??
        translateFromPresentation(presentation, "name", locale) ??
        row.name

    return typeof translated === "string" && translated.length ? translated : fallback
}

export const manifestTitle = (
    title: string | Record<string, string> | undefined,
    locale: string,
    fallback: string
) => {
    if (typeof title === "string") {
        return title
    }

    return title?.[locale] ?? title?.en ?? fallback
}
