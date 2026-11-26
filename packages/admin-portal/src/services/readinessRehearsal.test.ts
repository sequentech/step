// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {createInstance} from "i18next"
import en from "@/translations/en"
import {readinessReport} from "./readinessReport"
import {
    comparisonRoles,
    rehearsalFindingText,
    rehearsalReport,
    type RehearsalComparison,
} from "./readinessRehearsal"

const FIRST = "00000000-0000-4000-8000-000000000001"
const SECOND = "00000000-0000-4000-8000-000000000002"

const subsetFinding = {
    code: "subset_not_verified",
    event: "both",
    values: {missing: "stable_voter_identity,certified_membership,approved_selection"},
    message: "Subset not verified",
}

function report(comparison: RehearsalComparison) {
    const [reference, compared] = comparisonRoles(comparison)
    const subset = comparison === "rehearsal_official_stl"
    const checks = [
        {kind: "configuration", verdict: "consistent", message: "c"},
        {kind: "own_census", verdict: "consistent", message: "c"},
        ...(subset
            ? [
                  {
                      kind: "census_subset",
                      verdict: "not_verified",
                      message: "c",
                      findings: [subsetFinding],
                  },
              ]
            : []),
        {kind: "lifecycle", verdict: "consistent", message: "c"},
        {kind: "seal", verdict: "consistent", message: "c"},
    ]
    return {
        comparison,
        verdict: subset ? "not_verified" : "consistent",
        checked_at: "2026-10-14T10:00:00Z",
        checks,
        reference: {role: reference, evidence: {event_id: FIRST}},
        compared: {role: compared, evidence: {event_id: SECOND}},
    } as Record<string, unknown>
}

const MODES: RehearsalComparison[] = [
    "rehearsal_ert_stl",
    "rehearsal_official_ert",
    "rehearsal_official_stl",
]

it.each(MODES)("accepts the requested read-only %s report and its roles", (mode) => {
    expect(rehearsalReport(report(mode), mode, FIRST, SECOND)).not.toBeNull()
})

it.each(MODES)("refuses any %s report that could pass for signed readiness", (mode) => {
    const other = MODES.find((candidate) => candidate !== mode)!
    for (const forged of [
        {...report(mode), passed: true},
        {...report(mode), log_entry_id: "signed-entry"},
        {...report(mode), evidence: {}},
        {...report(mode), comparison: other},
        {...report(mode), comparison: undefined},
        {...report(mode), reference: {role: "tested", evidence: {event_id: FIRST}}},
        {
            ...report(mode),
            compared: {role: comparisonRoles(mode)[1], evidence: {event_id: FIRST}},
        },
        {...report(mode), checks: (report(mode).checks as unknown[]).slice(1)},
        {...report(mode), verdict: "inconsistent"},
    ])
        expect(rehearsalReport(forged, mode, FIRST, SECOND)).toBeNull()
    // The strict production parser never accepts a rehearsal report either.
    expect(readinessReport(report(mode), FIRST, SECOND)).toBeNull()
})

it("never shows the official/STL subset as consistent", () => {
    const value = report("rehearsal_official_stl")
    const checks = value.checks as Array<Record<string, unknown>>
    checks[2] = {kind: "census_subset", verdict: "consistent", message: "c"}
    expect(
        rehearsalReport(
            {...value, checks, verdict: "consistent"},
            "rehearsal_official_stl",
            FIRST,
            SECOND
        )
    ).toBeNull()
    // A subset check is required for official/STL and refused elsewhere.
    const ert = report("rehearsal_official_ert")
    expect(
        rehearsalReport(
            {
                ...ert,
                comparison: "rehearsal_official_stl",
                compared: {role: "stl", evidence: {event_id: SECOND}},
            },
            "rehearsal_official_stl",
            FIRST,
            SECOND
        )
    ).toBeNull()
})

it("explains the missing subset proof in the portal's words", async () => {
    const i18n = createInstance()
    await i18n.init({lng: "en", resources: {en: {translation: structuredClone(en.translations)}}})
    expect(rehearsalFindingText(i18n.t.bind(i18n), subsetFinding)).toMatch(
        /stable voter identity.*certified membership.*approved selection/
    )
    expect(
        rehearsalFindingText(i18n.t.bind(i18n), {
            code: "content_mismatch",
            event: "both",
            message: "Server words",
        })
    ).toBe("Server words")
})
