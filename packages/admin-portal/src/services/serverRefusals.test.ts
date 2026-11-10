// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ApolloError} from "@apollo/client"
import {GraphQLError} from "graphql"
import type {TFunction} from "i18next"
import {
    INVALID_DOCUMENT_NAME_ERROR_CODE,
    PHASE_CHANGING_KEY,
    documentNameRefusal,
    isInvalidDocumentNameError,
    phaseRefusalKey,
    secondTallyMessage,
    secondTallyRefusal,
} from "./serverRefusals"

const t = ((key: string) => key) as unknown as TFunction

/** An action refusal as Apollo hands it to a form: Hasura's message and code. */
const refusal = (message: string, code: string) =>
    new ApolloError({graphQLErrors: [new GraphQLError(message, {extensions: {code}})]})

const NAME_TEXT =
    "A document name cannot contain / \\ ; # % ? or a control character. Rename it and try again."
const WAIT_KEY = "refusals.documentWaitsForPostProcessing"

describe("documentNameRefusal", () => {
    it("translates the upload route's refusal of a name", () => {
        const error = refusal(NAME_TEXT, INVALID_DOCUMENT_NAME_ERROR_CODE)
        expect(isInvalidDocumentNameError(error)).toBe(true)
        expect(documentNameRefusal(t, error)).toBe("refusals.documentName")
    })

    it("recognizes the code without the text and the database's text without the code", () => {
        expect(documentNameRefusal(t, refusal("refused", INVALID_DOCUMENT_NAME_ERROR_CODE))).toBe(
            "refusals.documentName"
        )
        const databaseText =
            "Check constraint violation. A document name cannot contain \\ ; # % ? or a control character. Rename it and try again."
        expect(documentNameRefusal(t, refusal(databaseText, "constraint-violation"))).toBe(
            "refusals.documentName"
        )
    })

    it("leaves any other failure to the form's own message", () => {
        expect(documentNameRefusal(t, refusal("permission denied", "permission-error"))).toBe(
            undefined
        )
        expect(documentNameRefusal(t, new ApolloError({networkError: new Error("offline")}))).toBe(
            undefined
        )
        expect(documentNameRefusal(t, new Error("File upload failed"))).toBe(undefined)
        expect(documentNameRefusal(t, undefined)).toBe(undefined)
    })
})

describe("phaseRefusalKey", () => {
    it.each(["Preparation", "Execution", "Evaluation"])(
        "gives the caller's key for the refusal in %s",
        (phase) => {
            const text = `This operation requires electoral phase PostProcessing; current phase is ${phase}`
            expect(phaseRefusalKey(refusal(text, "InternalServerError"), WAIT_KEY)).toBe(WAIT_KEY)
            // Hooks that keep only the message of the error.
            expect(phaseRefusalKey(text, WAIT_KEY)).toBe(WAIT_KEY)
        }
    )

    it("gives the common key while the phase or safety state is changing", () => {
        const text = "Election safety state is changing; retry the operation"
        expect(phaseRefusalKey(refusal(text, "InternalServerError"), WAIT_KEY)).toBe(
            PHASE_CHANGING_KEY
        )
        expect(phaseRefusalKey(text, WAIT_KEY)).toBe(PHASE_CHANGING_KEY)
    })

    it("finds the text in a body Hasura could not parse", () => {
        const error = {
            graphQLErrors: [
                {
                    message: "not a valid json response from webhook",
                    extensions: {
                        code: "unexpected",
                        internal: {
                            response: {
                                body: "This operation requires electoral phase PostProcessing; current phase is Evaluation",
                            },
                        },
                    },
                },
            ],
        }
        expect(phaseRefusalKey(error, WAIT_KEY)).toBe(WAIT_KEY)
    })

    it("ignores other phase requirements and other failures", () => {
        const otherPhase =
            "This operation requires electoral phase Execution; current phase is Preparation"
        expect(phaseRefusalKey(refusal(otherPhase, "InternalServerError"), WAIT_KEY)).toBe(
            undefined
        )
        expect(phaseRefusalKey(refusal("Document not found", "not-found"), WAIT_KEY)).toBe(
            undefined
        )
        expect(phaseRefusalKey("Failed to fetch database from URL", WAIT_KEY)).toBe(undefined)
        expect(phaseRefusalKey(null, WAIT_KEY)).toBe(undefined)
        expect(phaseRefusalKey(undefined, WAIT_KEY)).toBe(undefined)
    })
})

describe("secondTallyRefusal", () => {
    const ELECTION = "8d1f6f0e-3a8b-4c7e-9a52-0b6f4a1e2c3d"

    it("names the counted election", () => {
        const text = `Election ${ELECTION}: its results are already counted, and a recount is not available.`
        expect(secondTallyRefusal(refusal(text, "TallyValidation"))).toEqual({
            key: "refusals.resultsAlreadyCounted",
            electionId: ELECTION,
        })
    })

    it("names the election whose results tally has not finished", () => {
        const text = `Election ${ELECTION}: a results tally of it has not finished. Wait until it finishes, or cancel it.`
        expect(secondTallyRefusal(refusal(text, "TallyValidation"))).toEqual({
            key: "refusals.resultsTallyUnfinished",
            electionId: ELECTION,
        })
    })

    it("leaves other refusals of a tally to the screen's own message", () => {
        expect(
            secondTallyRefusal(
                refusal(`Election ${ELECTION} is not part of this keys ceremony`, "TallyValidation")
            )
        ).toBe(undefined)
        expect(secondTallyRefusal(new ApolloError({networkError: new Error("offline")}))).toBe(
            undefined
        )
        expect(secondTallyRefusal(undefined)).toBe(undefined)
    })
})

describe("secondTallyMessage", () => {
    const ELECTION = "8d1f6f0e-3a8b-4c7e-9a52-0b6f4a1e2c3d"
    const counted = refusal(
        `Election ${ELECTION}: its results are already counted, and a recount is not available.`,
        "TallyValidation"
    )
    const interpolate = ((key: string, options?: {election?: string}) =>
        `${key}: ${options?.election}`) as unknown as TFunction

    it("names the election as the admin knows it", () => {
        const names = jest.fn((id: string) => (id === ELECTION ? "Mayor 2026" : undefined))
        expect(secondTallyMessage(interpolate, counted, names)).toBe(
            "refusals.resultsAlreadyCounted: Mayor 2026"
        )
        expect(names).toHaveBeenCalledWith(ELECTION)
    })

    it("falls back to the election's ID", () => {
        expect(secondTallyMessage(interpolate, counted, () => undefined)).toBe(
            `refusals.resultsAlreadyCounted: ${ELECTION}`
        )
    })

    it("leaves any other failure to the screen's own message", () => {
        const names = jest.fn()
        expect(
            secondTallyMessage(interpolate, refusal("Permission denied", "TallyValidation"), names)
        ).toBe(undefined)
        expect(names).not.toHaveBeenCalled()
    })
})
