// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * What the confirmation screen puts on the page, and what it leaves off.
 *
 * The sibling of `ReviewLayout.test.tsx`, and pinned for the same reason: this
 * arrangement is now a contract between the voting portal and the Election
 * Architect's preview rather than the inside of one route.
 *
 * The block that matters most here is the ballot identifier. It has two
 * renderings — one for a wide screen and one for a phone, because the raw hash
 * does not fit — and both are always in the DOM with CSS deciding which is seen.
 * A test that asserted "the identifier appears once" would be wrong about the
 * component and would fail the moment somebody looked at it on a phone.
 */

import {ThemeProvider} from "@mui/material/styles"
import {render as mount, screen} from "@testing-library/react"
import React from "react"

import theme from "../services/theme"
import {ConfirmationLayout} from "./ConfirmationLayout"

jest.mock("../components/QRCode/QRCode", () => ({
    __esModule: true,
    default: ({value}: {value: string}) => <div data-testid="stub-qr" data-value={value} />,
}))

const render = (ui: React.ReactElement) => mount(<ThemeProvider theme={theme}>{ui}</ThemeProvider>)

const props = {title: "Your ballot is in", ballotId: "abc123"}

describe("the confirmation screen's arrangement", () => {
    it("shows the identifier twice, once for each width", () => {
        render(<ConfirmationLayout {...props} />)

        expect(screen.getAllByText("abc123")).toHaveLength(2)
    })

    it("says something different on a phone when given something different", () => {
        render(<ConfirmationLayout {...props} ballotIdOnPhone="Ballot abc123" />)

        expect(screen.getByText("abc123")).toBeInTheDocument()
        expect(screen.getByText("Ballot abc123")).toBeInTheDocument()
    })

    it("renders the identifier as plain text when it links nowhere", () => {
        // A demo, and a preview: there is no tracker to send anybody to. An
        // anchor with no `href` is not a link to a browser or a screen reader,
        // which is the behaviour wanted — not a link that goes nowhere.
        const {container} = render(<ConfirmationLayout {...props} />)

        expect(container.querySelector("a[href]")).toBeNull()
    })

    it("links to the tracker when there is one, in a new tab", () => {
        render(<ConfirmationLayout {...props} ballotIdHref="https://verify.example/abc123" />)

        const [wide] = screen.getAllByRole("link")
        expect(wide).toHaveAttribute("href", "https://verify.example/abc123")
        expect(wide).toHaveAttribute("target", "_blank")
    })

    it("leaves out the QR when nothing was given to encode", () => {
        // The preview's case. A QR of an empty string is a scannable square
        // that leads nowhere, which is worse than no square.
        render(<ConfirmationLayout {...props} />)

        expect(screen.queryByTestId("stub-qr")).toBeNull()
    })

    it("encodes exactly what it was handed", () => {
        render(<ConfirmationLayout {...props} qrValue="https://verify.example/abc" />)

        expect(screen.getByTestId("stub-qr")).toHaveAttribute(
            "data-value",
            "https://verify.example/abc"
        )
    })

    it("takes the breadcrumb, the actions and the dialogs from its caller", () => {
        render(
            <ConfirmationLayout
                {...props}
                steps={<div data-testid="steps" />}
                actions={<div data-testid="actions" />}
            >
                <div data-testid="dialogs" />
            </ConfirmationLayout>
        )

        expect(screen.getByTestId("steps")).toBeInTheDocument()
        expect(screen.getByTestId("actions")).toBeInTheDocument()
        expect(screen.getByTestId("dialogs")).toBeInTheDocument()
    })

    it("offers no help buttons unless somebody can answer them", () => {
        const bare = render(<ConfirmationLayout {...props} />)
        // One button is always there: the tick beside the identifier.
        const before = bare.container.querySelectorAll("button").length
        bare.unmount()

        const helped = render(
            <ConfirmationLayout
                {...props}
                onTitleHelp={() => undefined}
                onBallotIdHelp={() => undefined}
            />
        )

        expect(helped.container.querySelectorAll("button").length).toBe(before + 2)
    })
})
