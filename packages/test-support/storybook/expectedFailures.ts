// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {afterEach} from "vitest"

interface ExpectedFailure {
    reason: string
    a11y: string[]
}

// Storybook generates each Vitest test. Its task.fails flag has the same
// semantics as test.fails: an unexpected pass fails the run. Only the exact
// documented axe rules qualify; render errors and interaction failures do not.
afterEach((context) => {
    const story = (
        context as typeof context & {
            story?: {parameters: {expectedFailure?: ExpectedFailure}}
        }
    ).story
    const expected = story?.parameters.expectedFailure
    if (!expected) return

    const errors = context.task.result?.errors ?? []
    const rules = [
        ...new Set(
            errors.flatMap((error) =>
                [
                    ...error.message.matchAll(
                        /https:\/\/dequeuniversity.com\/rules\/axe\/[^/]+\/([^?\s]+)/g
                    ),
                ].map((match) => match[1])
            )
        ),
    ].sort()
    const matches =
        errors.every((error) => error.message.includes("toHaveNoViolations")) &&
        JSON.stringify(rules) === JSON.stringify([...expected.a11y].sort())
    Object.assign(context.task, {fails: errors.length === 0 || matches})
})
