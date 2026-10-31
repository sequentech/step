// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {getTrustedWriteErrorMessage, rethrowTrustedWriteError} from "./trustedWriteError"

it.each([
    [
        "Initialization report state can only change through report generation",
        "generate the initialization report",
    ],
    [
        "Scheduled execution and prediction results can only be written by the scheduler",
        "Reload before saving",
    ],
    [
        "The voting status of election id can only change through the voting status actions",
        "Publish tab",
    ],
    [
        "The lockdown of election event id can only change through a scheduled lockdown event",
        "schedule a start or end",
    ],
    [
        "The grace period of election id can only change before voting starts",
        "cannot change after voting",
    ],
])("explains the trigger refusal: %s", (message, expected) => {
    const error = {
        body: {
            graphQLErrors: [
                {
                    message: "permission error",
                    extensions: {
                        internal: {error: {status_code: "42501", message}},
                    },
                },
            ],
        },
    }
    expect(getTrustedWriteErrorMessage(error)).toContain(expected)
    expect(() => rethrowTrustedWriteError(error)).toThrow(expected)
})

it("preserves unrelated permission and transport failures", () => {
    for (const error of [
        undefined,
        new Error("Forbidden"),
        {graphQLErrors: [{extensions: {code: "42501"}}]},
    ]) {
        expect(getTrustedWriteErrorMessage(error)).toBeUndefined()
        try {
            rethrowTrustedWriteError(error)
        } catch (caught) {
            expect(caught).toBe(error)
        }
    }
})
