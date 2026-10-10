// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {REPORT_MANIFEST_FILE_NAME, reportManifestDocumentId} from "./reportManifest"

describe("reportManifestDocumentId", () => {
    it("reads the hash manifest's document from the report's annotations", () => {
        expect(
            reportManifestDocumentId({
                access: {password_secret_id: "secret"},
                report_manifest: {report_type: "ACTIVITY_LOGS"},
                report_manifest_file: {document_id: "manifest-document", sha256: "ab"},
            })
        ).toBe("manifest-document")
    })

    it("finds none for a report generated without a signed configuration", () => {
        expect(reportManifestDocumentId({})).toBeUndefined()
        expect(reportManifestDocumentId({access: {password_secret_id: "secret"}})).toBeUndefined()
        expect(reportManifestDocumentId(null)).toBeUndefined()
        expect(reportManifestDocumentId(undefined)).toBeUndefined()
    })

    it("ignores annotations that are not shaped as a hash manifest link", () => {
        expect(reportManifestDocumentId("report_manifest_file")).toBeUndefined()
        expect(reportManifestDocumentId(["report_manifest_file"])).toBeUndefined()
        expect(reportManifestDocumentId({report_manifest_file: null})).toBeUndefined()
        expect(reportManifestDocumentId({report_manifest_file: "document"})).toBeUndefined()
        expect(reportManifestDocumentId({report_manifest_file: {sha256: "ab"}})).toBeUndefined()
        expect(reportManifestDocumentId({report_manifest_file: {document_id: 7}})).toBeUndefined()
        expect(reportManifestDocumentId({report_manifest_file: {document_id: ""}})).toBeUndefined()
    })

    it("saves it under the name the generation gave it", () => {
        expect(REPORT_MANIFEST_FILE_NAME).toBe("report-manifest.json")
    })
})
