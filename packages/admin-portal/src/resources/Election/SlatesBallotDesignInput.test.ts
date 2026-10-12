/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {cleanup, fireEvent, render, screen} from "@testing-library/react"
import {FieldValues, FormProvider, useForm, useWatch} from "react-hook-form"
import {SlatesBallotDesignInput} from "./SlatesBallotDesignInput"

jest.mock("@sequentech/ui-core", () => ({
    EMobileCandidateLists: {COLLAPSED: "collapsed", EXPANDED: "expanded"},
}))
jest.mock("sequent-core", () => ({}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key}),
}))
jest.mock("react-admin", () => ({
    required: () => () => undefined,
    SelectInput: ({source, label}: {source: string; label: string}) =>
        require("react").createElement("div", {"data-testid": source}, label),
}))
jest.mock("@sequentech/ui-essentials", () => ({
    Dialog: ({
        open,
        ok,
        cancel,
        children,
        handleClose,
    }: React.PropsWithChildren<{
        open: boolean
        ok: string
        cancel: string
        handleClose: (result: boolean) => void
    }>) =>
        open
            ? require("react").createElement(
                  "div",
                  {role: "dialog"},
                  children,
                  require("react").createElement("button", {onClick: () => handleClose(true)}, ok),
                  require("react").createElement(
                      "button",
                      {onClick: () => handleClose(false)},
                      cancel
                  )
              )
            : null,
}))

const EnabledValue: React.FC = () => {
    const enabled = useWatch({name: "slates_enabled"})
    return React.createElement("output", null, String(enabled))
}

const Form: React.FC<{enabled: boolean; slatesCount: number}> = ({enabled, slatesCount}) => {
    const methods = useForm<FieldValues>({defaultValues: {slates_enabled: enabled}})
    return React.createElement(FormProvider, {
        ...methods,
        children: [
            React.createElement(SlatesBallotDesignInput, {key: "input", slatesCount}),
            React.createElement(EnabledValue, {key: "value"}),
        ],
    })
}

const renderForm = (enabled: boolean, slatesCount: number) =>
    render(React.createElement(Form, {enabled, slatesCount}))

const toggle = () =>
    screen.getByRole("switch", {name: "electionScreen.slates.enable.label"}) as HTMLInputElement

describe("SlatesBallotDesignInput", () => {
    afterEach(cleanup)

    it("shows the mobile candidate lists only while slates are enabled", () => {
        renderForm(false, 0)
        expect(toggle().checked).toBe(false)
        expect(screen.queryByTestId("slates_mobile_candidate_lists")).toBeNull()
        expect(screen.getByText("electionScreen.slates.enable.helper")).toBeTruthy()

        fireEvent.click(toggle())

        expect(toggle().checked).toBe(true)
        expect(screen.getByTestId("slates_mobile_candidate_lists")).toBeTruthy()
    })

    it("turns slates off at once when there are none", () => {
        renderForm(true, 0)
        fireEvent.click(toggle())

        expect(screen.queryByRole("dialog")).toBeNull()
        expect(screen.getByText("false")).toBeTruthy()
    })

    it("asks before discarding the election's slates", () => {
        renderForm(true, 2)
        fireEvent.click(toggle())

        expect(screen.getByRole("dialog").textContent).toContain(
            "electionScreen.slates.disable.message"
        )
        expect(screen.getByText("true")).toBeTruthy()

        fireEvent.click(screen.getByText("common.label.cancel"))
        expect(screen.queryByRole("dialog")).toBeNull()
        expect(toggle().checked).toBe(true)

        fireEvent.click(toggle())
        fireEvent.click(screen.getByText("electionScreen.slates.disable.confirm"))
        expect(toggle().checked).toBe(false)
        expect(screen.getByText("false")).toBeTruthy()
    })
})
