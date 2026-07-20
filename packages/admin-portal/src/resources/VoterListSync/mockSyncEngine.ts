// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * File parsing/serialization here follows the format described in
 * DatafixPossibleImplementation.md ("Reconciliation File Format" and "Patch
 * Files Format") and is real, reusable logic.
 *
 * Everything under the "MOCK" banner below simulates the backend (S3 upload,
 * the Celery diff-calculation/patch-generation/apply tasks) so the wizards
 * can be exercised end to end without a server. Replace those functions with
 * the real GraphQL mutations/queries when the reconciliation backend lands -
 * the wizard components only depend on the exported function signatures, not
 * on how the result is produced.
 */

import {
    ESyncChangeCategory,
    ESyncPatchTarget,
    ParsedReconciliationRow,
    PatchField,
    PatchFieldValue,
    PATCH_FIELDS,
    RowFailure,
    SyncDiffRow,
    SyncFileMeta,
    SyncTaskResult,
    SyncVoterRecord,
} from "./types"
import {MOCK_ELECTION_EVENT_COUNTY_MUN} from "./constants"

const FIELD_LABELS: Record<PatchField, string> = {
    CountyMun: "County/Municipality",
    DoB: "Date of Birth",
    Ward: "Ward",
    Poll: "Poll",
    SchoolSupportCode: "School Support Code",
    Channel: "Voting Channel",
    Deleted: "Deleted",
}

const splitLines = (text: string): string[] =>
    text.split(/\r?\n/).filter((line) => line.trim().length > 0)

const parseMetaLine = (line: string): SyncFileMeta => {
    const fields = line
        .trim()
        .replace(/^#META,?/, "")
        .split(",")
    let sequence = 0
    let generatedAt = 0
    fields.forEach((field) => {
        const [key, value] = field.split("=")
        if (key === "Sequence") {
            sequence = Number.parseInt(value, 10) || 0
        }
        if (key === "GeneratedAt") {
            generatedAt = Number.parseInt(value, 10) || 0
        }
    })
    return {sequence, generatedAt}
}

export const formatGeneratedAt = (unixSeconds: number): string => {
    if (!unixSeconds) {
        return "-"
    }
    return new Date(unixSeconds * 1000).toLocaleString()
}

export const validateSequence = (
    meta: SyncFileMeta,
    lastAppliedSequence: number
): {valid: boolean; message?: string} => {
    if (meta.sequence <= lastAppliedSequence) {
        return {
            valid: false,
            message: `This file's Sequence (${meta.sequence}) is not newer than the last applied Sequence (${lastAppliedSequence}). Stale file, rejected - see "Stale-file protection" in DatafixPossibleImplementation.md.`,
        }
    }
    return {valid: true}
}

// ---------------------------------------------------------------------
// Real parsing: reconciliation file (CountyMun,VoterID,DoB,Ward,Poll,
// SchoolSupportCode,Channel,Deleted)
// ---------------------------------------------------------------------
export const parseReconciliationFile = (
    text: string
): {meta: SyncFileMeta; rows: ParsedReconciliationRow[]} => {
    const lines = splitLines(text)
    const meta = parseMetaLine(lines[0] ?? "")
    const rows = lines.slice(2).map((line) => {
        const [countyMun, voterId, dob, ward, poll, schoolSupportCode, channel, deleted] = line
            .split(",")
            .map((value) => value.trim())
        return {
            countyMun,
            voterId,
            dob,
            ward,
            poll,
            schoolSupportCode,
            channel,
            deleted: deleted?.toLowerCase() === "true",
        }
    })
    return {meta, rows}
}

// ---------------------------------------------------------------------
// Real parsing: patch file (VoterID, then <Field>_old/<Field>_new pairs)
// ---------------------------------------------------------------------
const categorizePatchField = (
    field: string,
    oldValue: string,
    newValue: string
): ESyncChangeCategory => {
    if (field === "Channel") {
        if (oldValue !== "INTERNET" && newValue === "INTERNET") {
            return ESyncChangeCategory.VOTED_INTERNET
        }
        if (newValue !== "NONE" && newValue !== "INTERNET") {
            return ESyncChangeCategory.VOTED_OTHER_CHANNEL
        }
    }
    if (field === "Deleted") {
        return newValue === "true"
            ? ESyncChangeCategory.DISABLED
            : ESyncChangeCategory.DELETION_REVERTED
    }
    return ESyncChangeCategory.PROFILE_UPDATE
}

export const parsePatchFile = (text: string): {meta: SyncFileMeta; rows: SyncDiffRow[]} => {
    const lines = splitLines(text)
    const meta = parseMetaLine(lines[0] ?? "")
    const header = (lines[1] ?? "").split(",").map((value) => value.trim())
    const fieldNames = header.slice(1, header.length).filter((_, index) => index % 2 === 0)
    const cleanedFieldNames = fieldNames.map((name) => name.replace(/_old$/, ""))

    const rows: SyncDiffRow[] = []
    lines.slice(2).forEach((line, lineIndex) => {
        const values = line.split(",").map((value) => value.trim())
        const voterId = values[0]
        const oldValues = cleanedFieldNames.map((_, fieldIndex) => values[1 + fieldIndex * 2])
        const isAddedVoter = oldValues.every((value) => value === "NONE")

        cleanedFieldNames.forEach((field, fieldIndex) => {
            const oldValue = values[1 + fieldIndex * 2]
            const newValue = values[2 + fieldIndex * 2]
            if (oldValue === newValue) {
                return
            }
            rows.push({
                id: `${voterId}:${field}:${lineIndex}`,
                voterId,
                field,
                label: FIELD_LABELS[field as PatchField] ?? field,
                oldValue,
                newValue,
                category: isAddedVoter
                    ? ESyncChangeCategory.VOTER_ADDED
                    : categorizePatchField(field, oldValue, newValue),
                target: "sequent",
            })
        })
    })

    return {meta, rows}
}

// Flattens a voter record into display rows, one per field that actually
// changed (old !== new) - e.g. an added voter's Channel can legitimately be
// NONE on both sides (hasn't voted yet), which isn't a field worth showing
// in the diff table even though the row itself is included in the patch.
let mockDiffRowIdCounter = 0
export const recordToDiffRows = (record: SyncVoterRecord): SyncDiffRow[] =>
    PATCH_FIELDS.filter(
        (field) => record.fields[field].oldValue !== record.fields[field].newValue
    ).map((field) => ({
        id: `${record.voterId}:${field}:${mockDiffRowIdCounter++}`,
        voterId: record.voterId,
        field,
        label: FIELD_LABELS[field],
        oldValue: record.fields[field].oldValue,
        newValue: record.fields[field].newValue,
        category: record.category,
        target: record.target,
    }))

// ---------------------------------------------------------------------
// CSV serialization for the download buttons. Every PATCH_FIELDS column is
// always written, in the same fixed order, for every voter that has at
// least one change - matching sequent_patch.csv/datafix_patch.csv. VoterID
// is the row key, never a column pair, and never changes.
// ---------------------------------------------------------------------
export const buildPatchCsv = (
    meta: SyncFileMeta,
    records: SyncVoterRecord[],
    target: ESyncPatchTarget
): string => {
    const targeted = records.filter((record) => record.target === target)

    const lines = [`#META,Sequence=${meta.sequence},GeneratedAt=${meta.generatedAt}`]
    lines.push(
        ["VoterID", ...PATCH_FIELDS.flatMap((field) => [`${field}_old`, `${field}_new`])].join(",")
    )
    targeted.forEach((record) => {
        const values = PATCH_FIELDS.flatMap((field) => [
            record.fields[field].oldValue,
            record.fields[field].newValue,
        ])
        lines.push([record.voterId, ...values].join(","))
    })

    return lines.join("\n")
}

export const buildRowFailuresCsv = (failures: RowFailure[]): string => {
    const lines = [
        "VoterID,Reason",
        ...failures.map((failure) => `${failure.voterId},"${failure.reason}"`),
    ]
    return lines.join("\n")
}

export const downloadTextFile = (filename: string, content: string): void => {
    const blob = new Blob([content], {type: "text/csv;charset=utf-8"})
    const url = URL.createObjectURL(blob)
    const anchor = document.createElement("a")
    anchor.href = url
    anchor.download = filename
    anchor.click()
    URL.revokeObjectURL(url)
}

// =======================================================================
// MOCK backend interaction below - replace when the reconciliation backend
// (S3 upload, diff-calculation task, generate/apply patch tasks) lands.
// =======================================================================

const delay = (ms: number): Promise<void> => new Promise((resolve) => setTimeout(resolve, ms))

// MOCK: replace with the existing GET_UPLOAD_URL mutation + PUT-to-S3 flow
// used by ImportScreen.tsx.
export const mockUploadToS3 = async (file: File): Promise<{documentId: string; sha256: string}> => {
    await delay(600)
    return {
        documentId: `mock-document-${Date.now()}`,
        sha256: `mock-sha256-${file.name}-${file.size}`,
    }
}

let mockFailureRowIdCounter = 0
const makeFailureRow = (
    voterId: string,
    field: PatchField,
    oldValue: string,
    newValue: string,
    failureReason: string
): SyncDiffRow => ({
    id: `${voterId}:${field}:failure:${mockFailureRowIdCounter++}`,
    voterId,
    field,
    label: FIELD_LABELS[field],
    oldValue,
    newValue,
    category: ESyncChangeCategory.ROW_FAILURE,
    target: "sequent",
    failureReason,
})

// A record's fields all start equal to the voter's current snapshot (no
// change); callers then override the one or two fields they want to demo.
const baselineFields = (row: ParsedReconciliationRow): Record<PatchField, PatchFieldValue> => ({
    CountyMun: {oldValue: row.countyMun, newValue: row.countyMun},
    DoB: {oldValue: row.dob, newValue: row.dob},
    Ward: {oldValue: row.ward, newValue: row.ward},
    Poll: {oldValue: row.poll, newValue: row.poll},
    SchoolSupportCode: {oldValue: row.schoolSupportCode, newValue: row.schoolSupportCode},
    Channel: {oldValue: row.channel, newValue: row.channel},
    Deleted: {oldValue: String(row.deleted), newValue: String(row.deleted)},
})

const pickSample = <T>(rows: T[], count: number): T[] => {
    if (rows.length <= count) {
        return rows
    }
    const step = Math.floor(rows.length / count)
    return Array.from({length: count}, (_, index) => rows[index * step])
}

const bumpWard = (ward: string): string => {
    const numeric = Number.parseInt(ward, 10)
    return Number.isNaN(numeric) ? `${ward}-A` : String(numeric + 1).padStart(ward.length, "0")
}

// MOCK: replace with a GraphQL query that reads the diff the backend Celery
// task already computed on upload (see "Patches creation flow" step 2). This
// fabricates a plausible, deterministic diff from the uploaded rows so every
// category described in "Handling Inconsistencies" shows up at least once,
// regardless of which reconciliation file gets dropped. Each scenario below
// builds a full SyncVoterRecord (every PATCH_FIELDS column, matching the
// voter's real snapshot except for the one field being demoed) so the
// generated patch CSV always has every column, not just the changed one.
//
// `round` simulates the operator loop in "Operator flow (per synchronization)":
// round 0 is the first import and always has Datafix-side changes (so there's
// something to generate a Datafix patch for); round 1 simulates the next
// reconciliation file coming back clean on the Datafix side (only Sequent-side
// changes remain, so the Sequent patch can be generated); round 2+ simulates
// full convergence (empty diff). The caller (GeneratePatchesWizard) advances
// this on every new import within the session.
export const mockCalculateReconciliationDiff = async (
    rows: ParsedReconciliationRow[],
    round: number
): Promise<{diffRows: SyncDiffRow[]; records: SyncVoterRecord[]}> => {
    await delay(900)

    if (rows.length === 0 || round >= 2) {
        return {diffRows: [], records: []}
    }

    const records: SyncVoterRecord[] = []
    const failureDiffRows: SyncDiffRow[] = []
    const [votedInternet, votedOther, profileUpdate, disabled, reverted, added] = pickSample(
        rows,
        6
    )
    const datafixSideClean = round >= 1

    // A) Sequent already holds a valid internet ballot; Datafix says NONE.
    // Only shown before the Datafix side has caught up (round 0).
    if (votedInternet && !datafixSideClean) {
        const fields = baselineFields(votedInternet)
        fields.Channel = {oldValue: votedInternet.channel, newValue: "INTERNET"}
        records.push({
            voterId: votedInternet.voterId,
            target: "datafix",
            category: ESyncChangeCategory.VOTED_INTERNET,
            fields,
        })
    }

    // B) Datafix reports a paper vote Sequent doesn't have.
    if (votedOther) {
        const fields = baselineFields(votedOther)
        fields.Channel = {oldValue: "NONE", newValue: "PAPER"}
        records.push({
            voterId: votedOther.voterId,
            target: "sequent",
            category: ESyncChangeCategory.VOTED_OTHER_CHANNEL,
            fields,
        })
    }

    // C) A voter-profile field changed on the Datafix side.
    if (profileUpdate) {
        const fields = baselineFields(profileUpdate)
        fields.Ward = {oldValue: profileUpdate.ward, newValue: bumpWard(profileUpdate.ward)}
        records.push({
            voterId: profileUpdate.voterId,
            target: "sequent",
            category: ESyncChangeCategory.PROFILE_UPDATE,
            fields,
        })
    }

    // C) Voter marked Deleted=true and hasn't voted -> disable in Sequent.
    if (disabled) {
        const fields = baselineFields(disabled)
        fields.Deleted = {oldValue: String(disabled.deleted), newValue: "true"}
        records.push({
            voterId: disabled.voterId,
            target: "sequent",
            category: ESyncChangeCategory.DISABLED,
            fields,
        })
    }

    // C exception) Voter marked Deleted=true but has voted -> deletion reverted.
    // Only shown before the Datafix side has caught up (round 0).
    if (reverted && !datafixSideClean) {
        const fields = baselineFields(reverted)
        fields.Deleted = {oldValue: "true", newValue: "false"}
        records.push({
            voterId: reverted.voterId,
            target: "datafix",
            category: ESyncChangeCategory.DELETION_REVERTED,
            fields,
        })
    }

    // D) Voter present in the file, unknown to Sequent -> added, all _old
    // values NONE per the "Patch Files Format" spec.
    if (added) {
        const snapshot = baselineFields(added)
        const fields = PATCH_FIELDS.reduce(
            (acc, field) => {
                acc[field] = {oldValue: "NONE", newValue: snapshot[field].newValue}
                return acc
            },
            {} as Record<PatchField, PatchFieldValue>
        )
        records.push({
            voterId: added.voterId,
            target: "sequent",
            category: ESyncChangeCategory.VOTER_ADDED,
            fields,
        })
    }

    // D) Voter Sequent has that the file doesn't -> reported to Datafix.
    // Synthesized: by definition it can't come from the uploaded rows. Only
    // shown before the Datafix side has caught up (round 0).
    if (!datafixSideClean) {
        const fields: Record<PatchField, PatchFieldValue> = {
            CountyMun: {oldValue: "NONE", newValue: MOCK_ELECTION_EVENT_COUNTY_MUN},
            DoB: {oldValue: "NONE", newValue: "1990-01-01"},
            Ward: {oldValue: "NONE", newValue: "01"},
            Poll: {oldValue: "NONE", newValue: "000"},
            SchoolSupportCode: {oldValue: "NONE", newValue: "P"},
            Channel: {oldValue: "NONE", newValue: "NONE"},
            Deleted: {oldValue: "NONE", newValue: "false"},
        }
        records.push({
            voterId: `SEQ-${rows[0].voterId}`,
            target: "datafix",
            category: ESyncChangeCategory.VOTER_ADDED,
            fields,
        })
    }

    // CountyMun processing error -> row failure, excluded from both patches
    // (so it's reported separately, not as a SyncVoterRecord). Only shown on
    // round 0 - treated as fixed by Datafix from then on, same as the other
    // Datafix-side items above.
    if (!datafixSideClean) {
        const failureReason =
            "CountyMun does not match this election event and the row is not marked Deleted - Datafix processing error."
        const badCountyMun = rows.find(
            (row) => row.countyMun !== MOCK_ELECTION_EVENT_COUNTY_MUN && !row.deleted
        )
        const failureSource = badCountyMun ?? rows[Math.min(6, rows.length - 1)]
        failureDiffRows.push(
            makeFailureRow(
                failureSource.voterId,
                "CountyMun",
                badCountyMun ? badCountyMun.countyMun : "0099",
                MOCK_ELECTION_EVENT_COUNTY_MUN,
                failureReason
            )
        )
    }

    const diffRows = [...records.flatMap(recordToDiffRows), ...failureDiffRows]
    return {diffRows, records}
}

export interface MockTaskStep {
    log: string
    delayMs: number
}

export const MOCK_GENERATE_PATCH_STEPS: MockTaskStep[] = [
    {log: "Task started", delayMs: 250},
    {log: "Validating reconciliation snapshot against current voter data...", delayMs: 700},
    {log: "Resolving inconsistencies per source-of-truth rules...", delayMs: 700},
    {log: "Writing patch file to S3...", delayMs: 500},
]

export const MOCK_APPLY_PATCH_STEPS: MockTaskStep[] = [
    {log: "Task started", delayMs: 250},
    {log: "Validating patch Sequence against the current reconciliation round...", delayMs: 600},
    {log: "Applying changes voter by voter (per-row atomic)...", delayMs: 900},
    {log: "Writing electoral log entry...", delayMs: 450},
]

// MOCK: replace with polling the real task_execution (see Widget.tsx /
// WidgetsContextProvider) once the Celery tasks from ETasksExecution.
// GENERATE_RECONCILIATION_PATCHES / APPLY_RECONCILIATION_PATCH exist.
export const runMockTask = async (
    steps: MockTaskStep[],
    diffRows: SyncDiffRow[],
    onLog: (log: string) => void
): Promise<SyncTaskResult> => {
    for (const step of steps) {
        await delay(step.delayMs)
        onLog(step.log)
    }

    const rowFailures: RowFailure[] = diffRows
        .filter((row) => row.category === ESyncChangeCategory.ROW_FAILURE)
        .map((row) => ({voterId: row.voterId, reason: row.failureReason ?? "Row failure"}))

    return {rowFailures}
}
