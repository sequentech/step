/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {render, screen, cleanup} from "@testing-library/react"
import "@testing-library/jest-dom"
import {readFileSync} from "fs"
import {join} from "path"
import ts from "typescript"
import {ConfirmationScreen} from "../src/screens/ConfirmationScreen"
import {IConfirmationBallot} from "../src/services/BallotService"

jest.mock("react-i18next", () => ({useTranslation: () => ({t: (key: string) => key})}))
jest.mock("react-router-dom", () => {
    const React = require("react")
    return {
        useNavigate: () => jest.fn(),
        Link: React.forwardRef(({to, ...props}: {to: string}, ref: React.Ref<HTMLAnchorElement>) =>
            React.createElement("a", {...props, href: to, ref})
        ),
    }
})
jest.mock("../src/screens/hooks/useElectionClassName", () => ({useElectionClassName: jest.fn()}))
jest.mock("../src/providers/SettingsContextProvider", () => ({
    SettingsContext: require("react").createContext({globalSettings: {PUBLIC_BUCKET_URL: ""}}),
}))
jest.mock("@sequentech/ui-core", () => ({
    sortContestList: (contests: unknown[]) => contests,
    isAcclaimedContest: () => false,
    EDeclineToVotePolicy: {ENABLED: "enabled"},
    EBlankBallotsPolicy: {ENABLED: "enabled"},
    EElectionEventContestEncryptionPolicy: {MULTIPLE_CONTESTS: "multiple-contests"},
}))
jest.mock("@sequentech/ui-essentials", () => ({
    theme: {palette: {customGrey: {main: "#666"}, black: "#000", red: {dark: "#900"}}},
    PageLimit: ({children}: React.PropsWithChildren) => <div>{children}</div>,
    ActionsContainer: ({children}: React.PropsWithChildren) => <div>{children}</div>,
    StyledButton: require("@mui/material/Button").default,
    Icon: () => <span aria-hidden="true" />,
    IconButton: ({ariaLabel}: {ariaLabel?: string}) => (
        <button aria-label={ariaLabel ?? "icon button"} />
    ),
    Dialog: () => null,
    BreadCrumbSteps: () => (
        <ol>
            <li>Import</li>
            <li>Verify</li>
        </ol>
    ),
    PlaintextVoteContest: () => null,
}))

afterEach(cleanup)

const renderConfirmation = (description = "") => {
    const ballot = {
        ballot_hash: "test-ballot-id",
        election_config: {description, contests: []},
        decoded_questions: [],
    } as unknown as IConfirmationBallot
    return render(<ConfirmationScreen confirmationBallot={ballot} ballotId={ballot.ballot_hash} />)
}

it("puts every verifier route inside one main landmark", () => {
    const source = ts.createSourceFile(
        "App.tsx",
        readFileSync(join(__dirname, "../src/App.tsx"), "utf8"),
        ts.ScriptTarget.Latest,
        true,
        ts.ScriptKind.TSX
    )
    const banners: ts.JsxElement[] = []
    const visit = (node: ts.Node) => {
        if (ts.isJsxElement(node) && node.openingElement.tagName.getText(source) === "PageBanner")
            banners.push(node)
        ts.forEachChild(node, visit)
    }
    visit(source)
    expect(banners).toHaveLength(1)
    expect(
        banners[0].openingElement.attributes.properties.some(
            (attribute) =>
                ts.isJsxAttribute(attribute) &&
                attribute.name.getText(source) === "component" &&
                attribute.initializer &&
                ts.isStringLiteral(attribute.initializer) &&
                attribute.initializer.text === "main"
        )
    ).toBe(true)
    expect(
        banners[0].children.some(
            (child) =>
                ts.isJsxElement(child) && child.openingElement.tagName.getText(source) === "Routes"
        )
    ).toBe(true)
})

it("does not emit an empty heading for an absent election description", () => {
    renderConfirmation()
    expect(screen.getAllByRole("heading").every((heading) => heading.textContent?.trim())).toBe(
        true
    )
})

it("renders a supplied election description once", () => {
    renderConfirmation("Test election description")
    expect(screen.getAllByText("Test election description")).toHaveLength(1)
})

it("renders Back as a single link without a nested button", () => {
    renderConfirmation()
    const back = screen.getByRole("link", {name: "confirmationScreen.backButton"})
    expect(back).toHaveAttribute("href", "/")
    expect(back.querySelector("button")).toBeNull()
})

it("names the three help buttons by their dialog titles", () => {
    renderConfirmation()
    for (const name of ["decodedBallotId", "userBallotId", "verifySelections"]) {
        expect(
            screen.getByRole("button", {name: `confirmationScreen.${name}HelpDialog.title`})
        ).toBeInTheDocument()
    }
})

it("hides the SVG.js measurement surface without hiding meaningful SVGs", () => {
    const style = document.createElement("style")
    style.textContent = readFileSync(join(__dirname, "../src/index.css"), "utf8")
    document.head.appendChild(style)
    const createSvg = (id: string, width: string, height: string) => {
        const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg")
        svg.id = id
        svg.setAttribute("width", width)
        svg.setAttribute("height", height)
        document.body.appendChild(svg)
        return svg
    }
    const helper = createSvg("SvgjsSvg1001", "2", "0")
    const chart = createSvg("SvgjsSvg1002", "300", "200")
    const other = createSvg("meaningful-svg", "2", "0")
    try {
        expect(getComputedStyle(helper).visibility).toBe("hidden")
        expect(getComputedStyle(helper).display).not.toBe("none")
        expect(getComputedStyle(chart).visibility).toBe("visible")
        expect(getComputedStyle(other).visibility).toBe("visible")
    } finally {
        helper.remove()
        chart.remove()
        other.remove()
        style.remove()
    }
})
