/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {cleanup, fireEvent, render, screen} from "@testing-library/react"
import {CoreAdminContext, Form} from "react-admin"
import {useFormContext} from "react-hook-form"
import {testI18n} from "@/components/timezones/__fixtures__/testI18n"
import {LogRangeDateTimeInput} from "./LogRangeDateTimeInput"

jest.mock("react-admin", () => {
    const {TextEncoder, TextDecoder} = jest.requireActual("node:util")
    Object.assign(globalThis, {TextEncoder, TextDecoder})
    return {
        ...jest.requireActual("ra-core"),
        DateTimeInput: jest.requireActual(
            "../../../../node_modules/react-admin/node_modules/ra-ui-materialui/dist/input/DateTimeInput.cjs"
        ).DateTimeInput,
    }
})
jest.mock("@sequentech/ui-core", () =>
    jest.requireActual("../../../../ui-core/src/services/timeZones")
)
const mockI18n = testI18n()
jest.mock("react-i18next", () => ({useTranslation: () => ({t: mockI18n.t, i18n: mockI18n})}))
afterEach(cleanup)

function ChangeZone() {
    const {setValue} = useFormContext()
    return (
        <button type="button" onClick={() => setValue("time_zone", "UTC")}>
            Use UTC
        </button>
    )
}
const show = (local: string, source = "created_from") =>
    render(
        <CoreAdminContext>
            <Form
                defaultValues={{[source]: local, time_zone: "America/Toronto"}}
                onSubmit={jest.fn()}
            >
                <LogRangeDateTimeInput source={source} label="Created from" />
                <ChangeZone />
            </Form>
        </CoreAdminContext>
    )

it.each(["created_from", "created_to", "statement_timestamp_from", "statement_timestamp_to"])(
    "explains %s gap in the chosen zone and removes the note when the zone changes",
    (source) => {
        show("2028-03-12T02:30", source)
        expect(screen.getByText(/does not exist in Toronto/)).toBeTruthy()
        fireEvent.click(screen.getByText("Use UTC"))
        expect(screen.queryByText(/does not exist in Toronto/)).toBeNull()
        expect((screen.getByLabelText("Created from") as HTMLInputElement).value).toBe(
            "2028-03-12T02:30"
        )
    }
)

it("explains the first occurrence chosen for an overlapping log range", () => {
    show("2028-11-05T01:30")
    expect(screen.getByText(/happens twice in Toronto/)).toBeTruthy()
})
