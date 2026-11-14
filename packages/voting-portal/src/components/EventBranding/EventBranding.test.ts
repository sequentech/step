// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    applyPageBranding,
    EPlatformBrandingPolicy,
    IElectionEventPresentation,
} from "@sequentech/ui-core"
import {eventLogo, eventStylesClassName} from "./EventBranding"

jest.mock("@sequentech/ui-essentials/public/Sequent_logo.svg", () => "platform-logo", {
    virtual: true,
})
jest.mock("@sequentech/ui-essentials/public/blank_logo.svg", () => "blank-logo", {virtual: true})

const presentationWith = (fields: Partial<IElectionEventPresentation>) =>
    fields as IElectionEventPresentation

const hidden = {platform_branding_policy: EPlatformBrandingPolicy.HIDE}

describe("the platform branding policy", () => {
    test("marks the page wrapper only when the platform is hidden", () => {
        expect(eventStylesClassName(presentationWith({}))).toBe("voting-portal-wrapper")
        expect(eventStylesClassName(presentationWith(hidden)).split(" ")).toEqual([
            "voting-portal-wrapper",
            "platform-branding-hidden",
        ])
    })
})

describe("the header logo", () => {
    test("is the event's own when it has one", () => {
        expect(eventLogo(presentationWith({logo_url: "https://event/logo.svg"}))).toBe(
            "https://event/logo.svg"
        )
        expect(eventLogo(presentationWith({...hidden, logo_url: "https://event/logo.svg"}))).toBe(
            "https://event/logo.svg"
        )
    })

    test("is blank while the event is unknown or has not said", () => {
        expect(eventLogo(undefined)).toBe("blank-logo")
        expect(eventLogo(presentationWith({}))).toBe("blank-logo")
    })

    test("is the platform's when the event has none, unless the platform is hidden", () => {
        const noLogo = {logo_url: null} as unknown as Partial<IElectionEventPresentation>
        expect(eventLogo(presentationWith(noLogo))).toBe("platform-logo")
        expect(eventLogo(presentationWith({...noLogo, ...hidden}))).toBe("blank-logo")
    })
})

describe("the browser tab", () => {
    test("replaces the icons it set before and takes the event's title", () => {
        document.head.innerHTML =
            '<link rel="icon" href="data:," data-page-branding><link rel="stylesheet" href="/a.css">'
        applyPageBranding(document, presentationWith({}), "Voting Portal")
        expect(document.title).toBe("Voting Portal")
        expect(document.head.querySelectorAll("link[data-page-branding]")).toHaveLength(5)

        applyPageBranding(
            document,
            presentationWith({
                ...hidden,
                favicon_url: "https://event/icon.png",
                page_title: " Vote ",
            }),
            "Voting Portal"
        )
        expect(document.title).toBe("Vote")
        const icons = document.head.querySelectorAll("link[data-page-branding]")
        expect(icons).toHaveLength(1)
        expect(icons[0].getAttribute("href")).toBe("https://event/icon.png")
        expect(document.head.querySelector('link[rel="stylesheet"]')).not.toBeNull()

        applyPageBranding(document, presentationWith({page_title: "  "}), "Voting Portal")
        expect(document.title).toBe("Voting Portal")
    })
})
