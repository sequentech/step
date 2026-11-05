/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import {fireEvent, render, screen, within} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import "@testing-library/jest-dom"
import {
    ZonedDateTimeField,
    ZonedDateTimeInput,
    editableOf,
    zonedValueOf,
    type IZonedDateTimeValue,
} from "./ZonedDateTimeInput"
import {MyTimeZoneProvider} from "./timeZoneService"
import {MY_TIME_ZONE, testI18n} from "./__fixtures__/testI18n"
import * as uiCoreZones from "../../../../ui-core/src/services/timeZones"

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
// The field is plain MUI; the react-admin wrapper isn't rendered here.
let mockField: {value: unknown; onChange: jest.Mock}
jest.mock("react-admin", () => ({
    useInput: () => ({field: mockField, fieldState: {invalid: false}}),
    InputHelperText: () => null,
}))

// "My time" is New York's, whatever the machine's zone.
const Field: React.FC<{
    onChange: (value: IZonedDateTimeValue | null) => void
    initialZone: string
    place?: string
}> = ({onChange, initialZone, place}) => {
    const [edited, setEdited] = useState({local: "", timezone: initialZone})
    return (
        <MyTimeZoneProvider zone={MY_TIME_ZONE}>
            <ZonedDateTimeField
                label="Scheduled at"
                local={edited.local}
                timezone={edited.timezone}
                place={place}
                onChange={(value, next) => {
                    setEdited(next)
                    onChange(value)
                }}
            />
        </MyTimeZoneProvider>
    )
}

const enter = (local: string) =>
    fireEvent.change(screen.getByLabelText("Scheduled at"), {target: {value: local}})

const preview = () => within(screen.getByTestId("zoned-preview"))

describe("ZonedDateTimeField", () => {
    it("saves the wall time, its zone and the instant, and previews both times", () => {
        const onChange = jest.fn()
        render(<Field onChange={onChange} initialZone="Asia/Dubai" place="Dubai PCG" />)
        enter("2028-04-09T00:00")
        expect(onChange).toHaveBeenLastCalledWith({
            scheduled_date: "2028-04-08T20:00:00Z",
            local: "2028-04-09T00:00",
            timezone: "Asia/Dubai",
        })
        expect(preview().getByText("Apr 09, 2028, 00:00 GMT+4 · Dubai PCG")).toBeInTheDocument()
        expect(preview().getByText("Apr 08, 2028, 16:00 EDT · my time")).toBeInTheDocument()
        expect(screen.queryByRole("alert")).toBeNull()
    })

    it("keeps the wall time and moves the instant when the zone changes", async () => {
        const onChange = jest.fn()
        render(<Field onChange={onChange} initialZone="Asia/Dubai" />)
        enter("2028-04-09T00:00")
        const user = userEvent
        const zone = screen.getByRole("combobox", {name: "Timezone"})
        await user.click(zone)
        await user.clear(zone)
        await user.type(zone, "Kathmandu{Enter}")
        // +05:45: the instant is 18:15 UTC the day before.
        expect(onChange).toHaveBeenLastCalledWith({
            scheduled_date: "2028-04-08T18:15:00Z",
            local: "2028-04-09T00:00",
            timezone: "Asia/Kathmandu",
        })
    })

    it("shows one line when the row's zone is my zone", () => {
        render(<Field onChange={jest.fn()} initialZone="America/New_York" />)
        enter("2028-04-09T00:00")
        expect(preview().getAllByText(/./)).toHaveLength(1)
        expect(preview().getByText("Apr 09, 2028, 00:00 EDT")).toBeInTheDocument()
    })

    it("explains a time that doesn't exist and saves the time shown", () => {
        const onChange = jest.fn()
        render(<Field onChange={onChange} initialZone="America/Toronto" />)
        enter("2028-03-12T02:30")
        expect(screen.getByRole("alert")).toHaveTextContent(
            "Mar 12, 2028, 02:30 does not exist in Toronto because clocks go forward. It will run at the time shown."
        )
        // The time shown: 03:30 EDT, the same distance after the gap.
        expect(onChange).toHaveBeenLastCalledWith(
            expect.objectContaining({scheduled_date: "2028-03-12T07:30:00Z"})
        )
        expect(preview().getByText("Mar 12, 2028, 03:30 EDT")).toBeInTheDocument()
    })

    it("explains a time that happens twice and uses the first", () => {
        const onChange = jest.fn()
        render(<Field onChange={onChange} initialZone="America/Toronto" />)
        enter("2028-11-05T01:30")
        expect(screen.getByRole("alert")).toHaveTextContent(
            "Nov 05, 2028, 01:30 happens twice in Toronto. The first one is used."
        )
        // The first 01:30 is still daylight time (UTC-4).
        expect(onChange).toHaveBeenLastCalledWith(
            expect.objectContaining({scheduled_date: "2028-11-05T05:30:00Z"})
        )
    })

    it("saves nothing while the wall time is empty", () => {
        const onChange = jest.fn()
        render(<Field onChange={onChange} initialZone="Asia/Dubai" />)
        enter("2028-04-09T00:00")
        enter("")
        expect(onChange).toHaveBeenLastCalledWith(null)
        expect(screen.queryByTestId("zoned-preview")).toBeNull()
    })
})

describe("editableOf and zonedValueOf", () => {
    it("edits a saved time as it was entered", () => {
        expect(
            editableOf(
                {
                    scheduled_date: "2028-04-08T20:00:00Z",
                    local: "2028-04-09T00:00",
                    timezone: "Asia/Dubai",
                },
                "Asia/Manila",
                uiCoreZones
            )
        ).toEqual({local: "2028-04-09T00:00", timezone: "Asia/Dubai"})
    })

    it("edits an older instant in the row's zone", () => {
        expect(
            editableOf({scheduled_date: "2028-05-08T11:00:00Z"}, "Asia/Manila", uiCoreZones)
        ).toEqual({local: "2028-05-08T19:00", timezone: "Asia/Manila"})
    })

    it("doesn't guess the zone of a date without an offset", () => {
        expect(
            editableOf({scheduled_date: "2028-05-08T19:00:00"}, "Asia/Manila", uiCoreZones)
        ).toEqual({local: "", timezone: "Asia/Manila"})
    })

    it("has no value for an incomplete wall time", () => {
        expect(zonedValueOf("2028-05-08", "Asia/Manila", uiCoreZones)).toBeNull()
    })
})

describe("ZonedDateTimeInput", () => {
    it("shows what the form holds after a reset from outside", () => {
        mockField = {value: undefined, onChange: jest.fn()}
        const input = () => (
            <MyTimeZoneProvider zone={MY_TIME_ZONE}>
                <ZonedDateTimeInput
                    source="schedule.zoned"
                    label="Scheduled at"
                    defaultZone="UTC"
                />
            </MyTimeZoneProvider>
        )
        const view = render(input())
        enter("2028-04-09T00:00")
        expect(mockField.onChange).toHaveBeenLastCalledWith(
            expect.objectContaining({local: "2028-04-09T00:00", timezone: "UTC"})
        )
        mockField = {...mockField, value: mockField.onChange.mock.lastCall?.[0]}
        view.rerender(input())
        expect(screen.getByLabelText("Scheduled at")).toHaveValue("2028-04-09T00:00")
        // The form is reset: the field is empty again, and so is the input.
        mockField = {...mockField, value: undefined}
        view.rerender(input())
        expect(screen.getByLabelText("Scheduled at")).toHaveValue("")
    })
})
