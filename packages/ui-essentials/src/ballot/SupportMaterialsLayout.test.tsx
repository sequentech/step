// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * The support materials tab, and the card it stacks.
 *
 * The third of the portal's screens lifted so the Election Architect's preview
 * can show it rather than describe it. The card is the part worth testing: the
 * portal's version is 244 lines that read a thumbnail out of the store by
 * `document_id`, and what came out of it is the row — an icon chosen by the
 * document's kind, a title, a subtitle, and a way in.
 */

import {ThemeProvider} from "@mui/material/styles"
import {render as mount, screen} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import React from "react"

import theme from "../services/theme"
import {SupportMaterialCard, SupportMaterialsLayout} from "./SupportMaterialsLayout"

const render = (ui: React.ReactElement) =>
    mount(<ThemeProvider theme={theme}>{ui}</ThemeProvider>)

describe("a document in the support materials list", () => {
    it("picks its icon from the kind, by substring", () => {
        // The value arrives as a full content type — `application/pdf`,
        // `video/mp4` — so equality would never match and every document would
        // get the generic icon. Asserted through `data-testid`, which is MUI's
        // own naming for its icons.
        const kinds = [
            ["image/png", "ImageIcon"],
            ["application/pdf", "PictureAsPdfIcon"],
            ["video/mp4", "VideoFileIcon"],
            ["audio/mpeg", "AudioFileIcon"],
            ["application/vnd.oasis.opendocument.text", "DescriptionIcon"],
        ] as const

        for (const [kind, icon] of kinds) {
            const drawn = render(<SupportMaterialCard title="Rules" kind={kind} />)
            expect(
                drawn.container.querySelector(`[data-testid="${icon}"]`)
            ).not.toBeNull()
            drawn.unmount()
        }
    })

    it("shows the title and subtitle it is given", () => {
        render(
            <SupportMaterialCard
                title="Candidate statements"
                subtitle="Two pages each"
                kind="application/pdf"
            />
        )

        expect(screen.getByText("Candidate statements")).toBeInTheDocument()
        expect(screen.getByText("Two pages each")).toBeInTheDocument()
    })

    it("draws no way in when there is nowhere to go", () => {
        // The preview's case: the documents in a plan have not been uploaded, so
        // there is no URL to open. Omitted rather than disabled — a disabled
        // control promises it would work under some condition the reader is left
        // to guess at, and here there is none.
        render(<SupportMaterialCard title="Rules" kind="application/pdf" />)

        expect(screen.queryByRole("button", {name: "Open"})).toBeNull()
    })

    it("opens when it is given somewhere to go", async () => {
        const opened = jest.fn()
        render(
            <SupportMaterialCard
                title="Rules"
                kind="application/pdf"
                onOpen={opened}
                openLabel="Open"
            />
        )

        await userEvent.click(screen.getByRole("button", {name: "Open"}))

        expect(opened).toHaveBeenCalledTimes(1)
    })
})

describe("the support materials tab", () => {
    it("stacks whatever cards it is handed", () => {
        render(
            <SupportMaterialsLayout title="Before you vote">
                <SupportMaterialCard title="Rules" kind="application/pdf" />
                <SupportMaterialCard title="A guide" kind="video/mp4" />
            </SupportMaterialsLayout>
        )

        expect(screen.getByText("Rules")).toBeInTheDocument()
        expect(screen.getByText("A guide")).toBeInTheDocument()
    })

    it("takes the breadcrumb and the way back from its caller", () => {
        // Only the host knows where back is: the portal navigates to its
        // election chooser and a preview has nowhere to go at all.
        render(
            <SupportMaterialsLayout
                title="Before you vote"
                steps={<div data-testid="steps" />}
                back={<div data-testid="back" />}
            />
        )

        expect(screen.getByTestId("steps")).toBeInTheDocument()
        expect(screen.getByTestId("back")).toBeInTheDocument()
    })

    it("draws neither when neither is given", () => {
        render(<SupportMaterialsLayout title="Before you vote" />)

        expect(screen.queryByTestId("steps")).toBeNull()
        expect(screen.queryByTestId("back")).toBeNull()
        expect(screen.getByText("Before you vote")).toBeInTheDocument()
    })
})
