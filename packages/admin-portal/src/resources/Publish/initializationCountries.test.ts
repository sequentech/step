// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {
    EInitializationScope as Scope,
    EInitializeReportPolicy as ReportPolicy,
} from "@sequentech/ui-core"
import type {ILifecycleSnapshotEntry} from "@/queries/Lifecycle"
import {
    initializesPerCountry,
    effectiveInitializationReportPolicy,
    initializationCountries,
    initializationAreaIds,
} from "./initializationCountries"

jest.mock("@sequentech/ui-core", () => ({
    ...jest.requireActual("../../../../ui-core/src/types/ElectionEventPresentation"),
    ...jest.requireActual("../../../../ui-core/src/types/ElectionPresentation"),
}))

const snapshot = (election: string | null, scope: Scope): ILifecycleSnapshotEntry => ({
    election_id: election,
    publication_id: "publication",
    published_at: "2026-10-03T00:00:00Z",
    approval_request_id: null,
    approval_code: null,
    signed: false,
    snapshot: {policies: {initialization_scope: scope}},
})

it("keeps the country requirement from either current or newest applicable published copy", () => {
    expect(initializesPerCountry(Scope.POST_AND_COUNTRY, [], "post")).toBe(true)
    expect(
        initializesPerCountry(Scope.POST, [snapshot(null, Scope.POST_AND_COUNTRY)], "post")
    ).toBe(true)
    expect(
        initializesPerCountry(
            Scope.EVENT,
            [snapshot("other", Scope.POST_AND_COUNTRY), snapshot("post", Scope.POST)],
            "post"
        )
    ).toBe(false)
    expect(
        initializesPerCountry(
            undefined,
            [snapshot("post", Scope.POST), snapshot(null, Scope.POST_AND_COUNTRY)],
            "post"
        )
    ).toBe(false)
    expect(initializesPerCountry(undefined, [], "post")).toBe(false)
})

it("offers only active styled countries holding this Post's contests, directly or through ancestors", () => {
    const countries = initializationCountries(
        {
            sequent_backend_area: [
                {id: "group"},
                {id: "north", parent_id: "group"},
                {id: "south"},
                {id: "foreign"},
                {id: "unstyled", parent_id: "group"},
            ],
            sequent_backend_area_contest: [
                {area_id: "group", contest: {election_id: "post"}},
                {area_id: "south", contest: {election_id: "post"}},
                {area_id: "foreign", contest: {election_id: "other"}},
            ],
            sequent_backend_ballot_style: [
                {area_id: "north"},
                {area_id: "south"},
                {area_id: "foreign"},
                {area_id: "north"},
            ],
        },
        "post"
    )
    expect(countries.map((area) => area.id)).toEqual(["north", "south"])
    expect(initializationAreaIds("north", countries)).toEqual(["north"])
    expect(initializationAreaIds("", countries)).toBeUndefined()
    expect(() => initializationAreaIds("foreign", countries)).toThrow("not eligible")
    expect(() => initializationAreaIds("", [])).toThrow("No eligible countries")
})

it("does not accept countries with missing or cyclic area ancestry", () => {
    expect(
        initializationCountries(
            {
                sequent_backend_area: [
                    {id: "broken", parent_id: "missing"},
                    {id: "cycle", parent_id: "cycle"},
                ],
                sequent_backend_area_contest: [
                    {area_id: "broken", contest: {election_id: "post"}},
                    {area_id: "cycle", contest: {election_id: "post"}},
                ],
                sequent_backend_ballot_style: [{area_id: "broken"}, {area_id: "cycle"}],
            },
            "post"
        )
    ).toEqual([])
})

it("retains a published required report until a new applicable publication loosens it", () => {
    const kept = snapshot("post", Scope.POST)
    kept.snapshot.initialization_report_policies = {post: ReportPolicy.REQUIRED}
    expect(effectiveInitializationReportPolicy(ReportPolicy.NOT_REQUIRED, [kept], "post")).toBe(
        ReportPolicy.REQUIRED
    )
    const newer = {
        ...kept,
        snapshot: {
            ...kept.snapshot,
            initialization_report_policies: {post: ReportPolicy.NOT_REQUIRED},
        },
    }
    expect(
        effectiveInitializationReportPolicy(ReportPolicy.NOT_REQUIRED, [newer, kept], "post")
    ).toBe(ReportPolicy.NOT_REQUIRED)
    expect(effectiveInitializationReportPolicy(ReportPolicy.REQUIRED, [newer], "post")).toBe(
        ReportPolicy.REQUIRED
    )
})
