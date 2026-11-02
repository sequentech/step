/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// "Updates from hubs" in the Transparency tab (design board AdminHubUpdates).

import React from "react"
import {act, fireEvent, render, screen, within} from "@testing-library/react"
import {useMutation} from "@apollo/client"
import axe from "axe-core"
import * as verify from "@vitrine/verify"
import type {Policy} from "@vitrine/verify"
import {policyText} from "@/fixtures/vitrinePolicy"
import {HubUpdatesSection} from "./HubUpdates"

jest.mock("@apollo/client", () => ({
    gql: (strings: TemplateStringsArray) => strings.join(""),
    useQuery: jest.fn(),
    useMutation: jest.fn(),
}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, options?: Record<string, unknown>) =>
            options ? `${key} ${JSON.stringify(options)}` : key,
        i18n: {language: "en"},
    }),
}))

const EVENT = "0b6c4f53-8c0b-4cbb-9b7a-6c8a3a5c2f10"
const ROLE = {headers: {"x-hasura-role": "election-event-write"}}
const KEYS = "transparencyTab.hubUpdates"

const save = jest.fn()
const changed = jest.fn()

let policy: Policy | undefined

const sealed = (hubExplorers: boolean): Policy => verify.pinPolicy(policyText({hubExplorers}))

const section = (presentation?: unknown, allowed = true) => (
    <HubUpdatesSection
        electionEventId={EVENT}
        presentation={presentation}
        policy={policy}
        canChange={allowed}
        onChanged={changed}
    />
)

const renderSection = async (presentation?: unknown, allowed = true) => {
    const view = render(section(presentation, allowed))
    await act(async () => {})
    return view
}

const switchOf = (name: "confirmation" | "castEmail") =>
    screen.getByRole("switch", {name: `${KEYS}.${name}.label`}) as HTMLInputElement

const stored = (confirmation: string, cast_email: string) => ({
    data: {
        update_sequent_backend_election_event_by_pk: {
            id: EVENT,
            hub_updates: {confirmation, cast_email},
        },
    },
})

beforeAll(async () => {
    await verify.init()
})

beforeEach(() => {
    jest.clearAllMocks()
    policy = sealed(true)
    ;(useMutation as jest.Mock).mockReturnValue([save, {loading: false}])
})

describe("the switches", () => {
    it("are both on when the event has not set them", async () => {
        await renderSection({css: "x"})
        expect(switchOf("confirmation").checked).toBe(true)
        expect(switchOf("castEmail").checked).toBe(true)
    })

    it("show what the event's presentation stores", async () => {
        await renderSection({hub_updates: {confirmation: "off", cast_email: "on"}})
        expect(switchOf("confirmation").checked).toBe(false)
        expect(switchOf("castEmail").checked).toBe(true)
    })

    it("are named and described by the board's words", async () => {
        await renderSection()
        expect(switchOf("confirmation").getAttribute("aria-describedby")).toBeTruthy()
        const help = document.getElementById(
            switchOf("castEmail").getAttribute("aria-describedby") ?? ""
        )
        expect(help?.textContent).toBe(`${KEYS}.castEmail.help`)
        expect(screen.getByRole("heading", {name: `${KEYS}.title`})).toBeTruthy()
        expect(screen.getByText(`${KEYS}.intro`)).toBeTruthy()
        expect(screen.getByText(`${KEYS}.footer`)).toBeTruthy()
    })

    it("are on a switch role, not a plain checkbox", async () => {
        await renderSection()
        expect(screen.getAllByRole("switch")).toHaveLength(2)
        expect(screen.queryAllByRole("checkbox")).toHaveLength(0)
    })
})

describe("turning one off", () => {
    it("stores both switches under hub_updates, as the election-event-write role, and says so", async () => {
        save.mockResolvedValue(stored("off", "on"))
        await renderSection()
        await act(async () => {
            fireEvent.click(switchOf("confirmation"))
        })
        expect(useMutation).toHaveBeenCalledWith(expect.stringContaining("SetHubUpdates"), {
            context: ROLE,
        })
        expect(save).toHaveBeenCalledTimes(1)
        expect(save).toHaveBeenCalledWith({
            variables: {
                electionEventId: EVENT,
                patch: {hub_updates: {confirmation: "off", cast_email: "on"}},
            },
        })
        expect(switchOf("confirmation").checked).toBe(false)
        expect(screen.getByTestId("hub-updates-notice").textContent).toBe(`${KEYS}.saved`)
        expect(changed).toHaveBeenCalledTimes(1)
    })

    it("keeps the other switch as it is stored", async () => {
        save.mockResolvedValue(stored("on", "off"))
        await renderSection({hub_updates: {confirmation: "on", cast_email: "on"}})
        await act(async () => {
            fireEvent.click(switchOf("castEmail"))
        })
        expect(save.mock.calls[0][0].variables.patch).toEqual({
            hub_updates: {confirmation: "on", cast_email: "off"},
        })
    })

    it("turns it on again", async () => {
        save.mockResolvedValue(stored("on", "on"))
        await renderSection({hub_updates: {confirmation: "off", cast_email: "on"}})
        await act(async () => {
            fireEvent.click(switchOf("confirmation"))
        })
        expect(save.mock.calls[0][0].variables.patch).toEqual({
            hub_updates: {confirmation: "on", cast_email: "on"},
        })
    })

    it("does not say it is saved, and leaves the switch, when the save fails", async () => {
        save.mockRejectedValue({graphQLErrors: [{message: "permission denied"}]})
        await renderSection()
        await act(async () => {
            fireEvent.click(switchOf("castEmail"))
        })
        expect(switchOf("castEmail").checked).toBe(true)
        const notice = screen.getByRole("alert")
        expect(notice.textContent).toBe(
            `${KEYS}.saveError ${JSON.stringify({reason: "permission denied"})}`
        )
        expect(changed).not.toHaveBeenCalled()
    })

    it("does not say it is saved when nothing was stored", async () => {
        save.mockResolvedValue({
            data: {update_sequent_backend_election_event_by_pk: {id: EVENT, hub_updates: null}},
        })
        await renderSection()
        await act(async () => {
            fireEvent.click(switchOf("confirmation"))
        })
        expect(switchOf("confirmation").checked).toBe(true)
        expect(screen.getByRole("alert").textContent).toContain(`${KEYS}.saveError`)
        expect(changed).not.toHaveBeenCalled()
    })

    it("reads the record again when it changes", async () => {
        save.mockResolvedValue(stored("off", "on"))
        const view = await renderSection()
        await act(async () => {
            fireEvent.click(switchOf("confirmation"))
        })
        view.rerender(section({hub_updates: {confirmation: "off", cast_email: "off"}}))
        expect(switchOf("confirmation").checked).toBe(false)
        expect(switchOf("castEmail").checked).toBe(false)
    })

    it("holds both switches while it saves", async () => {
        ;(useMutation as jest.Mock).mockReturnValue([save, {loading: true}])
        await renderSection()
        expect(switchOf("confirmation").disabled).toBe(true)
        expect(switchOf("castEmail").disabled).toBe(true)
    })
})

describe("without permission to change the event", () => {
    it("shows the switches as they are, locked, and says why", async () => {
        await renderSection({hub_updates: {confirmation: "off"}}, false)
        expect(switchOf("confirmation").disabled).toBe(true)
        expect(switchOf("confirmation").checked).toBe(false)
        expect(switchOf("castEmail").disabled).toBe(true)
        expect(screen.getByTestId("hub-updates-read-only").textContent).toBe(`${KEYS}.readOnly`)
        expect(save).not.toHaveBeenCalled()
    })

    it("does not say it with permission", async () => {
        await renderSection()
        expect(screen.queryByTestId("hub-updates-read-only")).toBeNull()
    })
})

describe("the hubs", () => {
    it("are the policy's hubs that have an explorer, with who runs each", async () => {
        await renderSection()
        const list = screen.getByTestId("hub-updates-hubs")
        const h1 = within(list).getByTestId("hub-updates-hub-h1")
        expect(h1.textContent).toContain("h1")
        expect(h1.textContent).toContain(
            `transparencyTab.overview.runBy ${JSON.stringify({party: "hubco1"})}`
        )
        expect(within(list).getByTestId("hub-updates-hub-h2").textContent).toContain("hubco2")
        expect(list.textContent).toContain(`${KEYS}.hubs.help`)
    })

    it("say that voters see them in a random order, and offer no way to order them", async () => {
        await renderSection()
        const list = screen.getByTestId("hub-updates-hubs")
        expect(list.textContent).toContain(`${KEYS}.hubs.order`)
        expect(within(list).queryAllByRole("button")).toHaveLength(0)
        expect(within(list).queryAllByRole("link")).toHaveLength(0)
        expect(list.querySelector("input, select, [draggable=true]")).toBeNull()
    })

    it("are none when the policy gives no hub a site", async () => {
        policy = sealed(false)
        await renderSection()
        expect(screen.getByTestId("hub-updates-no-hubs").textContent).toBe(`${KEYS}.hubs.none`)
        expect(screen.queryByText(`${KEYS}.hubs.order`)).toBeNull()
    })

    it("are none while the event has no sealed policy", async () => {
        policy = undefined
        await renderSection()
        expect(screen.getByTestId("hub-updates-no-hubs")).toBeTruthy()
        expect(switchOf("confirmation").checked).toBe(true)
    })

    it("are listed without links, since the admin only reads them here", async () => {
        await renderSection()
        expect(screen.getByTestId("hub-updates-hubs").querySelector("a")).toBeNull()
    })
})

describe("accessibility", () => {
    it("has no axe violations", async () => {
        const {container} = await renderSection({hub_updates: {castEmail: "off"}})
        const result = await axe.run(container, {rules: {"color-contrast": {enabled: false}}})
        expect(result.violations).toEqual([])
    })
})
