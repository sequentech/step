// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export type NestedTranslations = {[key: string]: string | NestedTranslations}

export const flattenTranslations = (
    value: NestedTranslations,
    prefix = ""
): Record<string, string> => {
    const result: Record<string, string> = {}
    Object.entries(value).forEach(([key, nested]) => {
        const path = prefix ? `${prefix}.${key}` : key
        if (typeof nested === "string") {
            result[path] = nested
        } else if (nested && typeof nested === "object") {
            Object.assign(result, flattenTranslations(nested, path))
        }
    })
    return result
}

export const nestTranslation = (key: string, value: string): NestedTranslations => {
    const keys = key.split(".")
    const root: NestedTranslations = {}
    keys.reduce((acc, part, index) => {
        if (index === keys.length - 1) {
            acc[part] = value
            return acc
        }
        const next = (acc[part] as NestedTranslations) || {}
        acc[part] = next
        return next
    }, root)
    return root
}

export const stripHtml = (value: string): string =>
    value
        .replace(/<[^>]+>/g, " ")
        .replace(/&nbsp;/g, " ")
        .replace(/\s+/g, " ")
        .trim()

export const normalizeText = (value: string): string => stripHtml(value).toLowerCase()

export const visibleText = (value: string): string =>
    stripHtml(value)
        .replace(/\u2060[\u200B\u200C]+\u2060/g, "")
        .replace(/\{\{[^}]+\}\}/g, "…")
        .trim()

const SEGMENT_LABELS: Record<string, string> = {
    electionSelectionScreen: "Ballot list",
    selectElection: "Ballot card",
    startScreen: "How to vote",
    votingScreen: "Ballot",
    reviewScreen: "Review",
    confirmationScreen: "Confirmation",
    auditScreen: "Audit",
    ballotLocator: "Ballot finder",
    breadcrumbSteps: "Steps",
    header: "Header",
    footer: "Footer",
    logout: "Logout",
    candidate: "Candidate",
    candidatesList: "Candidate list",
    materials: "Support materials",
    errors: "Errors",
    common: "Common",
    version: "Version",
    hash: "Hash",
    ballotHash: "Ballot ID",
    language: "Language",
    chooserHelpDialog: "Help dialog",
    demoDialog: "Demo dialog",
    ballotHelpDialog: "Help dialog",
    nonVotedDialog: "Invalid vote dialog",
    warningDialog: "Review dialog",
    confirmCastVoteDialog: "Confirm cast",
    auditBallotHelpDialog: "Audit help",
    ballotIdHelpDialog: "Ballot ID help",
    reviewScreenHelpDialog: "Help dialog",
    confirmationHelpDialog: "Help dialog",
    demoPrintDialog: "Demo print",
    demoBallotUrlDialog: "Demo tracker",
    ballotIdDemoHelpDialog: "Demo ballot ID help",
    step1HelpDialog: "Download help",
    step2HelpDialog: "Verifier help",
    titleHelpDialog: "Help dialog",
    implicit: "Ballot warnings",
    explicit: "Invalid vote",
    encoding: "Write-in errors",
    page: "Error page",
    session: "Session",
    modal: "Dialog",
    alerts: "Alerts",
    preferential: "Preferential",
}

const humanizeSegment = (segment: string): string => {
    if (SEGMENT_LABELS[segment]) {
        return SEGMENT_LABELS[segment]
    }
    return segment
        .replace(/_/g, " ")
        .replace(/([a-z])([A-Z])/g, "$1 $2")
        .replace(/\b\w/g, (char) => char.toUpperCase())
        .trim()
}

export const keyGroup = (key: string): string => humanizeSegment(key.split(".")[0] || key)

export const humanizeKey = (key: string): string => key.split(".").map(humanizeSegment).join(" · ")

const escapeRegex = (value: string): string => value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")

const hasInterpolation = (template: string): boolean => template.includes("{{")

const isTransTemplate = (template: string): boolean =>
    /<\d+\s*\/?>|<[A-Za-z][\w]*\s*\/?>/.test(template)

const isLabelPrefix = (template: string): boolean => /:\s*$/.test(stripHtml(template))

export const scoreKeyAgainstText = (
    elementText: string,
    key: string,
    template: string,
    preferredKeys: Set<string>
): number => {
    const haystack = normalizeText(elementText)
    if (!haystack) {
        return 0
    }
    const needle = normalizeText(template)
    if (!needle) {
        return 0
    }

    let score = 0
    if (!hasInterpolation(template) && haystack === needle) {
        score = 10000 + needle.length
    } else if (hasInterpolation(template)) {
        const source = escapeRegex(normalizeText(template)).replace(/\\{\\{[^}]+\\}\\}/g, ".+")
        if (new RegExp(`^${source}$`).test(haystack)) {
            score = 8000 + needle.length
        }
    } else if (
        isLabelPrefix(template) &&
        haystack.startsWith(needle) &&
        haystack.length > needle.length
    ) {
        score = 6000 + needle.length
    } else if (isTransTemplate(template) && haystack.startsWith(needle) && needle.length >= 8) {
        score = 5500 + needle.length
    }

    if (score === 0) {
        return 0
    }
    if (preferredKeys.has(key)) {
        score += 2000
    }
    score += key.split(".").length * 10
    return score
}

export const findBestKeyForText = (
    elementText: string,
    flattened: Record<string, string>,
    preferredKeys: string[]
): string | null => {
    const preferred = new Set(preferredKeys)
    let bestKey: string | null = null
    let bestScore = 0
    const keys =
        preferredKeys.length > 0
            ? [...new Set([...preferredKeys, ...Object.keys(flattened)])]
            : Object.keys(flattened)

    keys.forEach((key) => {
        const template = flattened[key]
        if (typeof template !== "string") {
            return
        }
        const score = scoreKeyAgainstText(elementText, key, template, preferred)
        if (score > bestScore) {
            bestKey = key
            bestScore = score
        }
    })

    return bestKey
}
