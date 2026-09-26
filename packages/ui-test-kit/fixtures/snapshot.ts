// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

/** The only snapshot schema version this module reads and writes. */
export const SNAPSHOT_VERSION = 1

/** Stable scenario identifiers, used in workbench deep links and Storybook story IDs. */
export enum ScenarioId {
    SIMPLE_PLURALITY = "simple-plurality",
    RANKED_MULTI_CONTEST = "ranked-multi-contest",
    KIOSK_VOTER = "kiosk-voter",
}

export const isScenarioId = (value: unknown): value is ScenarioId =>
    Object.values(ScenarioId).includes(value as ScenarioId)

/** How the voter reaches the portal; the values match the election `voting_channels` keys. */
export enum ScenarioChannel {
    ONLINE = "online",
    KIOSK = "kiosk",
}

export enum SnapshotOrigin {
    /** Built from a scenario definition. */
    BUNDLED = "bundled",
    /** Written by the workbench, with any local changes applied to the document. */
    EXPORTED = "exported",
}

export type JsonValue = string | number | boolean | null | JsonValue[] | JsonObject
export interface JsonObject {
    [key: string]: JsonValue
}

export interface PreviewCandidate extends JsonObject {
    id: string
}

export interface PreviewContest extends JsonObject {
    id: string
    min_votes: number
    max_votes: number
    candidates: PreviewCandidate[]
}

export interface PreviewBallotStyle extends JsonObject {
    id: string
    election_id: string
    election_event_id: string
    area_id: string
    contests: PreviewContest[]
}

export interface PreviewElection extends JsonObject {
    id: string
}

export interface PreviewElectionEvent extends JsonObject {
    id: string
}

/** A publication preview document, as the voting portal's preview route loads it. */
export interface PreviewDocument extends JsonObject {
    ballot_styles: PreviewBallotStyle[]
    elections: PreviewElection[]
    election_event: PreviewElectionEvent
    support_materials: JsonObject[]
    documents: JsonObject[]
}

export interface SnapshotProvenance {
    origin: SnapshotOrigin
    /** ISO 8601 time at which the snapshot was written. */
    createdAt: string
    /** Human-readable local changes applied on top of the scenario document. */
    changes: string[]
}

export interface ScenarioSnapshot {
    version: typeof SNAPSHOT_VERSION
    scenarioId: ScenarioId
    provenance: SnapshotProvenance
    tenantId: string
    areaId: string
    channel: ScenarioChannel
    preview: PreviewDocument
}

/** Rejected snapshot input; `issues` names each invalid field by its JSON path. */
export class SnapshotError extends Error {
    readonly issues: string[]

    constructor(issues: string[]) {
        super(`Invalid workbench snapshot:\n- ${issues.join("\n- ")}`)
        this.name = "SnapshotError"
        this.issues = issues
    }
}

const isObject = (value: unknown): value is JsonObject =>
    typeof value === "object" && value !== null && !Array.isArray(value)

const isNonEmptyString = (value: unknown): value is string =>
    typeof value === "string" && value.length > 0

const describe = (value: unknown) => (value === undefined ? "nothing" : JSON.stringify(value))

const isEnumValue = <T extends string>(values: Record<string, T>, value: unknown): value is T =>
    Object.values(values).includes(value as T)

function checkIds(issues: string[], path: string, value: JsonObject, keys: string[]) {
    for (const key of keys)
        if (!isNonEmptyString(value[key]))
            issues.push(
                `${path}.${key}: expected a non-empty string, found ${describe(value[key])}`
            )
}

function checkArray(issues: string[], path: string, value: unknown): value is JsonValue[] {
    if (Array.isArray(value)) return true
    issues.push(`${path}: expected an array, found ${describe(value)}`)
    return false
}

function checkContest(issues: string[], path: string, contest: JsonValue) {
    if (!isObject(contest)) {
        issues.push(`${path}: expected an object`)
        return
    }
    checkIds(issues, path, contest, ["id"])
    for (const key of ["min_votes", "max_votes"])
        if (!Number.isInteger(contest[key]) || (contest[key] as number) < 0)
            issues.push(
                `${path}.${key}: expected a non-negative integer, found ${describe(contest[key])}`
            )
    if (checkArray(issues, `${path}.candidates`, contest.candidates))
        contest.candidates.forEach((candidate, index) => {
            if (isObject(candidate))
                checkIds(issues, `${path}.candidates[${index}]`, candidate, ["id"])
            else issues.push(`${path}.candidates[${index}]: expected an object`)
        })
}

function checkPreview(issues: string[], preview: unknown, areaId: unknown) {
    if (!isObject(preview)) {
        issues.push(`preview: expected a publication preview object, found ${describe(preview)}`)
        return
    }
    const event = preview.election_event
    if (isObject(event)) checkIds(issues, "preview.election_event", event, ["id"])
    else issues.push(`preview.election_event: expected an object, found ${describe(event)}`)

    const electionIds = new Set<string>()
    if (checkArray(issues, "preview.elections", preview.elections))
        preview.elections.forEach((election, index) => {
            const path = `preview.elections[${index}]`
            if (!isObject(election)) issues.push(`${path}: expected an object`)
            else if (isNonEmptyString(election.id)) electionIds.add(election.id)
            else checkIds(issues, path, election, ["id"])
        })
    for (const key of ["support_materials", "documents"])
        checkArray(issues, `preview.${key}`, preview[key])

    if (!checkArray(issues, "preview.ballot_styles", preview.ballot_styles)) return
    preview.ballot_styles.forEach((style, index) => {
        const path = `preview.ballot_styles[${index}]`
        if (!isObject(style)) {
            issues.push(`${path}: expected an object`)
            return
        }
        checkIds(issues, path, style, ["id", "election_id", "election_event_id", "area_id"])
        // The preview loader shows a style only when its election is published beside it.
        if (style.area_id === areaId && isNonEmptyString(style.election_id)) {
            if (!electionIds.has(style.election_id))
                issues.push(
                    `${path}.election_id: election ${style.election_id} is not in preview.elections`
                )
            if (isObject(event) && style.election_event_id !== event.id)
                issues.push(
                    `${path}.election_event_id: expected the preview event ${describe(event.id)}, found ${describe(style.election_event_id)}`
                )
        }
        if (checkArray(issues, `${path}.contests`, style.contests))
            style.contests.forEach((contest, contestIndex) =>
                checkContest(issues, `${path}.contests[${contestIndex}]`, contest)
            )
    })
}

/** Checks the envelope and the parts of the preview document the portal loader relies on. */
export function validateSnapshot(value: unknown): ScenarioSnapshot {
    if (!isObject(value))
        throw new SnapshotError([`snapshot: expected an object, found ${describe(value)}`])
    // Other versions may be shaped differently, so their fields are not inspected.
    if (value.version !== SNAPSHOT_VERSION)
        throw new SnapshotError([
            `version: expected ${SNAPSHOT_VERSION}, found ${describe(value.version)}`,
        ])
    const issues: string[] = []
    if (!isScenarioId(value.scenarioId))
        issues.push(
            `scenarioId: expected one of ${Object.values(ScenarioId).join(", ")}, found ${describe(value.scenarioId)}`
        )
    const provenance = value.provenance
    if (!isObject(provenance))
        issues.push(`provenance: expected an object, found ${describe(provenance)}`)
    else {
        if (!isEnumValue(SnapshotOrigin, provenance.origin))
            issues.push(
                `provenance.origin: expected one of ${Object.values(SnapshotOrigin).join(", ")}, found ${describe(provenance.origin)}`
            )
        if (
            !isNonEmptyString(provenance.createdAt) ||
            Number.isNaN(Date.parse(provenance.createdAt))
        )
            issues.push(
                `provenance.createdAt: expected an ISO 8601 time, found ${describe(provenance.createdAt)}`
            )
        if (
            checkArray(issues, "provenance.changes", provenance.changes) &&
            !provenance.changes.every((change) => typeof change === "string")
        )
            issues.push("provenance.changes: expected strings")
    }
    checkIds(issues, "snapshot", value, ["tenantId", "areaId"])
    if (!isEnumValue(ScenarioChannel, value.channel))
        issues.push(
            `channel: expected one of ${Object.values(ScenarioChannel).join(", ")}, found ${describe(value.channel)}`
        )
    checkPreview(issues, value.preview, value.areaId)
    if (issues.length) throw new SnapshotError(issues)
    return value as unknown as ScenarioSnapshot
}

export function parseSnapshot(text: string): ScenarioSnapshot {
    let value: unknown
    try {
        value = JSON.parse(text)
    } catch (error) {
        throw new SnapshotError([`snapshot: not valid JSON (${(error as Error).message})`])
    }
    return validateSnapshot(value)
}

export const serializeSnapshot = (snapshot: ScenarioSnapshot): string =>
    `${JSON.stringify(snapshot, null, 2)}\n`
