/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {fireEvent, render, screen} from "@testing-library/react"
import "@testing-library/jest-dom"
import {
    ForensicAddressButton,
    ForensicBallotCount,
    ForensicCountry,
    ForensicPseudonym,
} from "./ForensicFields"

jest.mock("react-i18next", () => ({useTranslation: () => ({t: (key: string) => key})}))

test("forensic review shows a pseudonym, a country and the scoped ballot count", () => {
    render(
        <>
            <ForensicPseudonym code="7QKM-2HXD" />
            <ForensicCountry code="ES" showName />
            <ForensicBallotCount count={12} shared />
        </>
    )
    expect(screen.getByText("7QKM:2HXD")).toBeVisible()
    expect(screen.getByText("ES")).toBeVisible()
    expect(screen.getByText("Spain")).toBeVisible()
    expect(screen.getByText("12")).toBeVisible()
    expect(screen.getByTitle("ballotPrivacy.forensic.sharedHelp")).toBeVisible()
})

test("unknown country and an unshared address do not imply an attribution", () => {
    const {rerender} = render(
        <>
            <ForensicCountry code={null} showName />
            <ForensicBallotCount count={1} shared={false} />
        </>
    )
    expect(screen.getByText("—")).toBeVisible()
    expect(screen.queryByText("ballotPrivacy.forensic.shared")).not.toBeInTheDocument()
    rerender(<ForensicCountry code="ZZ" showName />)
    expect(screen.getByText("ZZ")).toBeVisible()
    expect(screen.queryByText("Spain")).not.toBeInTheDocument()
    rerender(<ForensicCountry code="ES" />)
    expect(screen.queryByText("Spain")).not.toBeInTheDocument()
})

test("opening a forensic address delegates to its audited review flow", () => {
    const open = jest.fn()
    render(<ForensicAddressButton ip="203.0.113.7" onOpen={open} />)
    fireEvent.click(screen.getByRole("button", {name: "203.0.113.7"}))
    expect(open).toHaveBeenCalledTimes(1)
})
