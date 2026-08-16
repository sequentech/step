// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * What the review screen puts on the page, and what it leaves off.
 *
 * `ReviewLayout` was lifted out of the voting portal's `ReviewScreen` so the
 * Election Architect's preview can render the same screen instead of a drawing
 * of it. These tests pin the arrangement — which blocks appear, in what order,
 * and which are dropped when their prop is absent — because that arrangement is
 * now the contract between two applications rather than the inside of one route.
 *
 * **`Question` is stubbed.** It has its own characterisation tests next door in
 * the portal, it needs the ballot engine and a WASM stub to render at all, and
 * what is being tested here is the frame around it. The stub records the props
 * it was handed, which is the part the layout is responsible for.
 */

import {ThemeProvider} from "@mui/material/styles"
import {render as mount, screen} from "@testing-library/react"
import React from "react"

import theme from "../services/theme"
import {ReviewLayout} from "./ReviewLayout"

/**
 * Inside the app's theme, because the components this draws read it.
 *
 * `BallotHash` interpolates `theme.palette.green.dark` into a styled component,
 * so an unwrapped render throws on the palette rather than failing an assertion —
 * which reads as the layout being broken when it is the harness that is.
 */
const render = (ui: React.ReactElement) => mount(<ThemeProvider theme={theme}>{ui}</ThemeProvider>)

const drawn: Array<Record<string, unknown>> = []

jest.mock("./Question", () => ({
    Question: (props: Record<string, unknown>) => {
        drawn.push(props)
        const contest = props.question as {id: string}
        return <div data-testid={`question-${contest.id}`} />
    },
}))

// Keys rather than sentences, but the *values* still land: `BallotHash` puts the
// identifier in through an interpolation, and a mock that returned the bare key
// would make "the identifier is on the page" untestable while looking fine.
jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, values?: Record<string, unknown>): string =>
            values === undefined ? key : `${key} ${Object.values(values).join(" ")}`,
    }),
}))

const aContest = (id: string) => ({id, title: `Contest ${id}`, candidates: []}) as never

const props = {
    ballotStyle: {} as never,
    contests: [aContest("first"), aContest("second")],
    errorSelectionState: {} as never,
    title: "Review your ballot",
}

beforeEach(() => {
    drawn.length = 0
})

describe("the review screen's arrangement", () => {
    it("draws one contest per contest, in the order given", () => {
        render(<ReviewLayout {...props} />)

        expect(screen.getByTestId("question-first")).toBeInTheDocument()
        expect(screen.getByTestId("question-second")).toBeInTheDocument()
        expect(drawn.map((each) => (each.question as {id: string}).id)).toEqual(["first", "second"])
    })

    it("reviews rather than collects", () => {
        // `isReview` is what turns a contest from a set of controls into a
        // summary of what was chosen. A review screen that passed `false` here
        // would offer the voter a second, silent chance to change their ballot
        // after they thought they had finished with it.
        render(<ReviewLayout {...props} />)

        expect(drawn.every((each) => each.isReview === true)).toBe(true)
    })

    it("leaves out the ballot identifier when there is none", () => {
        // Two callers need this and for different reasons: the portal when the
        // audit configuration says `NOT_SHOW`, and the preview always, because
        // no ballot has been cast to have an identifier. Drawing an empty one
        // would be worse than drawing nothing — it reads as a value that failed
        // to load.
        const {container} = render(<ReviewLayout {...props} />)

        expect(container.querySelector(".ballot-hash")).toBeNull()
    })

    it("shows the identifier when it is given one", () => {
        render(<ReviewLayout {...props} ballotId="ballot-42" />)

        expect(screen.getByText(/ballot-42/)).toBeInTheDocument()
    })

    it("offers no help button unless somebody can answer it", () => {
        // The icon is wired to a dialog the *caller* owns. Rendering it with no
        // handler would give a voter a button that does nothing.
        const plain = render(<ReviewLayout {...props} />)
        const before = plain.container.querySelectorAll("button").length
        plain.unmount()

        const helped = render(<ReviewLayout {...props} onTitleHelp={() => undefined} />)

        expect(helped.container.querySelectorAll("button").length).toBe(before + 1)
    })

    it("puts the error above the contests, not after them", () => {
        // A refusal to cast that appears below two screens of candidates is a
        // refusal nobody sees.
        render(<ReviewLayout {...props} error="Casting was refused" />)

        const error = screen.getByText("Casting was refused")
        const first = screen.getByTestId("question-first")

        expect(error.compareDocumentPosition(first) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
    })

    it("takes the breadcrumb and the actions from its caller", () => {
        // Neither is the layout's to build. The portal's breadcrumb counts an
        // election-list step the preview does not have, and its actions cast a
        // vote — which is the one thing a preview must never offer.
        render(
            <ReviewLayout
                {...props}
                steps={<div data-testid="steps" />}
                actions={<div data-testid="actions" />}
            />
        )

        expect(screen.getByTestId("steps")).toBeInTheDocument()
        expect(screen.getByTestId("actions")).toBeInTheDocument()
    })

    it("draws nothing where a caller supplies nothing", () => {
        render(<ReviewLayout {...props} />)

        expect(screen.queryByTestId("steps")).toBeNull()
        expect(screen.queryByTestId("actions")).toBeNull()
    })
})
