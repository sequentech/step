// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {
    EMobileCandidateLists,
    ESlateProblemSeverity,
    ISlate,
    ISlateProblem,
    ISlatesConfig,
} from "@sequentech/ui-core"
import {canonicalize_slates_js, check_election_slates_js} from "sequent-core"
import {
    applySlatesSettings,
    checkSlatesConfiguration,
    formatSlatesProblems,
    hasSlatesAnnotation,
    isEligibleSlateMember,
    putSlate,
    readSlatesConfig,
    removeSlate,
    slateCandidateChoices,
    slateFormValues,
    slateFromForm,
    slatesProblemsFromError,
    writeSlatesConfig,
} from "./slates"

jest.mock("@sequentech/ui-core", () => ({
    SLATES_ANNOTATION: "sequent.slates",
    EMobileCandidateLists: {COLLAPSED: "collapsed", EXPANDED: "expanded"},
    ESlateProblemSeverity: {ERROR: "error"},
    getSlateName: (slate: {name: Record<string, string>}, language: string) =>
        slate.name[language] ?? Object.values(slate.name)[0] ?? "",
}))
jest.mock("sequent-core", () => ({
    canonicalize_slates_js: jest.fn(),
    check_election_slates_js: jest.fn(),
}))

const canonicalize = jest.mocked(canonicalize_slates_js)
const check = jest.mocked(check_election_slates_js)

const unity: ISlate = {
    id: "unity",
    name: {en: "Unity Alliance", es: "Alianza Unidad"},
    members: {president: ["maya"], trustees: ["avery", "dominic"]},
}
const harbor: ISlate = {
    id: "harbor",
    name: {en: "Harbor Independents"},
    members: {trustees: ["harper"]},
}
const config: ISlatesConfig = {
    version: 1,
    mobile_candidate_lists: EMobileCandidateLists.COLLAPSED,
    slates: [unity, harbor],
}
const stored = JSON.stringify(config)
const problem: ISlateProblem = {
    severity: ESlateProblemSeverity.ERROR,
    code: "duplicate_id",
    path: "sequent.slates.slates[0].id",
    message: "two slates have the id 'a'",
}

const contests = [
    {id: "president", max_votes: 1},
    {id: "trustees", max_votes: 3},
]

describe("reading the slate configuration", () => {
    it("reads the stored annotation", () => {
        expect(readSlatesConfig({"sequent.slates": stored})).toEqual(config)
        expect(hasSlatesAnnotation({"sequent.slates": stored})).toBe(true)
    })

    it.each([
        ["no annotations", null],
        ["no slate annotation", {color: "blue"}],
        ["an annotation that is not text", {"sequent.slates": config}],
        ["text that is not JSON", {"sequent.slates": "{version:"}],
        ["a configuration without slates", {"sequent.slates": '{"version":1}'}],
        ["a slate without members", {"sequent.slates": '{"version":1,"slates":[{"id":"a"}]}'}],
    ])("reads nothing from %s", (_name, annotations) => {
        expect(readSlatesConfig(annotations)).toBeUndefined()
    })

    it("knows the annotation is there even when it cannot be read", () => {
        expect(hasSlatesAnnotation({"sequent.slates": "{version:"})).toBe(true)
        expect(hasSlatesAnnotation({color: "blue"})).toBe(false)
        expect(hasSlatesAnnotation(undefined)).toBe(false)
    })
})

describe("writing the slate configuration", () => {
    afterEach(() => jest.resetAllMocks())

    it("stores the canonical form and keeps the other annotations", () => {
        canonicalize.mockReturnValue("canonical")
        const annotations = {"color": "blue", "sequent.slates": "old"}

        expect(writeSlatesConfig(annotations, config)).toEqual({
            "color": "blue",
            "sequent.slates": "canonical",
        })
        expect(canonicalize).toHaveBeenCalledWith(stored)
        expect(annotations["sequent.slates"]).toBe("old")
    })

    it("removes the annotation and nothing else when there is no configuration", () => {
        expect(writeSlatesConfig({"color": "blue", "sequent.slates": stored}, undefined)).toEqual({
            color: "blue",
        })
        expect(canonicalize).not.toHaveBeenCalled()
    })

    it("does not store a configuration that cannot be read", () => {
        canonicalize.mockImplementation(() => {
            throw [problem]
        })
        expect(() => writeSlatesConfig({}, config)).toThrow()
    })
})

describe("the Ballot Design settings", () => {
    beforeEach(() => canonicalize.mockImplementation((text: string) => text))
    afterEach(() => jest.resetAllMocks())

    it("removes the slates when they are turned off", () => {
        expect(
            applySlatesSettings(
                {"color": "blue", "sequent.slates": stored},
                {enabled: false, mobileCandidateLists: EMobileCandidateLists.COLLAPSED}
            )
        ).toEqual({color: "blue"})
    })

    it("starts an empty configuration when they are turned on", () => {
        const annotations = applySlatesSettings(
            {color: "blue"},
            {enabled: true, mobileCandidateLists: EMobileCandidateLists.EXPANDED}
        )
        expect(annotations.color).toBe("blue")
        expect(JSON.parse(String(annotations["sequent.slates"]))).toEqual({
            version: 1,
            mobile_candidate_lists: "expanded",
            slates: [],
        })
    })

    it("changes the mobile candidate lists and keeps the slates", () => {
        const annotations = applySlatesSettings(
            {"sequent.slates": stored},
            {enabled: true, mobileCandidateLists: EMobileCandidateLists.EXPANDED}
        )
        expect(JSON.parse(String(annotations["sequent.slates"]))).toEqual({
            ...config,
            mobile_candidate_lists: "expanded",
        })
    })

    it("leaves a configuration it cannot read as it is", () => {
        const annotations = {"sequent.slates": "{version:"}
        expect(
            applySlatesSettings(annotations, {
                enabled: true,
                mobileCandidateLists: EMobileCandidateLists.EXPANDED,
            })
        ).toEqual(annotations)
        expect(canonicalize).not.toHaveBeenCalled()
    })
})

describe("editing slates", () => {
    it("replaces a slate in its place", () => {
        const renamed = {...unity, name: {en: "Unity"}}
        expect(putSlate(config, renamed).slates).toEqual([renamed, harbor])
    })

    it("appends a new slate", () => {
        const added = {id: "new", name: {en: "New"}, members: {president: ["lena"]}}
        expect(putSlate(config, added).slates).toEqual([unity, harbor, added])
        expect(config.slates).toHaveLength(2)
    })

    it("removes a slate", () => {
        expect(removeSlate(config, "unity").slates).toEqual([harbor])
        expect(removeSlate(config, "missing").slates).toEqual([unity, harbor])
    })
})

describe("slate members", () => {
    const candidate = (id: string, contestId: string, presentation = {}) => ({
        id,
        contest_id: contestId,
        presentation,
    })

    it.each([
        ["disabled", {is_disabled: true}],
        ["a write-in", {is_write_in: true}],
        ["the explicit blank option", {is_explicit_blank: true}],
        ["the explicit invalid option", {is_explicit_invalid: true}],
        ["a category list", {is_category_list: true}],
    ])("does not offer a candidate that is %s", (_name, presentation) => {
        expect(isEligibleSlateMember(candidate("x", "president", presentation))).toBe(false)
    })

    it("offers an ordinary candidate", () => {
        expect(isEligibleSlateMember(candidate("x", "president"))).toBe(true)
        expect(isEligibleSlateMember({id: "y", contest_id: "president", presentation: null})).toBe(
            true
        )
    })

    it("offers the contest's candidates that are on no other slate", () => {
        const candidates = [
            candidate("avery", "trustees"),
            candidate("harper", "trustees"),
            candidate("kim", "trustees"),
            candidate("maya", "president"),
            candidate("blank", "trustees", {is_explicit_blank: true}),
        ]
        expect(
            slateCandidateChoices(candidates, "trustees", config, "unity").map(({id}) => id)
        ).toEqual(["avery", "kim"])
        expect(
            slateCandidateChoices(candidates, "trustees", config, undefined).map(({id}) => id)
        ).toEqual(["kim"])
    })
})

describe("the slate form", () => {
    it("has one picker per seat of each contest, filled with the slate's members", () => {
        expect(slateFormValues(unity, contests, "en")).toEqual({
            name: "Unity Alliance",
            alias: "",
            description: "",
            external_id: "",
            members: {president: ["maya"], trustees: ["avery", "dominic", ""]},
        })
    })

    it("has empty pickers for a new slate", () => {
        expect(slateFormValues(undefined, contests, "en").members).toEqual({
            president: [""],
            trustees: ["", "", ""],
        })
    })

    it("keeps members beyond the maximum so they can be removed", () => {
        const crowded = {...unity, members: {president: ["maya", "lena"]}}
        expect(slateFormValues(crowded, contests, "en").members.president).toEqual(["maya", "lena"])
    })

    it("builds the slate from the form", () => {
        expect(
            slateFromForm(
                "unity",
                {
                    name: " Unity ",
                    alias: "U",
                    description: "Officers\nand trustees",
                    external_id: " slate-unity-2026 ",
                    members: {president: [""], trustees: ["avery", "", "avery", "kim"]},
                },
                "en",
                unity
            )
        ).toEqual({
            id: "unity",
            name: {en: "Unity", es: "Alianza Unidad"},
            alias: "U",
            description: "Officers\nand trustees",
            external_id: "slate-unity-2026",
            members: {trustees: ["avery", "kim"]},
        })
    })

    it("leaves out the optional fields that are empty", () => {
        const slate = slateFromForm(
            "new",
            {
                name: "New",
                alias: " ",
                description: "",
                external_id: "",
                members: {president: ["maya"]},
            },
            "es",
            undefined
        )
        expect(slate).toEqual({id: "new", name: {es: "New"}, members: {president: ["maya"]}})
    })
})

describe("checking slates", () => {
    afterEach(() => jest.resetAllMocks())

    it("checks the configuration against the election's contests and candidates", () => {
        check.mockReturnValue([problem])
        const electionContests = [{id: "contest"}] as never
        const candidates = [{id: "candidate"}] as never

        expect(checkSlatesConfiguration(stored, "en", electionContests, candidates)).toEqual([
            problem,
        ])
        expect(check).toHaveBeenCalledWith(
            stored,
            "en",
            JSON.stringify(electionContests),
            JSON.stringify(candidates)
        )
    })

    it.each([[{unexpected: true}], [[{...problem, severity: "notice"}]]])(
        "refuses an answer that is not a list of problems: %p",
        (answer) => {
            check.mockReturnValue(answer)
            expect(() => checkSlatesConfiguration(stored, "en", [], [])).toThrow()
        }
    )

    it("recognises thrown problems and formats them one per line", () => {
        expect(slatesProblemsFromError([problem])).toEqual([problem])
        expect(slatesProblemsFromError(new Error("boom"))).toBeUndefined()
        expect(
            formatSlatesProblems([problem, {...problem, path: "sequent.slates", message: "second"}])
        ).toBe("sequent.slates.slates[0].id: two slates have the id 'a'\nsequent.slates: second")
    })
})
