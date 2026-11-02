/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The status tag (design board StatusTag): its label, and a tone for each
// kind of state.

import React from "react"
import {render, screen} from "@testing-library/react"
import {EStatusTone, StatusTag, STATUS_TONES} from "./StatusTag"

describe("StatusTag", () => {
    it("shows its label, and is a success tag unless told otherwise", () => {
        render(<StatusTag label="Cast" />)
        const tag = screen.getByText("Cast")
        expect(tag).toBeVisible()
        expect(tag).toHaveAttribute("data-tone", EStatusTone.SUCCESS)
    })

    it.each(Object.values(EStatusTone))("marks the %s tone", (tone) => {
        render(<StatusTag label="Label" tone={tone} />)
        expect(screen.getByText("Label")).toHaveAttribute("data-tone", tone)
    })

    it("keeps the dot of a neutral tag hollow", () => {
        expect(STATUS_TONES[EStatusTone.NEUTRAL].dot).toBe("transparent")
        expect(STATUS_TONES[EStatusTone.SUCCESS].dot).toBe(STATUS_TONES[EStatusTone.SUCCESS].border)
    })

    it("hides the dot from assistive technology, so only the label is read", () => {
        const {container} = render(<StatusTag label="Not cast" tone={EStatusTone.NEUTRAL} />)
        expect(container.querySelector("svg")).toHaveAttribute("aria-hidden", "true")
        expect(screen.getByText("Not cast")).toBeVisible()
    })
})
