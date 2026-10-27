// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {CombinedGraphQLErrors} from "@apollo/client/errors"
import {TFunction} from "i18next"
import {castVoteErrorMessage} from "./CastVoteErrors"

const quota = "InsertFailedExceedsAllowedRevotes"
const t = ((key: string) => key) as TFunction
const error = (extensions: Record<string, unknown>) =>
    new CombinedGraphQLErrors({errors: [{message: "Cannot cast", extensions}]})

it("retains the direct quota error translation", () => {
    expect(castVoteErrorMessage(error({code: quota}), t)).toBe(
        `reviewScreen.error.CAST_VOTE_${quota}`
    )
})
it.each([
    {code: quota},
    {code: "unexpected", internal: {response: {body: {extensions: {code: quota}}}}},
])("keeps an explicit quota code when the wrapper says internal error", (extensions) => {
    const wrapped = new CombinedGraphQLErrors({errors: [{message: "internal error", extensions}]})
    expect(castVoteErrorMessage(wrapped, t)).toBe(`reviewScreen.error.CAST_VOTE_${quota}`)
})
it("retains the generic internal error when no recognized quota code is present", () => {
    expect(castVoteErrorMessage(new Error("internal error"), t)).toBe(
        "reviewScreen.error.INTERNAL_ERROR"
    )
})
it.each([
    {extensions: {code: quota}},
    JSON.stringify({message: "private server details", extensions: {code: quota}}),
])("recognizes the Hasura-wrapped quota response without displaying its message", (body) => {
    expect(
        castVoteErrorMessage(
            error({code: "unexpected", internal: {response: {body, status: 400}}}),
            t
        )
    ).toBe(`reviewScreen.error.CAST_VOTE_${quota}`)
})
it.each([
    null,
    "{",
    "null",
    "[]",
    {extensions: {code: 42}},
    {extensions: {code: "untrusted message"}},
])("keeps malformed or unknown wrapped errors generic: %s", (body) => {
    expect(castVoteErrorMessage(error({code: "unexpected", internal: {response: {body}}}), t)).toBe(
        "reviewScreen.error.CAST_VOTE_unexpected"
    )
})
it("does not let an internal body replace a distinct direct error", () => {
    expect(
        castVoteErrorMessage(
            error({
                code: "AreaNotFound",
                internal: {response: {body: {extensions: {code: quota}}}},
            }),
            t
        )
    ).toBe("reviewScreen.error.CAST_VOTE_AreaNotFound")
})
it("preserves genuine upstream timeout handling", () => {
    expect(
        castVoteErrorMessage(
            error({code: "unexpected", internal: {error: {message: "Response timeout"}}}),
            t
        )
    ).toBe("reviewScreen.error.CAST_VOTE_timeout")
})
