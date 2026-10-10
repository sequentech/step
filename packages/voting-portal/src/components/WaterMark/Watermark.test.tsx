// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {render} from "@testing-library/react"
import {MemoryRouter} from "react-router-dom"
import WatermarkBackground from "./Watermark"

let mockDemo = true
jest.mock("../../store/hooks", () => ({useAppSelector: () => mockDemo}))

/** Every rule on the page; emotion inserts rules without text in its fast mode. */
const stylesheet = () =>
    Array.from(document.styleSheets)
        .flatMap((sheet) => Array.from(sheet.cssRules).map((rule) => rule.cssText))
        .join("\n")

test("a demo tiles the bundled banner, not one at the host's root", () => {
    // The portal also runs below another path, as the voter preview the Election
    // Architect frames, where `/demo-banner.png` is the host's and does not exist.
    mockDemo = true
    const {container} = render(
        <MemoryRouter>
            <WatermarkBackground />
        </MemoryRouter>
    )
    expect(container.querySelector(".watermark-background")).not.toBeNull()
    // jest resolves images to this module path; a bundler to the asset's own URL.
    expect(stylesheet()).toContain("url(static-asset)")
    expect(stylesheet()).not.toContain("/demo-banner.png")
})

test("an election that is not a demo has no watermark", () => {
    mockDemo = false
    const {container} = render(
        <MemoryRouter>
            <WatermarkBackground />
        </MemoryRouter>
    )
    expect(container.querySelector(".watermark-background")).toBeNull()
})
