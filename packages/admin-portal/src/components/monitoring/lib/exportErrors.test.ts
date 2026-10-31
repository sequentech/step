// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {ApolloError} from "@apollo/client"
import {GraphQLError} from "graphql"
import type {MonitoringWidget} from "../types"
import {exportFailure} from "./exportErrors"

const refusal = (extensions: Record<string, unknown>, message = "refused") =>
    new ApolloError({graphQLErrors: [new GraphQLError(message, {extensions})]})

// Shows the key and its values, so the test reads what the viewer is told.
const t = (key: string, values?: Record<string, unknown>) =>
    values ? `${key} ${JSON.stringify(values)}` : key

const widgets: MonitoringWidget[] = [
    {
        id: "turnout-by-group",
        title: "Turnout by group",
        source: "voter_turnout",
        selectors: {breakdown: {label: "Breakdown", options: {sex: "Sex"}}},
        chart: {},
    },
]

const invalid = (...problems: Array<{code: string; path: string; message: string}>) =>
    refusal({code: "MONITORING_INVALID", problems}, "The configuration is not valid.")

describe("exportFailure", () => {
    it("tells an end before the start as the dialog's own range message", () => {
        const error = invalid({
            code: "invalid_value",
            path: "to",
            message: "The end of the range must come after its start.",
        })
        expect(exportFailure(error, t, widgets)).toBe("monitoring.export.invalidRange")
    })

    it("names the widget and selector of a pick Harvest does not know", () => {
        const error = invalid(
            {
                code: "unknown_selector",
                path: "widget_selector_values.turnout-by-group.grain",
                message: "'turnout-by-group' has no selector 'grain'.",
            },
            {
                code: "unknown_option",
                path: "widget_selector_values.turnout-by-group.breakdown",
                message: "'region' is not an option of 'breakdown'.",
            }
        )
        expect(exportFailure(error, t, widgets)).toBe(
            'monitoring.export.problems.unknownSelector {"widget":"Turnout by group","selector":"grain"} ' +
                'monitoring.export.problems.unknownOption {"widget":"Turnout by group","selector":"Breakdown"}'
        )
    })

    it("names an unknown widget by its id", () => {
        const error = invalid({
            code: "unknown_selector",
            path: "widget_selector_values.gone.day",
            message: "'gone' has no selector 'day'.",
        })
        expect(exportFailure(error, t, [])).toBe(
            'monitoring.export.problems.unknownSelector {"widget":"gone","selector":"day"}'
        )
    })

    it("falls back to the invalid message and Harvest's words for other problems", () => {
        const error = invalid({code: "invalid_value", path: "format", message: "No such format."})
        expect(exportFailure(error, t, widgets)).toBe("monitoring.errors.invalid No such format.")
        expect(exportFailure(invalid(), t, widgets)).toBe("monitoring.errors.invalid")
    })

    it("tells a bad request, such as a bound without an offset, with the common message", () => {
        const error = refusal(
            {code: "MONITORING_BAD_REQUEST"},
            "'from' must be an RFC 3339 instant with an offset."
        )
        expect(exportFailure(error, t, widgets)).toBe("monitoring.errors.badRequest")
    })

    it("tells other refusals with their own messages", () => {
        expect(exportFailure(refusal({code: "MONITORING_SNAPSHOT_PRUNED"}), t, widgets)).toBe(
            "monitoring.errors.snapshotPruned"
        )
        expect(exportFailure(new Error("down"), t, widgets)).toBe("monitoring.errors.unknown")
    })
})
