// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {translateFromPresentation} from "@sequentech/ui-core"

export const parseMaybeJson = (value: unknown): unknown => {
    if (typeof value !== "string") {
        return value
    }

    try {
        return JSON.parse(value)
    } catch {
        return value
    }
}

export const translatedLabel = (
    row: Record<string, unknown> | undefined,
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
