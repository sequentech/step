// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {readinessReport} from "./readinessReport"

const valid = {
    passed: true,
    checked_at: "2026-10-16T09:00:00Z",
    log_entry_id: "signed-entry",
    checks: [
        {kind: "configuration", passed: true, message: "Approved content matches"},
        {kind: "certified_voters", passed: true, message: "Certified file matches"},
        {kind: "official_zero", passed: true, message: "Official event is at zero"},
        {kind: "publication", passed: true, message: "Publication matches"},
        {kind: "tested_finished", passed: true, message: "Tested event is closed and sealed"},
    ],
    evidence: {official: {event_id: "official", ballot_count: 0}, tested: {event_id: "tested"}},
}
const read = (value: unknown) => readinessReport(value, "official", "tested")
it("retains the full server evidence of a signed pass", () => {
    expect(read(valid)).toBe(valid)
})
it("accepts an unsigned failed comparison with every check's outcome", () => {
    const failed = {
        ...valid,
        passed: false,
        log_entry_id: null,
        checks: valid.checks.map((check) =>
            check.kind === "official_zero" ? {...check, passed: false} : check
        ),
    }
    expect(read(failed)).toBe(failed)
})
it("accepts failures explained by findings, and older failures without them", () => {
    const explained = {
        ...valid,
        passed: false,
        log_entry_id: null,
        checks: valid.checks.map((check) =>
            check.kind === "official_zero"
                ? {
                      ...check,
                      passed: false,
                      findings: [
                          {
                              code: "ballots_present",
                              event: "official",
                              values: {count: "1"},
                              message: "The official event has 1 stored ballots",
                          },
                      ],
                  }
                : check
        ),
    }
    expect(read(explained)).toBe(explained)
})
it.each([
    ["absent", null],
    ["empty", {}],
    ["no timestamp", {...valid, checked_at: ""}],
    ["no checks", {...valid, checks: []}],
    ["missing check", {...valid, checks: valid.checks.slice(1)}],
    ["duplicate check", {...valid, checks: [...valid.checks, valid.checks[0]]}],
    ["malformed check", {...valid, checks: [...valid.checks.slice(1), null]}],
    [
        "non-boolean check",
        {...valid, checks: [{...valid.checks[0], passed: "true"}, ...valid.checks.slice(1)]},
    ],
    ["missing evidence", {...valid, evidence: null}],
    [
        "wrong official event",
        {...valid, evidence: {...valid.evidence, official: {event_id: "other"}}},
    ],
    ["wrong tested event", {...valid, evidence: {...valid.evidence, tested: {event_id: "other"}}}],
    ["absent signature", {...valid, log_entry_id: null}],
    ["empty signature", {...valid, log_entry_id: " "}],
    ["inconsistent verdict", {...valid, passed: false}],
    [
        "passed check with a failure finding",
        {
            ...valid,
            checks: [
                {
                    ...valid.checks[0],
                    findings: [{code: "content_mismatch", event: "both", message: "differs"}],
                },
                ...valid.checks.slice(1),
            ],
        },
    ],
    [
        "malformed findings",
        {
            ...valid,
            passed: false,
            log_entry_id: null,
            checks: [
                {...valid.checks[0], passed: false, findings: [{code: 1, message: "x"}]},
                ...valid.checks.slice(1),
            ],
        },
    ],
    [
        "findings that are not a list",
        {
            ...valid,
            passed: false,
            log_entry_id: null,
            checks: [
                {...valid.checks[0], passed: false, findings: "differs"},
                ...valid.checks.slice(1),
            ],
        },
    ],
])("rejects %s reports instead of displaying a passing result", (_name, value) => {
    expect(read(value)).toBeNull()
})
