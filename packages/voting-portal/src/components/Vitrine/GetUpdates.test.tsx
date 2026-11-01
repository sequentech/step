/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// "Get email updates" (design boards GetUpdates, ConfirmationUpdates and
// PhoneConfirmationUpdates).

import React from "react"
import {render, screen, within} from "@testing-library/react"
import axe from "axe-core"
import * as pure from "@vitrine/verify/pure"
import type {IOfferedHub} from "../../services/vitrine/hubUpdates"
import {votingPortalTranslations} from "../../translations"
import {GetUpdates} from "./GetUpdates"

jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, values?: Record<string, unknown>) =>
            values ? `${key} ${JSON.stringify(values)}` : key,
    }),
}))

const POLICY_ID = `sha256:${"cd".repeat(32)}`
const RECEIPT = "vitrine/receipt-chain/v1\nrecord\n"

const HUB_B: IOfferedHub = {name: "hub-b", party: "Partner Co", site: "https://hub-b.example/"}
const HUB_S: IOfferedHub = {
    name: "sequent-hub",
    party: "Sequent",
    site: "https://hub.sequent.example/#stale",
}
const HUB_C: IOfferedHub = {name: "hub-c", party: "Civic Org", site: "https://hub-c.example/app/"}

const css = (): string =>
    Array.from(document.styleSheets)
        .flatMap((sheet) => Array.from(sheet.cssRules).map((rule) => rule.cssText))
        .join("\n")

const tiles = (): HTMLAnchorElement[] =>
    within(screen.getByRole("list")).getAllByRole("link") as HTMLAnchorElement[]

const names = (): string[] =>
    tiles().map((tile) => tile.querySelector(".get-updates-hub-name")?.textContent ?? "")

// jsdom cannot compute colours, so the contrast rule cannot run here.
const violations = async (container: HTMLElement) =>
    (await axe.run(container, {rules: {"color-contrast": {enabled: false}}})).violations

afterEach(() => {
    jest.restoreAllMocks()
})

describe("the callout", () => {
    it("is a section named by its title, with what the emails say and the hubs under it", () => {
        render(<GetUpdates hubs={[HUB_B, HUB_S]} policyId={POLICY_ID} receipt={RECEIPT} />)
        const section = screen.getByRole("region", {name: "vitrine.updates.title"})
        expect(section.tagName).toBe("SECTION")
        expect(within(section).getByRole("heading", {level: 2}).textContent).toBe(
            "vitrine.updates.title"
        )
        expect(section.querySelector(".get-updates-description")?.textContent).toBe(
            "vitrine.updates.description"
        )
        expect(within(section).getByRole("list", {name: "vitrine.updates.listLabel"})).toBeTruthy()
        expect(section.querySelector(".get-updates-foot")?.textContent).toBe(
            "vitrine.updates.footnote"
        )
    })

    it("shows nothing without hubs", () => {
        const {container} = render(<GetUpdates hubs={[]} policyId={POLICY_ID} receipt={RECEIPT} />)
        expect(container.firstChild).toBeNull()
    })
})

describe("the hub tiles", () => {
    it("is one equal tile for every hub, each with its name and who runs it", () => {
        render(<GetUpdates hubs={[HUB_B, HUB_S, HUB_C]} policyId={POLICY_ID} receipt={RECEIPT} />)
        expect(tiles()).toHaveLength(3)
        expect([...names()].sort()).toEqual(["hub-b", "hub-c", "sequent-hub"])
        const who = Array.from(document.querySelectorAll(".get-updates-hub-who")).map(
            (e) => e.textContent
        )
        expect(who).toEqual(
            expect.arrayContaining([
                'vitrine.updates.runBy {"party":"Partner Co"}',
                'vitrine.updates.runBy {"party":"Sequent"}',
                'vitrine.updates.runBy {"party":"Civic Org"}',
            ])
        )
        // None is marked, ranked or styled apart from the others.
        const classes = new Set(tiles().map((tile) => tile.className))
        expect(classes.size).toBe(1)
        const styles = new Set(
            tiles().map((tile) => {
                const style = getComputedStyle(tile)
                return [style.backgroundColor, style.borderColor, style.fontWeight].join("|")
            })
        )
        expect(styles.size).toBe(1)
        expect(document.querySelector("[data-featured], [aria-current], [data-default]")).toBeNull()
        expect(document.body.textContent).not.toMatch(/independent|recommended|default|featured/i)
        expect(document.querySelector("input[type=radio], [role=radio]")).toBeNull()
    })

    it("opens the hub's own site in a new tab that knows nothing of this one", () => {
        render(<GetUpdates hubs={[HUB_B]} policyId={POLICY_ID} receipt={RECEIPT} />)
        const [tile] = tiles()
        expect(tile.getAttribute("target")).toBe("_blank")
        expect(tile.getAttribute("rel")).toBe("noopener noreferrer")
        expect(tile.getAttribute("aria-label")).toBe(
            'vitrine.updates.hubLabel {"name":"hub-b","party":"Partner Co"}'
        )
    })

    it("puts the policy ID and the receipt in the link's fragment, never in its path or query", () => {
        render(<GetUpdates hubs={[HUB_S]} policyId={POLICY_ID} receipt={RECEIPT} />)
        const href = tiles()[0].getAttribute("href") ?? ""
        const url = new URL(href)
        expect(url.origin + url.pathname).toBe("https://hub.sequent.example/")
        expect(url.search).toBe("")
        const params = new URLSearchParams(url.hash.slice(1))
        expect(params.get("policy")).toBe(POLICY_ID)
        expect(pure.bytes.fromUtf8(pure.encoding.b64urlDecode(params.get("watch") ?? ""))).toBe(
            RECEIPT
        )
        expect(href).not.toContain("stale")
    })

    it("links each hub to its own site", () => {
        render(<GetUpdates hubs={[HUB_B, HUB_S, HUB_C]} policyId={POLICY_ID} receipt={RECEIPT} />)
        const sites = tiles().map((tile) => (tile.getAttribute("href") ?? "").split("#")[0])
        expect([...sites].sort()).toEqual([
            "https://hub-b.example/",
            "https://hub-c.example/app/",
            "https://hub.sequent.example/",
        ])
    })

    it("opens the sites with the policy only, and says so, until there is a receipt", () => {
        const {container} = render(<GetUpdates hubs={[HUB_B]} policyId={POLICY_ID} />)
        const params = new URLSearchParams((tiles()[0].getAttribute("href") ?? "").split("#")[1])
        expect(params.get("policy")).toBe(POLICY_ID)
        expect(params.has("watch")).toBe(false)
        expect(container.querySelector(".get-updates-foot")?.textContent).toBe(
            "vitrine.updates.footnoteNoReceipt"
        )
    })

    it("takes the receipt into the links once it arrives, without moving the tiles", () => {
        const draw = jest.spyOn(Math, "random").mockReturnValue(0)
        const hubs = [HUB_B, HUB_S, HUB_C]
        const {rerender, container} = render(<GetUpdates hubs={hubs} policyId={POLICY_ID} />)
        const before = names()
        expect(tiles().every((tile) => !(tile.getAttribute("href") ?? "").includes("watch="))).toBe(
            true
        )
        draw.mockReturnValue(0.99)
        rerender(<GetUpdates hubs={hubs} policyId={POLICY_ID} receipt={RECEIPT} />)
        expect(names()).toEqual(before)
        expect(tiles().every((tile) => (tile.getAttribute("href") ?? "").includes("watch="))).toBe(
            true
        )
        expect(container.querySelector(".get-updates-foot")?.textContent).toBe(
            "vitrine.updates.footnote"
        )
    })
})

describe("the order", () => {
    const orderWith = (random: number, hubs = [HUB_B, HUB_S, HUB_C]): string[] => {
        jest.spyOn(Math, "random").mockReturnValue(random)
        const view = render(<GetUpdates hubs={hubs} policyId={POLICY_ID} receipt={RECEIPT} />)
        const order = names()
        view.unmount()
        jest.restoreAllMocks()
        return order
    }

    it("is drawn at random, so that any hub can come first", () => {
        // Fisher-Yates with a constant draw: 0 sends the first hub to the end.
        expect(orderWith(0.99)).toEqual(["hub-b", "sequent-hub", "hub-c"])
        expect(orderWith(0)).toEqual(["sequent-hub", "hub-c", "hub-b"])
        expect(orderWith(0.5)).not.toEqual(orderWith(0.99))
    })

    it("does not follow the order of the policy", () => {
        const firsts = new Set<string>()
        for (let n = 0; n < 60; n++) {
            const view = render(<GetUpdates hubs={[HUB_B, HUB_S]} policyId={POLICY_ID} />)
            firsts.add(names()[0])
            view.unmount()
        }
        expect(firsts).toEqual(new Set(["hub-b", "sequent-hub"]))
    })

    it("stays put while the page updates", () => {
        const hubs = [HUB_B, HUB_S, HUB_C]
        const {rerender} = render(<GetUpdates hubs={hubs} policyId={POLICY_ID} />)
        const first = names()
        jest.spyOn(Math, "random").mockReturnValue(0)
        for (let n = 0; n < 5; n++) {
            rerender(<GetUpdates hubs={[...hubs]} policyId={POLICY_ID} receipt={`chain ${n}`} />)
            expect(names()).toEqual(first)
        }
    })

    it("draws again for a different policy, which the screen asks for with a key", () => {
        const draw = jest.spyOn(Math, "random").mockReturnValue(0.99)
        const hubs = [HUB_B, HUB_S, HUB_C]
        const {rerender} = render(<GetUpdates key="one" hubs={hubs} policyId={POLICY_ID} />)
        expect(names()).toEqual(["hub-b", "sequent-hub", "hub-c"])
        draw.mockReturnValue(0)
        rerender(<GetUpdates key="two" hubs={hubs} policyId={POLICY_ID} />)
        expect(names()).toEqual(["sequent-hub", "hub-c", "hub-b"])
    })
})

describe("the layout", () => {
    it("is one column of tiles, for a phone, and a band with the tiles beside the text at 760 px", () => {
        render(<GetUpdates hubs={[HUB_B, HUB_S]} policyId={POLICY_ID} receipt={RECEIPT} />)
        const section = document.querySelector(".get-updates") as HTMLElement
        expect(getComputedStyle(section).containerType).toBe("inline-size")
        expect(getComputedStyle(screen.getByRole("list")).gridTemplateColumns).toBe(
            "minmax(0, 1fr)"
        )
        const rules = css()
        expect(rules).toMatch(/@container \(min-width: 760px\)/)
        expect(rules).toMatch(/grid-auto-flow: column/)
        expect(rules).toMatch(/grid-auto-columns: 220px/)
        expect(rules).toMatch(/grid-template-columns: minmax\(0, 1fr\) auto/)
    })

    it("gives each tile room for a finger", () => {
        render(<GetUpdates hubs={[HUB_B]} policyId={POLICY_ID} receipt={RECEIPT} />)
        expect(getComputedStyle(tiles()[0]).minHeight).toBe("56px")
    })
})

describe("the accessibility", () => {
    it("has no axe violations, with or without a receipt", async () => {
        const hubs = [HUB_B, HUB_S, HUB_C]
        const withReceipt = render(
            <GetUpdates hubs={hubs} policyId={POLICY_ID} receipt={RECEIPT} />
        )
        expect(await violations(withReceipt.container)).toEqual([])
        withReceipt.unmount()
        const without = render(<GetUpdates hubs={[HUB_B]} policyId={POLICY_ID} />)
        expect(await violations(without.container)).toEqual([])
    })

    it("keeps the decorative icons out of the tree, so a tile's name is the label alone", () => {
        render(<GetUpdates hubs={[HUB_B]} policyId={POLICY_ID} receipt={RECEIPT} />)
        for (const icon of Array.from(document.querySelectorAll("svg"))) {
            expect(icon.getAttribute("aria-hidden")).toBe("true")
        }
        expect(
            screen.getByRole("link", {
                name: 'vitrine.updates.hubLabel {"name":"hub-b","party":"Partner Co"}',
            })
        ).toBeTruthy()
    })
})

describe("the words", () => {
    type TCatalogue = {translations: {vitrine: {updates: Record<string, string>}}}
    const catalogues = Object.entries(votingPortalTranslations).map(
        ([language, resource]) =>
            [language, (resource as unknown as TCatalogue).translations.vitrine.updates] as const
    )
    const english = catalogues.find(([language]) => language === "en")![1]
    const placeholders = (text: string): string[] => (text.match(/{{\w+}}/g) ?? []).sort()

    it("are there in every language the portal ships", () => {
        expect(catalogues.map(([language]) => language).sort()).toEqual(
            ["cat", "en", "es", "eu", "fr", "gl", "nl", "tl"].sort()
        )
    })

    it.each(catalogues)("%s has every key, with the same placeholders", (_language, words) => {
        expect(Object.keys(words).sort()).toEqual(Object.keys(english).sort())
        for (const [key, text] of Object.entries(words)) {
            expect(text.trim()).not.toBe("")
            expect(placeholders(text)).toEqual(placeholders(english[key]))
        }
    })

    it("name no hub apart from the others", () => {
        for (const text of Object.values(english)) {
            expect(text).not.toMatch(/independent|recommended|default|featured|badge/i)
        }
    })
})
