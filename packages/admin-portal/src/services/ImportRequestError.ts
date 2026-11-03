// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
/** Safe stage metadata; detailed backend validation stays in the typed report. */
export class ImportRequestError extends Error {
    constructor(readonly stage: "transport" | "validation" | "creation") {
        super(
            stage === "transport"
                ? "Backend import request failed."
                : stage === "validation"
                  ? "Backend dry-run validation refused the package."
                  : "Backend creation request failed."
        )
        this.name = "ImportRequestError"
    }
}
