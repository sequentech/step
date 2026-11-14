/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {afterEach, beforeEach, expect, test} from "@jest/globals"
import {cleanup, render} from "@testing-library/react"
import {
    EPlatformBrandingPolicy,
    IElectionEventPresentation,
} from "../types/ElectionEventPresentation"
import {applyPageBranding, usePageBranding} from "./pageBranding"

const presentationWith = (fields: Partial<IElectionEventPresentation>) =>
    fields as IElectionEventPresentation

const hidden = {platform_branding_policy: EPlatformBrandingPolicy.HIDE}

const brandingLinks = () => document.head.querySelectorAll("link[data-page-branding]")

const Page: React.FC<{presentation?: IElectionEventPresentation}> = ({presentation}) => {
    usePageBranding(presentation)
    return null
}

beforeEach(() => {
    document.head.innerHTML =
        '<link rel="icon" href="data:," data-page-branding><link rel="stylesheet" href="/a.css">'
})

afterEach(cleanup)

test("replaces the icons it set before and takes the event's title", () => {
    applyPageBranding(document, presentationWith({}), "Voting Portal")
    expect(document.title).toBe("Voting Portal")
    expect(brandingLinks()).toHaveLength(5)

    applyPageBranding(
        document,
        presentationWith({...hidden, favicon_url: "https://event/icon.png", page_title: " Vote "}),
        "Voting Portal"
    )
    expect(document.title).toBe("Vote")
    expect(brandingLinks()).toHaveLength(1)
    expect(brandingLinks()[0].getAttribute("href")).toBe("https://event/icon.png")
    expect(document.head.querySelector('link[rel="stylesheet"]')).not.toBeNull()

    applyPageBranding(document, presentationWith({page_title: "  "}), "Voting Portal")
    expect(document.title).toBe("Voting Portal")
})

test("a page follows the event as it becomes known", () => {
    const view = render(<Page />)
    expect(brandingLinks()).toHaveLength(1)
    expect(brandingLinks()[0].getAttribute("href")).toBe("data:,")

    view.rerender(<Page presentation={presentationWith({page_title: "Vote"})} />)
    expect(document.title).toBe("Vote")
    expect(brandingLinks()).toHaveLength(5)

    view.rerender(<Page presentation={presentationWith({...hidden, page_title: "Vote"})} />)
    expect(brandingLinks()).toHaveLength(1)
    expect(brandingLinks()[0].getAttribute("href")).toBe("data:,")
})
