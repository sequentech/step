// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/** The name a report's hash manifest is saved with. */
export const REPORT_MANIFEST_FILE_NAME = "report-manifest.json"

const isRecord = (value: unknown): value is Record<string, unknown> =>
    typeof value === "object" && value !== null && !Array.isArray(value)

/**
 * The document that stores the hash manifest of a generated report, from the
 * annotations of the report's own document. A report of an event that was not
 * imported from a signed configuration has none.
 */
export const reportManifestDocumentId = (annotations: unknown): string | undefined => {
    if (!isRecord(annotations) || !isRecord(annotations.report_manifest_file)) {
        return undefined
    }
    const documentId = annotations.report_manifest_file.document_id
    return typeof documentId === "string" && documentId.length > 0 ? documentId : undefined
}
