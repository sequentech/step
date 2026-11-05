// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {EMobileCandidateLists} from "@sequentech/ui-core"
import {canonicalize_slates_js, check_election_slates_js} from "sequent-core"
import {
    checkSlatesConfiguration,
    checkSlatesStructure,
    formatSlatesProblems,
    getMobileCandidateLists,
    readSlatesConfiguration,
    setMobileCandidateLists,
    slatesProblemsFromError,
    writeSlatesConfiguration,
} from "./slates"

jest.mock("@sequentech/ui-core", () => ({
    SLATES_ANNOTATION: "sequent.slates",
    EMobileCandidateLists: {COLLAPSED: "collapsed", EXPANDED: "expanded"},
}))
jest.mock("sequent-core", () => ({
    canonicalize_slates_js: jest.fn(),
    check_election_slates_js: jest.fn(),
}))

const canonicalize = jest.mocked(canonicalize_slates_js)
const check = jest.mocked(check_election_slates_js)

const slates = {
    version: 1,
    slates: [{id: "forward", name: {en: "Forward"}, members: {contest: ["candidate"]}}],
}
const stored = JSON.stringify(slates)
const problem = {
    severity: "error",
    code: "duplicate_id",
    path: "sequent.slates.slates[0].id",
    message: "two slates have the id 'a'",
}

describe("slate configuration", () => {
    afterEach(() => jest.resetAllMocks())

    it("reads the stored annotation as editable text", () => {
        expect(readSlatesConfiguration({"sequent.slates": stored})).toBe(
            JSON.stringify(slates, null, 2)
        )
    })

    it.each([
        ["no annotations", null, ""],
        ["no slate annotation", {color: "blue"}, ""],
        ["an annotation that is not text", {"sequent.slates": slates}, ""],
        ["text that is not JSON", {"sequent.slates": "{version:"}, "{version:"],
    ])("reads %s", (_name, annotations, expected) => {
        expect(readSlatesConfiguration(annotations)).toBe(expected)
    })

    it("stores the canonical form and keeps the other annotations", () => {
        canonicalize.mockReturnValue(stored)
        const annotations = {"color": "blue", "sequent.slates": "old"}

        expect(writeSlatesConfiguration(annotations, JSON.stringify(slates, null, 2))).toEqual({
            "color": "blue",
            "sequent.slates": stored,
        })
        expect(annotations["sequent.slates"]).toBe("old")
    })

    it.each([[""], ["  \n"], [null], [undefined]])(
        "removes the annotation when the field is %p",
        (text) => {
            expect(
                writeSlatesConfiguration({"color": "blue", "sequent.slates": stored}, text)
            ).toEqual({color: "blue"})
            expect(canonicalize).not.toHaveBeenCalled()
        }
    )

    it("does not store a configuration that cannot be read", () => {
        canonicalize.mockImplementation(() => {
            throw [problem]
        })
        expect(() => writeSlatesConfiguration({}, "{version:")).toThrow()
    })

    it("reads the mobile candidate lists default", () => {
        expect(getMobileCandidateLists(stored)).toBe(EMobileCandidateLists.COLLAPSED)
        expect(
            getMobileCandidateLists(JSON.stringify({...slates, mobile_candidate_lists: "expanded"}))
        ).toBe(EMobileCandidateLists.EXPANDED)
    })

    it.each([[""], [null], ["{version:"], ["[]"], ['{"mobile_candidate_lists":"sideways"}']])(
        "has no mobile candidate lists default for %p",
        (text) => {
            expect(getMobileCandidateLists(text)).toBeUndefined()
        }
    )

    it("changes the mobile candidate lists default and nothing else", () => {
        const changed = setMobileCandidateLists(stored, EMobileCandidateLists.EXPANDED)
        expect(JSON.parse(changed)).toEqual({...slates, mobile_candidate_lists: "expanded"})
        expect(setMobileCandidateLists("{version:", EMobileCandidateLists.EXPANDED)).toBe(
            "{version:"
        )
    })

    it("checks the configuration against the election's contests and candidates", () => {
        check.mockReturnValue([problem])
        const contests = [{id: "contest"}] as never
        const candidates = [{id: "candidate"}] as never

        expect(checkSlatesConfiguration(stored, "en", contests, candidates)).toEqual([problem])
        expect(check).toHaveBeenCalledWith(
            stored,
            "en",
            JSON.stringify(contests),
            JSON.stringify(candidates)
        )
    })

    it("refuses an answer that is not a list of problems", () => {
        check.mockReturnValue({unexpected: true})
        expect(() => checkSlatesConfiguration(stored, "en", [], [])).toThrow()
    })

    it("checks the structure alone when the election cannot be loaded whole", () => {
        canonicalize.mockReturnValue(stored)
        expect(checkSlatesStructure(stored)).toEqual([])

        canonicalize.mockImplementation(() => {
            throw [problem]
        })
        expect(checkSlatesStructure("{version:")).toEqual([problem])

        canonicalize.mockImplementation(() => {
            throw new Error("boom")
        })
        expect(() => checkSlatesStructure(stored)).toThrow("boom")
    })

    it("recognises thrown problems and formats them one per line", () => {
        expect(slatesProblemsFromError([problem])).toEqual([problem])
        expect(slatesProblemsFromError(new Error("boom"))).toBeUndefined()
        expect(
            formatSlatesProblems([problem, {...problem, path: "sequent.slates", message: "second"}])
        ).toBe("sequent.slates.slates[0].id: two slates have the id 'a'\nsequent.slates: second")
    })
})
