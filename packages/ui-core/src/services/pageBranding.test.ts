// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    EPlatformBrandingPolicy,
    IElectionEventPresentation,
} from "../types/ElectionEventPresentation"
import {eventLogo, hidesPlatformBranding, pageHeadLinks} from "./pageBranding"

const presentationWith = (fields: Partial<IElectionEventPresentation>) =>
    fields as IElectionEventPresentation

const hidden = {platform_branding_policy: EPlatformBrandingPolicy.HIDE}

describe("the platform branding policy", () => {
    test("shows the platform unless the event hides it", () => {
        expect(hidesPlatformBranding(undefined)).toBe(false)
        expect(hidesPlatformBranding(presentationWith({}))).toBe(false)
        expect(hidesPlatformBranding(presentationWith({platform_branding_policy: null}))).toBe(
            false
        )
        expect(
            hidesPlatformBranding(
                presentationWith({platform_branding_policy: EPlatformBrandingPolicy.SHOW})
            )
        ).toBe(false)
        expect(
            hidesPlatformBranding(presentationWith({platform_branding_policy: "a-newer-policy"}))
        ).toBe(false)
        expect(hidesPlatformBranding(presentationWith(hidden))).toBe(true)
    })
})

describe("the header logo", () => {
    const defaults = {platform: "platform-logo", blank: "blank-logo"}

    test("is the event's own when it has one", () => {
        expect(
            eventLogo(presentationWith({...hidden, logo_url: "https://event/logo.svg"}), defaults)
        ).toBe("https://event/logo.svg")
    })

    test("is blank while the event is unknown or has not said", () => {
        expect(eventLogo(undefined, defaults)).toBe("blank-logo")
        expect(eventLogo(presentationWith({}), defaults)).toBe("blank-logo")
    })

    test("is the platform's when the event has none, unless the platform is hidden", () => {
        const noLogo = {logo_url: null} as unknown as Partial<IElectionEventPresentation>
        expect(eventLogo(presentationWith(noLogo), defaults)).toBe("platform-logo")
        expect(eventLogo(presentationWith({...noLogo, ...hidden}), defaults)).toBe("blank-logo")
    })
})

describe("the browser tab", () => {
    const hrefs = (presentation?: IElectionEventPresentation) =>
        pageHeadLinks(presentation).map((link) => link.href)

    test("has no icon while the event is unknown", () => {
        expect(hrefs(undefined)).toEqual(["data:,"])
    })

    test("has the platform's icons and manifest by default", () => {
        const links = pageHeadLinks(presentationWith({}))
        expect(links.map((link) => link.rel)).toEqual([
            "icon",
            "icon",
            "shortcut icon",
            "apple-touch-icon",
            "manifest",
        ])
    })

    test("has only the event's icon when it sets one", () => {
        expect(hrefs(presentationWith({favicon_url: " https://event/icon.png "}))).toEqual([
            "https://event/icon.png",
        ])
        expect(hrefs(presentationWith({...hidden, favicon_url: "/icon.png"}))).toEqual([
            "/icon.png",
        ])
    })

    test("has no icon when the platform is hidden and the event sets none", () => {
        expect(hrefs(presentationWith(hidden))).toEqual(["data:,"])
        expect(hrefs(presentationWith({...hidden, favicon_url: "  "}))).toEqual(["data:,"])
    })

    test("ignores an icon address that is not a web address", () => {
        expect(hrefs(presentationWith({...hidden, favicon_url: "javascript:alert(1)"}))).toEqual([
            "data:,",
        ])
        expect(
            pageHeadLinks(presentationWith({favicon_url: "data:image/png;base64,AAAA"}))
        ).toHaveLength(5)
    })
})
