/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import {render, screen, within} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import "@testing-library/jest-dom"
import {TimeZonePicker} from "./TimeZonePicker"
import {MyTimeZoneProvider} from "./timeZoneService"
import {MY_TIME_ZONE, testI18n} from "./__fixtures__/testI18n"

jest.mock("@sequentech/ui-core", () => ({
    ...jest.requireActual("../../../../ui-core/src/services/timeZones"),
    ...jest.requireActual("../../../../ui-core/src/services/eventTimeZones"),
    ...jest.requireActual("../../../../ui-core/src/types/ScheduledOutcome"),
    ...jest.requireActual("../../../../ui-core/src/types/ElectionEventPresentation"),
}))
const mockI18n = testI18n()
jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: mockI18n.t, i18n: mockI18n}),
}))

// Labels are taken at the scheduled time, so they don't depend on today's DST.
const AT = new Date("2028-04-08T20:00:00Z")
const ZONES = ["Asia/Manila", "Asia/Dubai", "America/Managua", "Europe/London", "Europe/Madrid"]

const Picker: React.FC<{onChange: (zone: string | null) => void; initial?: string}> = ({
    onChange,
    initial = "Asia/Dubai",
}) => {
    const [zone, setZone] = useState<string | null>(initial)
    return (
        <MyTimeZoneProvider zone={MY_TIME_ZONE}>
            <TimeZonePicker
                label="Timezone"
                value={zone}
                zones={ZONES}
                primary="Asia/Manila"
                at={AT}
                onChange={(next) => {
                    setZone(next)
                    onChange(next)
                }}
            />
        </MyTimeZoneProvider>
    )
}

const field = () => screen.getByRole("combobox", {name: "Timezone"}) as HTMLInputElement

/** Types a search the way a person does: focus, clear, type. */
const search = async (query: string) => {
    const user = userEvent
    await user.click(field())
    await user.clear(field())
    await user.type(field(), query)
    return user
}

const shown = () =>
    within(screen.getByRole("listbox"))
        .getAllByRole("option")
        .map((option) => option.firstChild?.textContent)

describe("TimeZonePicker", () => {
    it("shows the selected zone as its offset and city", () => {
        render(<Picker onChange={jest.fn()} />)
        expect(field()).toHaveValue("(GMT+04:00) Dubai")
    })

    it("finds Manila by city, marks the primary and names its country and zone", async () => {
        render(<Picker onChange={jest.fn()} />)
        await search("Manila")
        const options = within(screen.getByRole("listbox")).getAllByRole("option")
        expect(options).toHaveLength(1)
        expect(options[0]).toHaveTextContent("(GMT+08:00) Manila · primary")
        expect(options[0]).toHaveTextContent("Philippines · Philippine Standard Time")
    })

    it.each([
        ["a country", "Nicaragua", ["(GMT-06:00) Managua"]],
        ["an abbreviation", "PhST", ["(GMT+08:00) Manila · primary"]],
        ["an offset", "GMT+04:00", ["(GMT+04:00) Dubai"]],
        ["a zone name", "Europe/Mad", ["(GMT+02:00) Madrid"]],
    ])("searches by %s", async (_what, query, expected) => {
        render(<Picker onChange={jest.fn()} />)
        await search(query)
        expect(shown()).toEqual(expected)
    })

    it("chooses with the keyboard: type, arrows, Enter", async () => {
        const onChange = jest.fn()
        render(<Picker onChange={onChange} />)
        const user = await search("man")
        // The first match is highlighted; ArrowDown moves to the next.
        expect(shown()).toEqual(["(GMT-06:00) Managua", "(GMT+08:00) Manila · primary"])
        await user.keyboard("{ArrowDown}{Enter}")
        expect(onChange).toHaveBeenLastCalledWith("Asia/Manila")
        expect(field()).toHaveValue("(GMT+08:00) Manila · primary")
        expect(screen.queryByRole("listbox")).toBeNull()
    })

    it("closes on Escape without changing the zone", async () => {
        const onChange = jest.fn()
        render(<Picker onChange={onChange} />)
        const user = userEvent
        await user.click(field())
        await user.keyboard("{ArrowDown}")
        expect(screen.getByRole("listbox")).toBeInTheDocument()
        await user.keyboard("{Escape}")
        expect(screen.queryByRole("listbox")).toBeNull()
        expect(onChange).not.toHaveBeenCalled()
        expect(field()).toHaveValue("(GMT+04:00) Dubai")
    })

    it("says so when nothing matches", async () => {
        render(<Picker onChange={jest.fn()} />)
        await search("Atlantis")
        expect(screen.getByText(mockI18n.t("lifecycle.picker.noMatch"))).toBeInTheDocument()
    })
})
