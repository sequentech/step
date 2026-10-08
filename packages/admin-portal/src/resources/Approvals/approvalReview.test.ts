// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import i18next from "i18next"
import en from "@/translations/en"
import {IApplicationsStatus} from "@/types/applications"
import {EFieldMatch, EIdentityMethod, Translate, humanizeField} from "./approvalMatrix"
import {
    IApplication,
    IRegistryVoter,
    applicantData,
    applicantName,
    candidateFilters,
    comparable,
    compareWithVoter,
    decidedBy,
    decisionDetails,
    differenceText,
    differingFields,
    displayValue,
    enrollmentSummary,
    initials,
    isAlreadyEnrolled,
    joinList,
    listAnnotation,
    rankCandidates,
    sameValue,
    searchFilters,
    voterName,
    voterValue,
    waitingTime,
} from "./approvalReview"

const FIELDS = ["firstName", "middleName", "lastName", "dateOfBirth", "embassy"]

const decision = (
    fields: Record<string, string>,
    extra: Record<string, unknown> = {},
    inputs: Record<string, unknown> = {}
) => ({
    matrix_version: 1,
    rule: 5,
    conditions: {differing: "exactly_1", fields: {embassy: "MATCHES"}},
    inputs: {identity: "VERIFIED", voter_found: true, already_enrolled: false, fields, ...inputs},
    ...extra,
})

const application = (overrides: Partial<IApplication> = {}): IApplication => ({
    status: IApplicationsStatus.PENDING,
    created_at: "2028-04-13T03:41:00Z",
    applicant_data: {
        "firstName": "JUAN CARLOS",
        "middleName": "",
        "lastName": "DELA CRUZ",
        "dateOfBirth": "1990-01-01",
        "embassy": "Tokyo PE",
        "email": "jc.delacruz@example.com",
        "sequent.read-only.id-card-type": "philippinePassport",
    },
    annotations: {
        "search-attributes": "firstName, middleName,lastName,dateOfBirth,embassy",
        "unset-attributes": "email",
        "decision": decision({
            firstName: "DIFFERS",
            middleName: "MATCHES",
            lastName: "MATCHES",
            dateOfBirth: "MATCHES",
            embassy: "MATCHES",
        }),
    },
    ...overrides,
})

const voter = (overrides: Partial<IRegistryVoter> = {}): IRegistryVoter => ({
    id: "voter-1",
    username: "juan.delacruz",
    first_name: "Juan",
    last_name: "Dela Cruz",
    email: null,
    attributes: {dateOfBirth: ["1990-01-01"], embassy: ["Tokyo PE"]},
    ...overrides,
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

describe("the applicant", () => {
    it("is named by the names on the enrollment", () => {
        expect(applicantName(application())).toBe("JUAN CARLOS DELA CRUZ")
        expect(
            applicantName({applicant_data: {firstName: "Ana", middleName: "M", lastName: "Li"}})
        ).toBe("Ana M Li")
        expect(applicantName({applicant_data: null})).toBe("")
    })

    it("reads answers as text", () => {
        expect(applicantData({applicant_data: {age: 34, name: " Ana ", extra: {a: 1}}})).toEqual({
            age: "34",
            name: "Ana",
            extra: "",
        })
        expect(applicantData({applicant_data: ["Ana"]})).toEqual({})
    })

    it("has the initials of the first and last word", () => {
        expect(initials("Juan Carlos Dela Cruz")).toBe("JC")
        expect(initials("ana")).toBe("A")
        expect(initials("  ")).toBe("")
    })

    it("lists a comma separated annotation", () => {
        expect(listAnnotation(application(), "search-attributes")).toEqual(FIELDS)
        expect(listAnnotation(application(), "missing")).toEqual([])
        expect(listAnnotation({annotations: null}, "search-attributes")).toEqual([])
    })
})

describe("the decision record", () => {
    it("is read from the annotations", () => {
        const details = decisionDetails(application())
        expect(details).toEqual({
            matrixVersion: 1,
            rule: 5,
            conditions: {differing: "exactly_1", fields: {embassy: "MATCHES"}},
            identity: EIdentityMethod.VERIFIED,
            voterFound: true,
            alreadyEnrolled: false,
            approvedVoters: 0,
            fields: {
                firstName: EFieldMatch.DIFFERS,
                middleName: EFieldMatch.MATCHES,
                lastName: EFieldMatch.MATCHES,
                dateOfBirth: EFieldMatch.MATCHES,
                embassy: EFieldMatch.MATCHES,
            },
        })
        expect(details && differingFields(details)).toEqual(["firstName"])
    })

    it("reads the last rule, a typed identity and unknown values", () => {
        const details = decisionDetails({
            annotations: {
                decision: {
                    matrix_version: 2,
                    rule: null,
                    accepted_candidates: 2,
                    inputs: {identity: "MANUAL_ENTRY", fields: {firstName: "SIMILAR"}},
                },
            },
        })
        expect(details).toEqual({
            matrixVersion: 2,
            rule: null,
            conditions: {},
            identity: EIdentityMethod.MANUAL_ENTRY,
            voterFound: false,
            alreadyEnrolled: false,
            approvedVoters: 2,
            fields: {},
        })
        expect(
            decisionDetails({annotations: {decision: {matrix_version: 1, inputs: {identity: "X"}}}})
                ?.identity
        ).toBeNull()
    })

    it.each([{}, {annotations: {}}, {annotations: {decision: {rule: 1}}}, {annotations: "x"}])(
        "has none for %j",
        (value) => {
            expect(decisionDetails(value)).toBeNull()
        }
    )

    it("names the officer who decided by hand", () => {
        expect(decidedBy({annotations: {verified_by: "ana"}})).toBe("ana")
        expect(decidedBy(application())).toBe("")
    })
})

describe("what happened", () => {
    const summary = (value: IApplication) => enrollmentSummary(value, t, humanizeField)

    it("says which details differ from the registry", () => {
        expect(summary(application())).toEqual({
            headline: "First name differs from the registry",
            detail: "ID scan verified",
        })
        const two = application({
            annotations: {decision: decision({dateOfBirth: "DIFFERS", embassy: "DIFFERS"})},
        })
        expect(summary(two).headline).toBe("Date of birth and embassy differ from the registry")
    })

    it("says that the details were typed by hand", () => {
        const typed = application({
            annotations: {
                decision: decision({firstName: "MATCHES"}, {}, {identity: "MANUAL_ENTRY"}),
            },
        })
        expect(summary(typed)).toEqual({
            headline: "Details typed by hand, not read from an ID scan",
            detail: "Needs a face-to-face check",
        })
    })

    it("says that no voter was found, or that everything matches", () => {
        const none = application({
            annotations: {decision: decision({}, {}, {voter_found: false, identity: null})},
        })
        expect(summary(none)).toEqual({headline: "No voter found in the registry", detail: ""})
        const all = application({annotations: {decision: decision({firstName: "MATCHES"})}})
        expect(summary(all).headline).toBe("All details match the registry")
    })

    it("falls back for an application without a decision record", () => {
        expect(summary(application({annotations: {}}))).toEqual({
            headline: "Waiting for a person to decide",
            detail: "",
        })
    })

    it("says who approved or rejected", () => {
        const accepted = application({status: IApplicationsStatus.ACCEPTED})
        expect(summary(accepted)).toEqual({
            headline: "Approved automatically",
            detail: "First name differs from the registry",
        })
        expect(summary({...accepted, annotations: {verified_by: "ana"}}).headline).toBe(
            "Approved by ana"
        )
        const rejected = application({status: IApplicationsStatus.REJECTED})
        expect(summary(rejected).headline).toBe("Rejected automatically")
        expect(summary({...rejected, annotations: {verified_by: "ana"}})).toEqual({
            headline: "Rejected by ana",
            detail: "",
        })
    })

    it("joins lists with a last and", () => {
        expect(joinList([], t)).toBe("")
        expect(joinList(["a"], t)).toBe("a")
        expect(joinList(["a", "b", "c"], t)).toBe("a, b and c")
    })
})

describe("waitingTime", () => {
    const now = new Date("2028-04-13T06:41:00Z")

    it.each([
        ["2028-04-13T06:40:50Z", "1 minute"],
        ["2028-04-13T06:11:00Z", "30 minutes"],
        ["2028-04-13T05:41:00Z", "1 hour"],
        ["2028-04-13T03:41:00Z", "3 hours"],
        ["2028-04-12T06:41:00Z", "1 day"],
        ["2028-04-04T03:00:00Z", "9 days"],
        ["2028-04-14T03:00:00Z", "1 minute"],
    ])("of %s is %s", (createdAt, expected) => {
        expect(waitingTime(createdAt, now, t)).toBe(expected)
    })

    it("is empty without a date", () => {
        expect(waitingTime(null, now, t)).toBe("")
        expect(waitingTime("soon", now, t)).toBe("")
    })
})

describe("comparing with a registry voter", () => {
    it("ignores capital letters, accents, hyphens and periods", () => {
        expect(comparable("  Peña-Nieto  Jr. ")).toBe("pena nieto jr")
        expect(sameValue("DELA CRUZ", "Dela  Cruz")).toBe(true)
        expect(sameValue("Juan Carlos", "Juan")).toBe(false)
    })

    it("reads a voter's names and attributes", () => {
        expect(voterValue(voter(), "firstName")).toBe("Juan")
        expect(voterValue(voter(), "embassy")).toBe("Tokyo PE")
        expect(voterValue(voter({attributes: {embassy: "Dubai"}}), "embassy")).toBe("Dubai")
        expect(voterValue(voter({attributes: null}), "embassy")).toBe("")
        expect(voterValue(voter(), "email")).toBe("")
    })

    it("compares each field", () => {
        expect(compareWithVoter(application(), voter(), FIELDS)).toEqual([
            {fields: ["firstName"], enrollment: "JUAN CARLOS", registry: "Juan", same: false},
            {fields: ["middleName"], enrollment: "", registry: "", same: true},
            {fields: ["lastName"], enrollment: "DELA CRUZ", registry: "Dela Cruz", same: true},
            {fields: ["dateOfBirth"], enrollment: "1990-01-01", registry: "1990-01-01", same: true},
            {fields: ["embassy"], enrollment: "Tokyo PE", registry: "Tokyo PE", same: true},
        ])
    })

    it("compares first and middle name together for a driver's license", () => {
        const license = application({
            applicant_data: {
                "firstName": "Juan",
                "middleName": "Carlos",
                "lastName": "Cruz",
                "sequent.read-only.id-card-type": "driversLicense",
            },
        })
        const registered = voter({
            first_name: "Juan Carlos",
            last_name: "Cruz",
            attributes: {middleName: [""]},
        })
        expect(
            compareWithVoter(license, registered, ["firstName", "middleName", "lastName"])
        ).toEqual([
            {
                fields: ["firstName", "middleName"],
                enrollment: "Juan Carlos",
                registry: "Juan Carlos",
                same: true,
            },
            {fields: ["lastName"], enrollment: "Cruz", registry: "Cruz", same: true},
        ])
        expect(compareWithVoter(license, registered, ["middleName"])).toEqual([
            {fields: ["middleName"], enrollment: "Carlos", registry: "", same: false},
        ])
    })

    it("knows an enrolled voter by the attributes enrolling sets", () => {
        expect(isAlreadyEnrolled(voter(), ["email"])).toBe(false)
        expect(isAlreadyEnrolled(voter({email: "juan@example.com"}), ["email"])).toBe(true)
        expect(isAlreadyEnrolled(voter({email: "juan@example.com"}), [])).toBe(false)
    })

    it("ranks the voters, the closest first, the enrolled last and each once", () => {
        const far = voter({id: "voter-2", first_name: "Juan Santos", attributes: {}})
        const enrolled = voter({id: "voter-3", first_name: "JUAN CARLOS", email: "j@example.com"})
        const ranked = rankCandidates(
            application(),
            [far, voter(), voter(), enrolled, voter({id: null})],
            FIELDS,
            ["email"]
        )
        expect(ranked.map((candidate) => [candidate.voter.id, candidate.matching])).toEqual([
            ["voter-1", 4],
            ["voter-2", 2],
            ["voter-3", 5],
        ])
        expect(ranked.map((candidate) => candidate.alreadyEnrolled)).toEqual([false, false, true])
    })

    it("names a voter", () => {
        expect(voterName(voter({attributes: {middleName: ["Santos"]}}))).toBe(
            "Juan Santos Dela Cruz"
        )
        expect(voterName({username: "voter7"})).toBe("voter7")
    })

    it("says how a detail differs", () => {
        const [first] = compareWithVoter(application(), voter(), FIELDS)
        expect(differenceText(first, t, humanizeField)).toBe(
            "the first name is “JUAN CARLOS” on the enrollment and “Juan” in the registry"
        )
        expect(
            differenceText(
                {fields: ["firstName", "middleName"], enrollment: "", registry: "Ana", same: false},
                t,
                humanizeField
            )
        ).toBe("the first name and middle name is “—” on the enrollment and “Ana” in the registry")
    })
})

describe("displayValue", () => {
    it("writes a date the way people read it", () => {
        expect(displayValue("1990-01-01", "en-GB")).toBe("1 Jan 1990")
        expect(displayValue("1988-03-12", "en-GB")).toBe("12 Mar 1988")
    })

    it("keeps other values, and marks a missing one", () => {
        expect(displayValue("Tokyo PE", "en-GB")).toBe("Tokyo PE")
        expect(displayValue("1990-13-45", "en-GB")).toBe("1990-13-45")
        expect(displayValue("", "en-GB")).toBe("—")
    })
})

describe("registry lookups", () => {
    it("look for every compared detail, and for all but one", () => {
        const filters = candidateFilters(application(), FIELDS)
        expect(filters).toHaveLength(5)
        expect(filters[0]).toEqual({
            first_name: {IsLike: "JUAN CARLOS"},
            last_name: {IsLike: "DELA CRUZ"},
            attributes: {dateOfBirth: "1990-01-01", embassy: "Tokyo PE"},
        })
        expect(filters[1]).toEqual({
            last_name: {IsLike: "DELA CRUZ"},
            attributes: {dateOfBirth: "1990-01-01", embassy: "Tokyo PE"},
        })
        expect(filters[4]).toEqual({
            first_name: {IsLike: "JUAN CARLOS"},
            last_name: {IsLike: "DELA CRUZ"},
            attributes: {dateOfBirth: "1990-01-01"},
        })
    })

    it("skip the lookup without any detail", () => {
        expect(candidateFilters(application(), ["firstName"])).toEqual([
            {first_name: {IsLike: "JUAN CARLOS"}},
        ])
        expect(candidateFilters({applicant_data: {}}, FIELDS)).toEqual([])
    })

    it("search by first name, last name and email", () => {
        expect(searchFilters("  ")).toEqual([])
        expect(searchFilters("cruz")).toEqual([
            {first_name: {IsLike: "cruz"}},
            {last_name: {IsLike: "cruz"}},
            {email: {IsLike: "cruz"}},
        ])
        expect(searchFilters("Juan Dela Cruz").slice(3)).toEqual([
            {first_name: {IsLike: "Juan Dela"}, last_name: {IsLike: "Cruz"}},
            {first_name: {IsLike: "Juan"}, last_name: {IsLike: "Dela Cruz"}},
        ])
    })
})
