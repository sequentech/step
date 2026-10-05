// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import i18next from "i18next"
import en from "@/translations/en"
import {IApplicationsStatus} from "@/types/applications"
import {
    EDifferingFields,
    EFieldMatch,
    EIdentityMethod,
    EMatrixError,
    EMatrixReason,
    IApprovalMatrix,
    IApprovalRule,
    Translate,
    addRule,
    cleanConditions,
    cleanMatrix,
    conditionLabels,
    defaultEnrollment,
    deleteRule,
    enrollmentFor,
    humanizeField,
    matrixChanges,
    moveRule,
    profileFieldLabel,
    rejectionReasonKey,
    readMatrix,
    replaceRule,
    ruleSentence,
    sameMatrix,
    validateMatrix,
    validateRule,
    withComparedFields,
} from "./approvalMatrix"

const {ACCEPTED, PENDING, REJECTED} = IApplicationsStatus

const comelec = (): IApprovalMatrix => ({
    compared_fields: ["firstName", "middleName", "lastName", "dateOfBirth", "embassy"],
    rules: [
        {
            when: {already_enrolled: true, differing: EDifferingFields.AT_MOST_1},
            then: {decision: REJECTED, reason: EMatrixReason.ALREADY_APPROVED},
        },
        {
            when: {identity: EIdentityMethod.MANUAL_ENTRY},
            then: {decision: PENDING, reason: EMatrixReason.IDENTITY_NOT_VERIFIED},
        },
        {when: {differing: EDifferingFields.NONE}, then: {decision: ACCEPTED}},
        {
            when: {differing: EDifferingFields.EXACTLY_1, fields: {embassy: EFieldMatch.DIFFERS}},
            then: {decision: ACCEPTED},
        },
        {
            when: {differing: EDifferingFields.EXACTLY_1, fields: {embassy: EFieldMatch.MATCHES}},
            then: {decision: PENDING, reason: EMatrixReason.NO_VOTER},
        },
    ],
    otherwise: {decision: REJECTED, reason: EMatrixReason.NO_VOTER},
})

let t: Translate

beforeAll(async () => {
    const instance = i18next.createInstance()
    await instance.init({
        lng: "en",
        defaultNS: "translations",
        resources: {en},
        interpolation: {escapeValue: false},
    })
    t = (key, options) => instance.t(key, options ?? {})
})

describe("readMatrix", () => {
    it("reads the matrix the backend returns", () => {
        expect(readMatrix(JSON.parse(JSON.stringify(comelec())))).toEqual(comelec())
    })

    it("drops conditions it does not know", () => {
        const matrix = readMatrix({
            compared_fields: ["firstName", 7],
            rules: [
                {
                    when: {
                        identity: "SELFIE",
                        voter_found: "yes",
                        valid_id: "",
                        differing: "exactly_9",
                        fields: {firstName: "SIMILAR"},
                    },
                    then: {decision: "PENDING", reason: "BECAUSE"},
                },
            ],
            otherwise: {decision: "REJECTED", reason: "NO_VOTER"},
        })

        expect(matrix).toEqual({
            compared_fields: ["firstName"],
            rules: [{when: {}, then: {decision: PENDING}}],
            otherwise: {decision: REJECTED, reason: EMatrixReason.NO_VOTER},
        })
    })

    it.each([
        null,
        "matrix",
        [],
        {rules: [], otherwise: {decision: "REJECTED"}},
        {compared_fields: [], rules: {}, otherwise: {decision: "REJECTED"}},
        {compared_fields: [], rules: [], otherwise: {decision: "APPROVED"}},
        {compared_fields: [], rules: [], otherwise: null},
        {compared_fields: [], rules: [null], otherwise: {decision: "REJECTED"}},
        {compared_fields: [], rules: [{when: {}}], otherwise: {decision: "REJECTED"}},
    ])("refuses %j", (value) => {
        expect(readMatrix(value)).toBeNull()
    })
})

describe("cleaning", () => {
    it("keeps only conditions that are set", () => {
        expect(
            cleanConditions(
                {
                    identity: undefined,
                    voter_found: false,
                    valid_id: "",
                    fields: {embassy: EFieldMatch.DIFFERS, placeOfBirth: EFieldMatch.MATCHES},
                },
                ["embassy"]
            )
        ).toEqual({voter_found: false, fields: {embassy: EFieldMatch.DIFFERS}})
    })

    it("leaves out the fields when none is compared", () => {
        expect(cleanConditions({fields: {placeOfBirth: EFieldMatch.MATCHES}}, ["embassy"])).toEqual(
            {}
        )
    })

    it("sends no reason with an approval and no repeated or blank field", () => {
        const matrix = comelec()
        matrix.compared_fields = [" firstName ", "firstName", "", "embassy"]
        matrix.rules = [{when: {}, then: {decision: ACCEPTED, reason: EMatrixReason.OTHER}}]

        expect(cleanMatrix(matrix)).toEqual({
            compared_fields: ["firstName", "embassy"],
            rules: [{when: {}, then: {decision: ACCEPTED}}],
            otherwise: {decision: REJECTED, reason: EMatrixReason.NO_VOTER},
        })
    })

    it("compares matrices whatever the order of their keys", () => {
        const reordered = comelec()
        reordered.rules[3] = {
            then: {decision: ACCEPTED},
            when: {fields: {embassy: EFieldMatch.DIFFERS}, differing: EDifferingFields.EXACTLY_1},
        }
        expect(sameMatrix(comelec(), reordered)).toBe(true)
        expect(sameMatrix(comelec(), moveRule(comelec(), 0, 1))).toBe(false)
    })
})

describe("validateRule", () => {
    const rule = (when: IApprovalRule["when"], then: IApprovalRule["then"]): IApprovalRule => ({
        when,
        then,
    })

    it("accepts the rules of the built-in matrix", () => {
        comelec().rules.forEach((current) => expect(validateRule(current)).toEqual([]))
        expect(validateMatrix(comelec())).toEqual([])
    })

    it("refuses approving an identity entered manually", () => {
        expect(
            validateRule(rule({identity: EIdentityMethod.MANUAL_ENTRY}, {decision: ACCEPTED}))
        ).toEqual([EMatrixError.ACCEPTS_MANUAL_ENTRY])
    })

    it("refuses approving a voter who is already enrolled", () => {
        expect(validateRule(rule({already_enrolled: true}, {decision: ACCEPTED}))).toEqual([
            EMatrixError.ACCEPTS_ALREADY_ENROLLED,
        ])
    })

    it("refuses approving without a registry voter", () => {
        expect(validateRule(rule({voter_found: false}, {decision: ACCEPTED}))).toEqual([
            EMatrixError.ACCEPTS_WITHOUT_VOTER,
        ])
    })

    it("refuses an approving last rule", () => {
        expect(validateRule(rule({}, {decision: ACCEPTED}), true)).toEqual([
            EMatrixError.OTHERWISE_ACCEPTS,
        ])
    })

    it.each([PENDING, REJECTED])("asks for the reason of %s", (decision) => {
        expect(validateRule(rule({voter_found: true}, {decision}))).toEqual([
            EMatrixError.MISSING_REASON,
        ])
        expect(validateRule(rule({}, {decision}), true)).toEqual([EMatrixError.MISSING_REASON])
    })

    it("refuses a rule without conditions, which would hide the rules below it", () => {
        expect(validateRule(rule({}, {decision: PENDING, reason: EMatrixReason.OTHER}))).toEqual([
            EMatrixError.NO_CONDITIONS,
        ])
        expect(
            validateRule(rule({}, {decision: PENDING, reason: EMatrixReason.OTHER}), true)
        ).toEqual([])
    })

    it("names the rule of each error of a matrix", () => {
        const matrix = comelec()
        matrix.compared_fields = []
        matrix.rules[1].then = {decision: ACCEPTED}
        matrix.otherwise = {decision: REJECTED}

        expect(validateMatrix(matrix)).toEqual([
            {code: EMatrixError.NO_COMPARED_FIELDS, rule: null},
            {code: EMatrixError.ACCEPTS_MANUAL_ENTRY, rule: 2},
            {code: EMatrixError.MISSING_REASON, rule: null},
        ])
    })
})

describe("editing", () => {
    const decisions = (matrix: IApprovalMatrix) =>
        matrix.rules.map((current) => current.then.reason ?? current.then.decision)

    it("adds a rule after the others", () => {
        const added = addRule(comelec(), {when: {}, then: {decision: ACCEPTED}})
        expect(added.rules).toHaveLength(6)
        expect(added.rules[5]).toEqual({when: {}, then: {decision: ACCEPTED}})
        expect(comelec().rules).toHaveLength(5)
    })

    it("replaces and deletes one rule", () => {
        const replaced = replaceRule(comelec(), 2, {
            when: {},
            then: {decision: PENDING, reason: EMatrixReason.OTHER},
        })
        expect(decisions(replaced)[2]).toBe(EMatrixReason.OTHER)
        expect(decisions(deleteRule(replaced, 2))).toEqual([
            EMatrixReason.ALREADY_APPROVED,
            EMatrixReason.IDENTITY_NOT_VERIFIED,
            ACCEPTED,
            EMatrixReason.NO_VOTER,
        ])
    })

    it("moves a rule up and down", () => {
        expect(decisions(moveRule(comelec(), 1, -1)).slice(0, 2)).toEqual([
            EMatrixReason.IDENTITY_NOT_VERIFIED,
            EMatrixReason.ALREADY_APPROVED,
        ])
        expect(decisions(moveRule(comelec(), 0, 1)).slice(0, 2)).toEqual([
            EMatrixReason.IDENTITY_NOT_VERIFIED,
            EMatrixReason.ALREADY_APPROVED,
        ])
    })

    it("keeps the order at either end and for unknown positions", () => {
        const matrix = comelec()
        expect(moveRule(matrix, 0, -1)).toBe(matrix)
        expect(moveRule(matrix, 4, 1)).toBe(matrix)
        expect(moveRule(matrix, 9, -1)).toBe(matrix)
        expect(moveRule(matrix, -1, 1)).toBe(matrix)
    })

    it("drops the conditions on a field that is no longer compared", () => {
        const matrix = withComparedFields(comelec(), ["firstName", "lastName", "dateOfBirth"])
        expect(matrix.rules[3].when).toEqual({differing: EDifferingFields.EXACTLY_1})
        expect(matrix.compared_fields).toEqual(["firstName", "lastName", "dateOfBirth"])
    })
})

describe("labels", () => {
    it("names each condition", () => {
        expect(
            comelec().rules.map((current) => conditionLabels(current.when, t).join(" · "))
        ).toEqual([
            "Already enrolled · At most 1 detail differs",
            "Identity typed by hand",
            "All details match",
            "Exactly 1 detail differs · Embassy differs",
            "Exactly 1 detail differs · Embassy matches",
        ])
    })

    it("names the remaining conditions", () => {
        expect(
            conditionLabels(
                {
                    identity: EIdentityMethod.VERIFIED,
                    voter_found: false,
                    already_enrolled: false,
                    valid_id: "philippinePassport",
                    differing: EDifferingFields.AT_LEAST_3,
                },
                t
            )
        ).toEqual([
            "Identity verified by ID scan",
            "No voter found in the registry",
            "Not enrolled yet",
            "ID: Philippine Passport",
            "3 or more details differ",
        ])
        expect(conditionLabels({voter_found: true}, t)).toEqual(["Voter found in the registry"])
        expect(
            conditionLabels({differing: EDifferingFields.EXACTLY_2}, t).concat(
                conditionLabels({differing: EDifferingFields.AT_MOST_2}, t)
            )
        ).toEqual(["Exactly 2 details differ", "At most 2 details differ"])
    })

    it("says so when a rule has no condition", () => {
        expect(conditionLabels({}, t)).toEqual(["No conditions yet"])
    })

    it("uses the caller's field names", () => {
        expect(
            conditionLabels({fields: {dateOfBirth: EFieldMatch.DIFFERS}}, t, () => "Birthday")
        ).toEqual(["Birthday differs"])
    })

    it("names fields the user profile does not", () => {
        expect(humanizeField("dateOfBirth")).toBe("Date Of Birth")
        expect(humanizeField("sequent.read-only.id-card-number")).toBe(
            "Sequent Read Only Id Card Number"
        )
    })

    it("says a rule in one sentence", () => {
        const rules = comelec().rules
        expect(ruleSentence(rules[3], false, t)).toBe(
            "When exactly 1 detail differs and embassy differs, approve the enrollment automatically."
        )
        expect(ruleSentence(rules[1], false, t)).toBe(
            "When identity typed by hand, send the enrollment to a person."
        )
        expect(ruleSentence({when: {}, then: comelec().otherwise}, true, t)).toBe(
            "If none of the rules above apply, reject the enrollment."
        )
        expect(ruleSentence({when: {}, then: {decision: PENDING}}, false, t)).toBe(
            "Add a condition to say when this rule applies."
        )
    })
})

describe("matrixChanges", () => {
    const changes = (draft: IApprovalMatrix) => matrixChanges(comelec(), draft, t)

    it("has none for the saved matrix", () => {
        expect(changes(comelec())).toEqual([])
    })

    it("says which decision a rule changed to", () => {
        const draft = comelec()
        draft.rules[3].then = {decision: PENDING, reason: EMatrixReason.NO_VOTER}
        expect(changes(draft)).toEqual(["Rule 4: approve automatically → send to a person"])
    })

    it("says that a rule's conditions or reason changed", () => {
        const draft = comelec()
        draft.rules[4].when = {differing: EDifferingFields.EXACTLY_2}
        draft.rules[1].then = {decision: PENDING, reason: EMatrixReason.OTHER}
        expect(changes(draft)).toEqual(["Rule 2 changed", "Rule 5 changed"])
    })

    it("names an added rule by its position", () => {
        const added = addRule(comelec(), {
            when: {voter_found: false},
            then: {decision: REJECTED, reason: EMatrixReason.NO_VOTER},
        })
        expect(changes(added)).toEqual(["Rule 6 added"])
        expect(changes(moveRule(moveRule(added, 5, -1), 4, -1))).toEqual(["Rule 4 added"])
    })

    it("names a removed rule by its conditions", () => {
        expect(changes(deleteRule(comelec(), 1))).toEqual([
            "A rule was removed (Identity typed by hand)",
        ])
    })

    it("says that the rules were reordered", () => {
        expect(changes(moveRule(comelec(), 0, 1))).toEqual(["Rules were reordered"])
    })

    it("lists an edit and a removal made together", () => {
        const draft = deleteRule(comelec(), 4)
        draft.rules[2].then = {decision: PENDING, reason: EMatrixReason.OTHER}
        expect(changes(draft)).toEqual([
            "Rule 3: approve automatically → send to a person",
            "A rule was removed (Exactly 1 detail differs, Embassy matches)",
        ])
    })

    it("says that the last rule or the details compared changed", () => {
        const draft = withComparedFields(comelec(), ["firstName", "lastName", "embassy"])
        draft.otherwise = {decision: PENDING, reason: EMatrixReason.NO_VOTER}
        expect(changes(draft)).toEqual(["The details compared changed", "The last rule changed"])
    })
})

describe("test enrollment", () => {
    it("starts from a verified voter whose fields match", () => {
        expect(defaultEnrollment(["firstName", "embassy"])).toEqual({
            identity: EIdentityMethod.VERIFIED,
            voter_found: true,
            already_enrolled: false,
            valid_id: null,
            fields: {firstName: EFieldMatch.MATCHES, embassy: EFieldMatch.MATCHES},
        })
    })

    it("follows the compared fields", () => {
        const enrollment = defaultEnrollment(["firstName", "embassy"])
        enrollment.fields.embassy = EFieldMatch.DIFFERS

        expect(enrollmentFor(enrollment, ["embassy", "lastName"]).fields).toEqual({
            embassy: EFieldMatch.DIFFERS,
            lastName: EFieldMatch.MATCHES,
        })
    })
})

describe("profileFieldLabel", () => {
    const label = profileFieldLabel(
        [
            {name: "first_name", display_name: "${firstName}"},
            {name: "embassy", display_name: null},
            {name: null, display_name: "Nameless"},
        ],
        (key) => `t(${key})`,
        (displayName) => displayName.replace(/[${}]/g, "")
    )

    it("uses the user profile's name of the field", () => {
        expect(label("firstName")).toBe("t(firstName)")
        expect(label("embassy")).toBe("t(embassy)")
    })

    it("falls back to the field's own name", () => {
        expect(label("dateOfBirth")).toBe("Date Of Birth")
    })
})

describe("rejectionReasonKey", () => {
    it("reads a reason the matrix stored by name", () => {
        expect(rejectionReasonKey("NO_VOTER")).toBe("approvalsScreen.matrix.reasons.NO_VOTER")
        expect(rejectionReasonKey("IDENTITY_NOT_VERIFIED")).toBe(
            "approvalsScreen.matrix.reasons.IDENTITY_NOT_VERIFIED"
        )
    })

    it("keeps the slug of a manual rejection and the missing reason", () => {
        expect(rejectionReasonKey("no-matching-voter")).toBe(
            "approvalsScreen.reject.reasons.no-matching-voter"
        )
        expect(rejectionReasonKey(null)).toBe("approvalsScreen.reject.reasons.undefined")
        expect(rejectionReasonKey(undefined)).toBe("approvalsScreen.reject.reasons.undefined")
    })
})
