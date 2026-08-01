// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {EExportFormat} from "@/types/results"
import {getExportDocumentId} from "./exportDocuments"

describe("getExportDocumentId", () => {
    it("prefers the archive containing generated PDFs", () => {
        expect(
            getExportDocumentId(
                {tar_gz: "original", tar_gz_pdfs: "with-pdfs"},
                EExportFormat.TAR_GZ
            )
        ).toBe("with-pdfs")
    })

    it("keeps TAR_GZ available when only the PDF archive exists", () => {
        expect(getExportDocumentId({tar_gz_pdfs: "with-pdfs"}, EExportFormat.TAR_GZ)).toBe(
            "with-pdfs"
        )
    })

    it("returns the requested non-archive document", () => {
        expect(getExportDocumentId({json: "json-id"}, EExportFormat.JSON)).toBe("json-id")
        expect(getExportDocumentId({}, EExportFormat.JSON)).toBeUndefined()
    })
})
