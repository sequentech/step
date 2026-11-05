/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// "Election authority contact" in the Transparency tab.

import React from "react"
import {act, fireEvent, render, screen} from "@testing-library/react"
import {useMutation} from "@apollo/client"
import axe from "axe-core"
import {AuthorityContactSection} from "./AuthorityContact"

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
const KEYS = "transparencyTab.authorityContact"

const save = jest.fn()
const changed = jest.fn()

interface IProps {
    presentation?: unknown
    canChange?: boolean
    locked?: boolean
    sealed?: boolean
}

const section = ({presentation, canChange = true, locked = false, sealed = false}: IProps = {}) => (
    <AuthorityContactSection
        electionEventId={EVENT}
        presentation={presentation}
        canChange={canChange}
        locked={locked}
        sealed={sealed}
        onChanged={changed}
    />
)

const renderSection = async (props: IProps = {}) => {
    const view = render(section(props))
    await act(async () => {})
    return view
}

const field = (kind: string) => screen.getByTestId(`authority-contact-${kind}`) as HTMLInputElement
const type = (kind: string, value: string) => fireEvent.change(field(kind), {target: {value}})
const saveButton = () => screen.getByTestId("authority-contact-save") as HTMLButtonElement
const stored = (contact: unknown) => ({
    data: {
        update_sequent_backend_election_event_by_pk: {id: EVENT, authority_contact: contact},
    },
})

beforeEach(() => {
    jest.clearAllMocks()
    ;(useMutation as jest.Mock).mockReturnValue([save, {loading: false}])
})

describe("the fields", () => {
    it("are four, empty when the event has no contact", async () => {
        await renderSection({presentation: {css: "x"}})
        for (const kind of ["name", "email", "phone", "url"]) {
            expect(field(kind).value).toBe("")
            expect(screen.getByLabelText(`${KEYS}.${kind}.label`)).toBe(field(kind))
        }
        expect(screen.getByRole("heading", {name: `${KEYS}.title`})).toBeTruthy()
        expect(screen.getByText(`${KEYS}.intro`)).toBeTruthy()
        expect(saveButton().disabled).toBe(true)
    })

    it("show what the event's presentation stores", async () => {
        await renderSection({
            presentation: {
                authority_contact: {name: "Electoral Board of Acme", phone: "+34900000000"},
            },
        })
        expect(field("name").value).toBe("Electoral Board of Acme")
        expect(field("phone").value).toBe("+34900000000")
        expect(field("email").value).toBe("")
    })

    it("have no accessibility violations", async () => {
        const view = await renderSection({presentation: {authority_contact: {name: "Board"}}})
        type("email", "nope")
        const results = await axe.run(view.container, {rules: {"color-contrast": {enabled: false}}})
        expect(results.violations).toEqual([])
    })
})

describe("saving", () => {
    it("stores the kinds that are set under authority_contact, as the election-event-write role", async () => {
        save.mockResolvedValue(stored({name: "Board", email: "elections@acme.example"}))
        await renderSection()
        type("name", " Board ")
        type("email", "elections@acme.example")
        expect(saveButton().disabled).toBe(false)
        await act(async () => {
            fireEvent.click(saveButton())
        })
        expect(useMutation).toHaveBeenCalledWith(expect.stringContaining("SetAuthorityContact"), {
            context: ROLE,
        })
        expect(save).toHaveBeenCalledTimes(1)
        expect(save).toHaveBeenCalledWith({
            variables: {
                electionEventId: EVENT,
                patch: {authority_contact: {name: "Board", email: "elections@acme.example"}},
            },
        })
        expect(screen.getByTestId("authority-contact-notice").textContent).toBe(`${KEYS}.saved`)
        expect(changed).toHaveBeenCalledTimes(1)
        expect(saveButton().disabled).toBe(true)
    })

    it("drops a kind that was cleared", async () => {
        save.mockResolvedValue(stored({name: "Board"}))
        await renderSection({
            presentation: {authority_contact: {name: "Board", phone: "+34900000000"}},
        })
        type("phone", "")
        await act(async () => {
            fireEvent.click(saveButton())
        })
        expect(save.mock.calls[0][0].variables.patch).toEqual({
            authority_contact: {name: "Board"},
        })
    })

    it("can clear the whole contact", async () => {
        save.mockResolvedValue(stored({}))
        await renderSection({presentation: {authority_contact: {name: "Board"}}})
        type("name", "")
        await act(async () => {
            fireEvent.click(saveButton())
        })
        expect(save.mock.calls[0][0].variables.patch).toEqual({authority_contact: {}})
        expect(screen.getByTestId("authority-contact-notice").textContent).toBe(`${KEYS}.saved`)
    })

    it("does not say it is saved, and keeps what was typed, when the save fails", async () => {
        save.mockRejectedValue({graphQLErrors: [{message: "permission denied"}]})
        await renderSection()
        type("name", "Board")
        await act(async () => {
            fireEvent.click(saveButton())
        })
        expect(screen.getByRole("alert").textContent).toBe(
            `${KEYS}.saveError ${JSON.stringify({reason: "permission denied"})}`
        )
        expect(field("name").value).toBe("Board")
        expect(saveButton().disabled).toBe(false)
        expect(changed).not.toHaveBeenCalled()
    })

    it("does not say it is saved when nothing was stored", async () => {
        save.mockResolvedValue(stored(null))
        await renderSection()
        type("name", "Board")
        await act(async () => {
            fireEvent.click(saveButton())
        })
        expect(screen.getByRole("alert").textContent).toContain(`${KEYS}.saveError`)
        expect(changed).not.toHaveBeenCalled()
    })

    it("reads the record again when it changes", async () => {
        const view = await renderSection({presentation: {authority_contact: {name: "Board"}}})
        type("name", "Something else")
        view.rerender(section({presentation: {authority_contact: {name: "Board of Acme"}}}))
        expect(field("name").value).toBe("Board of Acme")
    })

    it("holds the fields while it saves", async () => {
        ;(useMutation as jest.Mock).mockReturnValue([save, {loading: true}])
        await renderSection()
        expect(field("name").disabled).toBe(true)
    })
})

describe("a value that breaks the policy's syntax", () => {
    it.each([
        ["name", "Electoral  Board"],
        ["email", "elections@acme"],
        ["phone", "+34 900 000 000"],
        ["url", "http://elections.acme.example"],
    ])("in the %s says so and can't be saved", async (kind, value) => {
        await renderSection()
        type(kind, value)
        expect(field(kind).getAttribute("aria-invalid")).toBe("true")
        expect(screen.getByText(`${KEYS}.${kind}.invalid`)).toBeTruthy()
        expect(saveButton().disabled).toBe(true)
        fireEvent.submit(saveButton().closest("form") as HTMLFormElement)
        expect(save).not.toHaveBeenCalled()
        type(kind, "")
        expect(field(kind).getAttribute("aria-invalid")).toBe("false")
    })
})

describe("when it can't be changed", () => {
    it("shows the fields as they are, with a word for the admin who may not change them", async () => {
        await renderSection({
            presentation: {authority_contact: {name: "Board"}},
            canChange: false,
        })
        expect(field("name").value).toBe("Board")
        expect(field("name").disabled).toBe(true)
        expect(screen.queryByTestId("authority-contact-save")).toBeNull()
        expect(screen.getByTestId("authority-contact-read-only").textContent).toBe(
            `${KEYS}.readOnly`
        )
    })

    it("is locked once voting has opened", async () => {
        await renderSection({presentation: {authority_contact: {name: "Board"}}, locked: true})
        expect(field("name").disabled).toBe(true)
        expect(screen.queryByTestId("authority-contact-save")).toBeNull()
        expect(screen.getByTestId("authority-contact-read-only").textContent).toBe(`${KEYS}.locked`)
    })
})

describe("with a sealed policy", () => {
    it("says that saving changes the draft, once something is changed", async () => {
        await renderSection({sealed: true})
        expect(screen.queryByTestId("authority-contact-successor")).toBeNull()
        type("name", "Board")
        expect(screen.getByTestId("authority-contact-successor").textContent).toBe(
            `${KEYS}.successor`
        )
    })

    it("says nothing of a successor before a policy is sealed", async () => {
        await renderSection({sealed: false})
        type("name", "Board")
        expect(screen.queryByTestId("authority-contact-successor")).toBeNull()
    })
})
