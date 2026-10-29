// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {renderToStaticMarkup} from "react-dom/server"
import {ENumberFormatPolicy} from "../types/ElectionEventPresentation"
import {NumberFormatProvider, useNumberFormat} from "./NumberFormatContext"

const Figures: React.FC = () => {
    const {policy, formatNumber, formatPercentage} = useNumberFormat()
    return (
        <span>
            {[
                policy,
                formatNumber(1234567),
                formatNumber(1234.5, 1),
                formatPercentage(45.678),
            ].join(" | ")}
        </span>
    )
}

describe("NumberFormatContext", () => {
    it("writes the figures below a provider in its policy", () => {
        expect(
            renderToStaticMarkup(
                <NumberFormatProvider policy={ENumberFormatPolicy.PERIOD_COMMA}>
                    <Figures />
                </NumberFormatProvider>
            )
        ).toBe("<span>period-comma | 1.234.567 | 1.234,5 | 45,68%</span>")
    })

    it("uses the default format without a provider", () => {
        expect(renderToStaticMarkup(<Figures />)).toBe(
            "<span>comma-period | 1,234,567 | 1,234.5 | 45.68%</span>"
        )
    })

    it.each([undefined, null, "no-such-format"])(
        "uses the default format for an event whose policy is %p",
        (policy) => {
            expect(
                renderToStaticMarkup(
                    <NumberFormatProvider policy={policy}>
                        <Figures />
                    </NumberFormatProvider>
                )
            ).toBe("<span>comma-period | 1,234,567 | 1,234.5 | 45.68%</span>")
        }
    )

    it("lets the nearest provider decide", () => {
        expect(
            renderToStaticMarkup(
                <NumberFormatProvider policy={ENumberFormatPolicy.PERIOD_COMMA}>
                    <NumberFormatProvider policy={ENumberFormatPolicy.APOSTROPHE_PERIOD}>
                        <Figures />
                    </NumberFormatProvider>
                </NumberFormatProvider>
            )
        ).toBe("<span>apostrophe-period | 1’234’567 | 1’234.5 | 45.68%</span>")
    })
})
