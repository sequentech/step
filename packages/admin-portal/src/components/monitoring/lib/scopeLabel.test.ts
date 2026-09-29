// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {EScopeSelector, type MonitoringScopeOptions} from "../types"
import {scopeLabel, selectorWords, type Translate} from "./scopeLabel"

const words: Record<string, string> = {
    "monitoring.selectors.region": "Region",
    "monitoring.selectors.post": "Post",
    "monitoring.selectors.country": "Country",
    "monitoring.selectors.allRegions": "All regions",
    "monitoring.selectors.allPosts": "All Posts",
    "monitoring.selectors.allAuthorizedPosts": "All authorized Posts",
    "monitoring.selectors.allCountries": "All countries",
}
const t: Translate = (key, options) =>
    key === "monitoring.selectors.authorizedOnly"
        ? `${options?.all} (authorized)`
        : (words[key] ?? key)

const options: MonitoringScopeOptions = {
    regions: [{key: "north", label: "North"}],
    posts: [{key: "p1", label: "Madrid", region: "north"}],
    countries: [{key: "ES", label: "Spain"}],
}

describe("selectorWords", () => {
    it("uses the portal's words when the settings name none", () => {
        expect(selectorWords(EScopeSelector.REGION, undefined, t, false)).toEqual({
            label: "Region",
            all: "All regions",
        })
    })

    it("uses the settings' words", () => {
        const settings = {
            time_zone: "UTC",
            selectors: {region: {label: "Faculty", all: "All faculties"}},
        }
        expect(selectorWords(EScopeSelector.REGION, settings, t, false)).toEqual({
            label: "Faculty",
            all: "All faculties",
        })
    })

    it("tells a restricted viewer that All means the Posts they may see", () => {
        expect(selectorWords(EScopeSelector.POST, undefined, t, false).all).toBe("All Posts")
        expect(selectorWords(EScopeSelector.POST, undefined, t, true).all).toBe(
            "All authorized Posts"
        )
        const settings = {time_zone: "UTC", selectors: {post: {label: "Ward", all: "All wards"}}}
        expect(selectorWords(EScopeSelector.POST, settings, t, true).all).toBe(
            "All wards (authorized)"
        )
    })
})

describe("scopeLabel", () => {
    const selectors = [EScopeSelector.REGION, EScopeSelector.POST, EScopeSelector.COUNTRY]

    it("names every dashboard selector's choice", () => {
        expect(scopeLabel({scope: {}, selectors, options, t, restricted: true})).toBe(
            "All regions · All authorized Posts · All countries"
        )
        expect(
            scopeLabel({
                scope: {region: "north", post: "p1", country: "ES"},
                selectors,
                options,
                t,
                restricted: false,
            })
        ).toBe("North · Madrid · Spain")
    })

    it("names the pinned Post, and a key it has no label for as it is", () => {
        expect(
            scopeLabel({
                scope: {country: "FR"},
                selectors,
                options,
                t,
                restricted: false,
                pinnedPost: "p1",
            })
        ).toBe("All regions · Madrid · FR")
    })
})
