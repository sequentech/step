// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {IApplicationsStatus} from "@/types/applications"
import {EFieldMatch, EIdentityMethod, ITestEnrollment} from "./approvalMatrix"
import {
    EFieldState,
    EHistoryComparison,
    EMatrixEntry,
    EPanelState,
    IRecordedDecision,
    customizedSample,
    enrollmentFor,
    fieldState,
    historyComparison,
    panelState,
    recordedDecision,
    sameEnrollment,
    sampleEnrollment,
} from "./approvalExample"
import type {IApplication} from "./approvalReview"

const {ACCEPTED, PENDING, REJECTED} = IApplicationsStatus
const COMPARED = ["firstName", "middleName", "lastName", "dateOfBirth"]
const ID_CARD_TYPE = "sequent.read-only.id-card-type"

const inputs = (overrides: Record<string, unknown> = {}) => ({
    identity: "MANUAL_ENTRY",
    voter_found: true,
    already_enrolled: false,
    valid_id: null,
    fields: {firstName: "MATCHES", middleName: "MATCHES", lastName: "DIFFERS"},
    differing: 1,
    ...overrides,
})

const application = (
    record: Record<string, unknown> = {},
    data: Record<string, string> = {firstName: "Carol", middleName: "Ann", [ID_CARD_TYPE]: "iBP"}
): IApplication => ({
    status: PENDING,
    applicant_data: data,
    annotations: {
        decision: {
            matrix_version: 1,
            matrix_source: "BUILT_IN",
            rule: 2,
            conditions: {identity: "MANUAL_ENTRY"},
            decision: "PENDING",
            reason: "IDENTITY_NOT_VERIFIED",
            inputs: inputs(),
            candidates: 1,
            accepted_candidates: 0,
            ...record,
        },
    },
})

const recorded = (
    record: Record<string, unknown> = {},
    data?: Record<string, string>
): IRecordedDecision => {
    const value = recordedDecision(application(record, data))
    if (!value) throw new Error("no recorded decision")
    return value
}

describe("recordedDecision", () => {
    it("reads the decision and the inputs the rules saw", () => {
        expect(recorded()).toEqual({
            matrixVersion: 1,
            rule: 2,
            decision: PENDING,
            invariant: null,
            approvedVoters: 0,
            candidates: 1,
            jointNames: false,
            enrollment: {
                identity: EIdentityMethod.MANUAL_ENTRY,
                voter_found: true,
                already_enrolled: false,
                valid_id: null,
                fields: {
                    firstName: EFieldMatch.MATCHES,
                    middleName: EFieldMatch.MATCHES,
                    lastName: EFieldMatch.DIFFERS,
                },
            },
        })
    })

    it("keeps an identity and an ID type that weren't reported as empty", () => {
        const value = recorded({inputs: inputs({identity: null, valid_id: "seamanBook"})})
        expect(value.enrollment).toMatchObject({identity: null, valid_id: "seamanBook"})
    })

    it("reads the last rule, an invariant and several approved voters", () => {
        expect(
            recorded({rule: null, invariant: "MANUAL_ENTRY_NOT_ACCEPTED", accepted_candidates: 2})
        ).toMatchObject({rule: null, invariant: "MANUAL_ENTRY_NOT_ACCEPTED", approvedVoters: 2})
    })

    it.each([undefined, null, "2", -1, 1.5, Number.NaN, Number.POSITIVE_INFINITY])(
        "keeps a number of compared voters recorded as %p unknown",
        (candidates) => {
            expect(recorded({candidates}).candidates).toBeNull()
        }
    )

    it.each([undefined, -2, 0.5, "3"])(
        "doesn't count approved voters recorded as %p",
        (accepted_candidates) => {
            expect(recorded({accepted_candidates}).approvedVoters).toBeNull()
        }
    )

    it("reads inputs without fields as no compared results", () => {
        const value = recorded({inputs: inputs({fields: undefined})})
        expect(value.enrollment?.fields).toEqual({})
    })

    it("can't read a record without its rule", () => {
        // Harvest always writes the rule, empty for the last one.
        expect(recorded({rule: undefined})).toMatchObject({decision: null, enrollment: null})
    })

    it.each([
        ["a voter but no compared voters", {candidates: 0}],
        [
            "no voter but compared voters",
            {candidates: 1, inputs: inputs({voter_found: false, fields: {}})},
        ],
    ])("can't replay a record with %s", (_, record) => {
        expect(recorded(record).enrollment).toBeNull()
    })

    it("replays a record whose number of compared voters wasn't recorded", () => {
        expect(recorded({candidates: undefined}).enrollment).not.toBeNull()
    })

    it("reads a record without a voter", () => {
        const value = recorded({candidates: 0, inputs: inputs({voter_found: false, fields: {}})})
        expect(value.enrollment).toMatchObject({voter_found: false, fields: {}})
    })

    it.each([
        ["no inputs", {inputs: undefined}],
        ["inputs that aren't an object", {inputs: "VERIFIED"}],
        ["no voter_found", {inputs: inputs({voter_found: undefined})}],
        ["a voter_found that isn't a boolean", {inputs: inputs({voter_found: "true"})}],
        ["no already_enrolled", {inputs: inputs({already_enrolled: undefined})}],
        ["an already_enrolled that isn't a boolean", {inputs: inputs({already_enrolled: 0})}],
        ["an unknown identity", {inputs: inputs({identity: "FACE"})}],
        ["no identity", {inputs: inputs({identity: undefined})}],
        ["an ID type that isn't text", {inputs: inputs({valid_id: 7})}],
        ["no ID type", {inputs: inputs({valid_id: undefined})}],
        ["fields that aren't an object", {inputs: inputs({fields: ["firstName"]})}],
        ["empty fields", {inputs: inputs({fields: null})}],
        ["an unknown field result", {inputs: inputs({fields: {firstName: "SIMILAR"}})}],
    ])("can't replay a record with %s", (_, record) => {
        const value = recorded(record)
        expect(value.enrollment).toBeNull()
    })

    it("has no recorded decision without a matrix version", () => {
        expect(recordedDecision({annotations: {}})).toBeNull()
        expect(recordedDecision({annotations: {decision: {rule: 1}}})).toBeNull()
        expect(recordedDecision({})).toBeNull()
    })

    it.each([
        ["a decision that isn't a status", {decision: "MAYBE"}],
        ["no decision", {decision: undefined}],
        ["a rule that isn't a position", {rule: 0}],
        ["a fractional rule", {rule: 1.5}],
        ["a rule written as text", {rule: "2"}],
        ["an unknown invariant", {invariant: "NEVER"}],
        ["a version of 0", {matrix_version: 0}],
        ["a fractional version", {matrix_version: 1.5}],
    ])("can't read a record with %s", (_, record) => {
        expect(recorded(record)).toMatchObject({decision: null, enrollment: null})
    })

    it.each([
        ["a seaman's book", {[ID_CARD_TYPE]: "seamanBook", middleName: "Ann"}, true],
        ["a driver's license", {[ID_CARD_TYPE]: "driversLicense"}, true],
        ["a passport without a middle name", {[ID_CARD_TYPE]: "philippinePassport"}, true],
        [
            "a passport with a middle name",
            {[ID_CARD_TYPE]: "philippinePassport", middleName: "Ann"},
            false,
        ],
        ["another ID", {[ID_CARD_TYPE]: "iBP"}, false],
    ])("compares first and middle name together for %s: %s", (_, data, joint) => {
        expect(recorded({}, data).jointNames).toBe(joint)
    })
})

describe("fieldState", () => {
    const enrollment: ITestEnrollment = {
        ...sampleEnrollment(),
        fields: {lastName: EFieldMatch.DIFFERS},
    }

    it("shows a recorded result", () => {
        expect(fieldState(enrollment, "lastName", EMatrixEntry.ENROLLMENT, false)).toBe(
            EFieldState.DIFFERS
        )
    })

    it("never turns a detail an enrollment didn't record into Same", () => {
        expect(fieldState(enrollment, "dateOfBirth", EMatrixEntry.ENROLLMENT, false)).toBe(
            EFieldState.NOT_RECORDED
        )
    })

    it("compares the middle name with the first name when the ID gives them together", () => {
        expect(fieldState(enrollment, "middleName", EMatrixEntry.ENROLLMENT, true)).toBe(
            EFieldState.WITH_FIRST_NAME
        )
        expect(fieldState(enrollment, "lastName", EMatrixEntry.ENROLLMENT, true)).toBe(
            EFieldState.DIFFERS
        )
        expect(fieldState(enrollment, "middleName", EMatrixEntry.ENROLLMENT, false)).toBe(
            EFieldState.NOT_RECORDED
        )
    })

    it("treats every detail of the sample as the same unless chosen", () => {
        expect(fieldState(enrollment, "dateOfBirth", EMatrixEntry.TOOLBAR, false)).toBe(
            EFieldState.MATCHES
        )
        expect(fieldState(enrollment, "lastName", EMatrixEntry.TOOLBAR, false)).toBe(
            EFieldState.DIFFERS
        )
    })
})

describe("enrollmentFor", () => {
    it("sends the sample's compared details as the same", () => {
        expect(enrollmentFor(sampleEnrollment(), COMPARED, EMatrixEntry.TOOLBAR, false)).toEqual({
            identity: EIdentityMethod.VERIFIED,
            voter_found: true,
            already_enrolled: false,
            valid_id: null,
            fields: {
                firstName: EFieldMatch.MATCHES,
                middleName: EFieldMatch.MATCHES,
                lastName: EFieldMatch.MATCHES,
                dateOfBirth: EFieldMatch.MATCHES,
            },
        })
    })

    it("leaves out what an enrollment didn't record, as the rules saw it", () => {
        const enrollment = recorded({}, {[ID_CARD_TYPE]: "seamanBook"}).enrollment
        if (!enrollment) throw new Error("unreadable")
        expect(
            enrollmentFor(
                {...enrollment, fields: {firstName: EFieldMatch.DIFFERS}},
                COMPARED,
                EMatrixEntry.ENROLLMENT,
                true
            ).fields
        ).toEqual({firstName: EFieldMatch.DIFFERS})
    })

    it("only sends the compared details", () => {
        const enrollment = {...sampleEnrollment(), fields: {embassy: EFieldMatch.DIFFERS}}
        expect(
            enrollmentFor(enrollment, ["lastName"], EMatrixEntry.ENROLLMENT, false).fields
        ).toEqual({})
    })

    it("compares nothing, and nobody is enrolled, without a voter", () => {
        const enrollment: ITestEnrollment = {
            ...sampleEnrollment(),
            voter_found: false,
            already_enrolled: true,
            fields: {lastName: EFieldMatch.DIFFERS},
        }
        expect(enrollmentFor(enrollment, COMPARED, EMatrixEntry.TOOLBAR, false)).toMatchObject({
            voter_found: false,
            already_enrolled: false,
            fields: {},
        })
    })
})

describe("sameEnrollment", () => {
    it("compares every input", () => {
        const sample = sampleEnrollment()
        expect(sameEnrollment(sample, sampleEnrollment())).toBe(true)
        expect(sameEnrollment(sample, {...sample, valid_id: "iBP"})).toBe(false)
        expect(sameEnrollment(sample, {...sample, identity: null})).toBe(false)
        expect(
            sameEnrollment(
                {...sample, fields: {a: EFieldMatch.MATCHES, b: EFieldMatch.DIFFERS}},
                {...sample, fields: {b: EFieldMatch.DIFFERS, a: EFieldMatch.MATCHES}}
            )
        ).toBe(true)
        expect(sameEnrollment(sample, {...sample, fields: {a: EFieldMatch.MATCHES}})).toBe(false)
    })
})

describe("customizedSample", () => {
    it("is the sample until a compared detail or an input changes", () => {
        expect(customizedSample(sampleEnrollment(), COMPARED)).toBe(false)
        // Choosing Same for a detail is still the sample.
        expect(
            customizedSample(
                {...sampleEnrollment(), fields: {lastName: EFieldMatch.MATCHES}},
                COMPARED
            )
        ).toBe(false)
        expect(
            customizedSample(
                {...sampleEnrollment(), identity: EIdentityMethod.MANUAL_ENTRY},
                COMPARED
            )
        ).toBe(true)
        expect(
            customizedSample(
                {...sampleEnrollment(), fields: {lastName: EFieldMatch.DIFFERS}},
                COMPARED
            )
        ).toBe(true)
        // A detail that is no longer compared doesn't count.
        expect(
            customizedSample(
                {...sampleEnrollment(), fields: {embassy: EFieldMatch.DIFFERS}},
                COMPARED
            )
        ).toBe(false)
    })
})

describe("historyComparison", () => {
    const complete = recorded({
        inputs: inputs({
            fields: {
                firstName: "MATCHES",
                middleName: "MATCHES",
                lastName: "DIFFERS",
                dateOfBirth: "MATCHES",
            },
        }),
    })
    const replay = (value: IRecordedDecision) => {
        if (!value.enrollment) throw new Error("unreadable")
        return value.enrollment
    }

    it("compares the decision of the unchanged recorded details", () => {
        expect(historyComparison(complete, replay(complete), COMPARED, PENDING)).toBe(
            EHistoryComparison.SAME
        )
        expect(historyComparison(complete, replay(complete), COMPARED, ACCEPTED)).toBe(
            EHistoryComparison.DIFFERENT
        )
    })

    it("doesn't compare changed details", () => {
        expect(
            historyComparison(
                complete,
                {...replay(complete), identity: EIdentityMethod.VERIFIED},
                COMPARED,
                PENDING
            )
        ).toBe(EHistoryComparison.EDITED)
    })

    it("only counts changes the rules see as edits", () => {
        const none = recorded({candidates: 0, inputs: inputs({voter_found: false, fields: {}})})
        // Already enrolled and details mean nothing without a voter.
        expect(
            historyComparison(
                none,
                {...replay(none), already_enrolled: true, fields: {lastName: EFieldMatch.DIFFERS}},
                COMPARED,
                PENDING
            )
        ).toBe(EHistoryComparison.SAME)
        // Nor does a detail that isn't compared any more.
        expect(
            historyComparison(
                complete,
                {
                    ...replay(complete),
                    fields: {...replay(complete).fields, embassy: EFieldMatch.DIFFERS},
                },
                COMPARED,
                PENDING
            )
        ).toBe(EHistoryComparison.SAME)
    })

    it("names the first reason it doesn't compare", () => {
        const several = {...recorded(), candidates: 2}
        expect(
            historyComparison(several, {...replay(several), valid_id: "iBP"}, COMPARED, PENDING)
        ).toBe(EHistoryComparison.EDITED)
        // Several voters and an unrecorded detail: the voters come first.
        expect(historyComparison(several, replay(several), COMPARED, PENDING)).toBe(
            EHistoryComparison.SEVERAL_VOTERS
        )
        const unknown = {...recorded(), candidates: null}
        expect(historyComparison(unknown, replay(unknown), COMPARED, PENDING)).toBe(
            EHistoryComparison.UNKNOWN_VOTERS
        )
    })

    it("doesn't compare when the number of compared voters is unknown or several", () => {
        const unknown = {...complete, candidates: null}
        expect(historyComparison(unknown, replay(unknown), COMPARED, PENDING)).toBe(
            EHistoryComparison.UNKNOWN_VOTERS
        )
        const several = {...complete, candidates: 3}
        expect(historyComparison(several, replay(several), COMPARED, PENDING)).toBe(
            EHistoryComparison.SEVERAL_VOTERS
        )
    })

    it("doesn't compare when a compared detail wasn't recorded", () => {
        const partial = recorded()
        expect(historyComparison(partial, replay(partial), COMPARED, PENDING)).toBe(
            EHistoryComparison.NOT_RECORDED
        )
    })

    it("counts a middle name compared with the first name as recorded", () => {
        const joint = recorded(
            {
                inputs: inputs({
                    fields: {firstName: "MATCHES", lastName: "DIFFERS", dateOfBirth: "MATCHES"},
                }),
            },
            {[ID_CARD_TYPE]: "seamanBook"}
        )
        expect(historyComparison(joint, replay(joint), COMPARED, PENDING)).toBe(
            EHistoryComparison.SAME
        )
    })

    it("compares a record without a voter whatever is compared", () => {
        const none = recorded({candidates: 0, inputs: inputs({voter_found: false, fields: {}})})
        expect(historyComparison(none, replay(none), COMPARED, REJECTED)).toBe(
            EHistoryComparison.DIFFERENT
        )
    })

    it("says nothing without a result, a readable record or a recorded decision", () => {
        expect(historyComparison(complete, replay(complete), COMPARED, null)).toBeNull()
        const unreadable = recorded({inputs: undefined})
        expect(historyComparison(unreadable, sampleEnrollment(), COMPARED, PENDING)).toBeNull()
        const undecided = {...complete, decision: null}
        expect(historyComparison(undecided, replay(undecided), COMPARED, PENDING)).toBeNull()
    })
})

describe("panelState", () => {
    const enrollment = recorded({
        inputs: inputs({
            fields: {
                firstName: "MATCHES",
                middleName: "MATCHES",
                lastName: "DIFFERS",
                dateOfBirth: "MATCHES",
            },
        }),
    })
    const replayed = enrollment.enrollment as ITestEnrollment
    const typed = {...sampleEnrollment(), identity: EIdentityMethod.MANUAL_ENTRY}
    const {TOOLBAR, ENROLLMENT} = EMatrixEntry

    it("is the sample, or the administrator's own example, from the toolbar", () => {
        expect(panelState(TOOLBAR, null, sampleEnrollment(), COMPARED)).toBe(EPanelState.SAMPLE)
        expect(panelState(TOOLBAR, null, typed, COMPARED)).toBe(EPanelState.CUSTOM)
    })

    it("is an enrollment's recorded details until other details are used", () => {
        expect(panelState(ENROLLMENT, enrollment, replayed, COMPARED)).toBe(EPanelState.RECORDED)
        expect(panelState(ENROLLMENT, enrollment, {...replayed, valid_id: "iBP"}, COMPARED)).toBe(
            EPanelState.CHANGED
        )
    })

    it("counts only changes the rules see, as the comparison does", () => {
        // A detail no longer compared is no change.
        const embassy = {...replayed, fields: {...replayed.fields, embassy: EFieldMatch.DIFFERS}}
        expect(panelState(ENROLLMENT, enrollment, embassy, COMPARED)).toBe(EPanelState.RECORDED)
    })

    it.each([
        ["can't be read", recorded({inputs: undefined})],
        ["has no decision record", null],
    ])("uses the sample for an enrollment whose decision %s", (_, decision) => {
        expect(panelState(ENROLLMENT, decision, sampleEnrollment(), COMPARED)).toBe(
            EPanelState.UNREADABLE
        )
        // Other details then started from the sample, not from the enrollment's.
        expect(panelState(ENROLLMENT, decision, typed, COMPARED)).toBe(
            EPanelState.CHANGED_FROM_SAMPLE
        )
    })
})
