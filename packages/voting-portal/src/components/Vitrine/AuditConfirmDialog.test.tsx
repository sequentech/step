// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {fireEvent, render, screen, within} from "@testing-library/react"
import {AuditConfirmDialog} from "./AuditConfirmDialog"

jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key}),
    Trans: ({i18nKey, values}: {i18nKey: string; values: {id: string}}) => (
        <>{`${i18nKey} ${values.id}`}</>
    ),
}))

const TRACKING_ID = "FTBE-MHRX-0123-4567-89AB-CDEF-GHJK-MNPQ-RSTV-WXYZ-0123-4567-89AB"
const KEYS = "reviewScreen.auditConfirm"

const draw = (onClose = jest.fn()) => {
    render(<AuditConfirmDialog open trackingId={TRACKING_ID} onClose={onClose} />)
    return {onClose, dialog: screen.getByRole("dialog", {name: `${KEYS}.title`})}
}

describe("AuditConfirmDialog", () => {
    it("asks to audit instead of casting, naming the ballot by its short ID", () => {
        const {dialog} = draw()
        expect(dialog).toHaveAccessibleDescription(`${KEYS}.text FTBE-MHRX`)
        expect(within(dialog).getByText(`${KEYS}.never`)).toBeVisible()
        expect(within(dialog).getByText(`${KEYS}.again`)).toBeVisible()
    })

    it("offers Cancel and Audit ballot, and nothing else", () => {
        const {dialog} = draw()
        expect(
            within(dialog)
                .getAllByRole("button")
                .map((button) => button.textContent)
        ).toEqual([`${KEYS}.cancel`, `${KEYS}.audit`])
    })

    it("audits on Audit ballot", () => {
        const {dialog, onClose} = draw()
        fireEvent.click(within(dialog).getByRole("button", {name: `${KEYS}.audit`}))
        expect(onClose).toHaveBeenCalledWith(true)
    })

    it("does not audit on Cancel or when dismissed", () => {
        const {dialog, onClose} = draw()
        fireEvent.click(within(dialog).getByRole("button", {name: `${KEYS}.cancel`}))
        fireEvent.keyDown(dialog, {key: "Escape"})
        expect(onClose).toHaveBeenCalledTimes(2)
        expect(onClose).not.toHaveBeenCalledWith(true)
    })

    it("draws nothing while closed", () => {
        render(<AuditConfirmDialog open={false} trackingId={TRACKING_ID} onClose={jest.fn()} />)
        expect(screen.queryByRole("dialog")).toBeNull()
    })
})
