// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {renderToStaticMarkup} from "react-dom/server"
import WarnBox, {warnIdToClassName} from "./WarnBox"

describe("warnIdToClassName", () => {
    it("sanitizes dots in the warning id", () => {
        expect(warnIdToClassName("errors.implicit.underVote")).toBe(
            "warn--errors-implicit-underVote"
        )
    })

    it("keeps alphanumerics, underscores and hyphens", () => {
        expect(warnIdToClassName("errors_a-b1")).toBe("warn--errors_a-b1")
    })
})

describe("WarnBox", () => {
    it("adds an id-derived class and data attributes when warnId is set", () => {
        const markup = renderToStaticMarkup(
            <WarnBox variant="warning" warnId="errors.implicit.underVote" warnType="Implicit">
                Under vote
            </WarnBox>
        )

        expect(markup).toContain("warn--errors-implicit-underVote")
        expect(markup).toContain('data-warn-id="errors.implicit.underVote"')
        expect(markup).toContain('data-warn-type="Implicit"')
    })

    it("forwards className and id to the root container", () => {
        const markup = renderToStaticMarkup(
            <WarnBox className="my-class" id="my-id">
                Message
            </WarnBox>
        )

        expect(markup).toContain("my-class")
        expect(markup).toContain('id="my-id"')
    })

    it("renders no CSS hook attributes when warnId is not set", () => {
        const markup = renderToStaticMarkup(<WarnBox variant="warning">Message</WarnBox>)

        expect(markup).not.toContain("warn--")
        expect(markup).not.toContain("data-warn-id")
        expect(markup).not.toContain("data-warn-type")
    })
})
