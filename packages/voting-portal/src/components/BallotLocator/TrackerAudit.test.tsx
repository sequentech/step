/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// What the ballot tracker shows under the record of an audited ballot (design
// boards TrackerAudit and PhoneTrackerAudit): the selections it decodes into,
// or why there are none.

import React from "react"
import {render, screen, within} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import type {IBallotStyle, IDecodedVoteContest} from "@sequentech/ui-core"
import {EAuditOutcome, TAudit} from "../../services/vitrine/ballotAudit"
import {TrackerAudit} from "./TrackerAudit"

jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, values?: Record<string, unknown>) =>
            values ? `${key} ${JSON.stringify(values)}` : key,
    }),
}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({
        Candidate: ({
            title,
            description,
            isPreferentialVote,
            selectedPosition,
            isInvalidVote,
            children,
        }: {
            title: string
            description?: React.ReactNode
            isPreferentialVote?: boolean
            selectedPosition?: number | null
            isInvalidVote?: boolean
            children?: React.ReactNode
        }) => (
            <li className="candidate-item" data-invalid={String(Boolean(isInvalidVote))}>
                {children}
                <span className="candidate-title">{title}</span>
                <span className="candidate-description">{description}</span>
                {isPreferentialVote && selectedPosition ? (
                    <span className="candidate-position">{`position ${selectedPosition}`}</span>
                ) : null}
            </li>
        ),
        BlankAnswer: ({title}: {title?: string}) => (
            <li className="blank-answer">{title ?? "candidate.blankVote"}</li>
        ),
    }),
    {virtual: true}
)
jest.mock("../../services/BallotService", () => ({
    provideBallotService: () => ({
        isPreferential: (algorithm?: string) => algorithm === "instant-runoff",
    }),
}))
jest.mock("mui-image", () => ({
    __esModule: true,
    default: ({src}: {src: string}) => <img className="candidate-image" src={src} alt="" />,
}))

const BUCKET = "https://bucket.example/"

const candidate = (id: string, extra: Record<string, unknown> = {}) => ({
    id,
    name: `Name of ${id}`,
    description: `About ${id}`,
    ...extra,
})

const STYLE = {
    id: "style",
    contests: [
        {
            id: "mayor",
            name: "Mayor",
            name_i18n: {es: "Alcaldía"},
            counting_algorithm: "plurality-at-large",
            candidates: [
                candidate("ortega", {
                    name_i18n: {es: "María Ortega"},
                    presentation: {urls: [{url: "ortega.png", is_image: true, title: "image"}]},
                }),
                candidate("puig"),
                candidate("invalid", {presentation: {is_explicit_invalid: true}}),
                candidate("writein", {presentation: {is_write_in: true}}),
            ],
        },
        {
            id: "council",
            name: "City council",
            counting_algorithm: "plurality-at-large",
            candidates: [candidate("harbour"), candidate("hill")],
        },
        {
            id: "committee",
            name: "Committee",
            counting_algorithm: "instant-runoff",
            candidates: [candidate("a"), candidate("b"), candidate("c")],
        },
    ],
} as unknown as IBallotStyle

const decoded = (
    contestId: string,
    chosen: [string, number][] = [],
    extra: Partial<IDecodedVoteContest> = {}
): IDecodedVoteContest => ({
    contest_id: contestId,
    is_explicit_invalid: false,
    is_decline_to_vote: false,
    is_blank_ballot: false,
    invalid_errors: [],
    invalid_alerts: [],
    choices: chosen.map(([id, selected]) => ({id, selected})),
    ...extra,
})

const verified = (selections: IDecodedVoteContest[]): TAudit => ({
    outcome: EAuditOutcome.VERIFIED,
    selections,
})

const renderAudit = (audit: TAudit, language = "en") =>
    render(
        <TrackerAudit
            audit={audit}
            ballotStyle={STYLE}
            language={language}
            publicBucketUrl={BUCKET}
        />
    )

const contestNamed = (name: string) =>
    screen.getByRole("heading", {name, level: 3}).closest("section") as HTMLElement

const titlesIn = (container: HTMLElement) =>
    Array.from(container.querySelectorAll(".candidate-title")).map((title) => title.textContent)

describe("the selections of a verified audited ballot", () => {
    it("are in a card of their own, with what the portal revealed and what to do if they are not the voter's", () => {
        renderAudit(verified([decoded("mayor", [["puig", 0]])]))
        const card = screen.getByRole("region", {name: "ballotLocator.audit.title"})
        expect(within(card).getByRole("heading", {level: 2})).toHaveTextContent(
            "ballotLocator.audit.title"
        )
        expect(within(card).getByText("ballotLocator.audit.note")).toBeVisible()
    })

    it("lists each contest, in the order of the ballot, with the candidates the voter chose", () => {
        renderAudit(verified([decoded("council", [["hill", 0]]), decoded("mayor", [["puig", 0]])]))
        const headings = screen.getAllByRole("heading", {level: 3}).map((h) => h.textContent)
        expect(headings).toEqual(["Mayor", "City council"])
        expect(titlesIn(contestNamed("Mayor"))).toEqual(["Name of puig"])
        expect(titlesIn(contestNamed("City council"))).toEqual(["Name of hill"])
    })

    it("shows the candidate's description, and only the candidates that were chosen", () => {
        renderAudit(
            verified([
                decoded("council", [
                    ["hill", 0],
                    ["harbour", -1],
                ]),
            ])
        )
        expect(titlesIn(contestNamed("City council"))).toEqual(["Name of hill"])
        expect(screen.getByText("About hill")).toBeVisible()
        expect(screen.queryByText("Name of harbour")).toBeNull()
    })

    it("names contests and candidates in the voter's language where the ballot has it", () => {
        renderAudit(verified([decoded("mayor", [["ortega", 0]])]), "es")
        expect(screen.getByRole("heading", {name: "Alcaldía", level: 3})).toBeVisible()
        expect(screen.getByText("María Ortega")).toBeVisible()
    })

    it("shows a candidate's picture", () => {
        const {container} = renderAudit(verified([decoded("mayor", [["ortega", 0]])]))
        expect(container.querySelector("img.candidate-image")).toHaveAttribute(
            "src",
            `${BUCKET}ortega.png`
        )
    })

    it("shows a blank vote when no candidate was chosen", () => {
        renderAudit(verified([decoded("council")]))
        expect(within(contestNamed("City council")).getByText("candidate.blankVote")).toBeVisible()
    })

    it("shows a contest the voter declined to vote in as such", () => {
        renderAudit(verified([decoded("council", [], {is_decline_to_vote: true})]))
        expect(
            within(contestNamed("City council")).getByText("reviewScreen.declineToVote")
        ).toBeVisible()
    })

    it("shows the explicit invalid vote, when that was the voter's choice, and not otherwise", () => {
        const {container, rerender} = renderAudit(
            verified([decoded("mayor", [], {is_explicit_invalid: true})])
        )
        expect(titlesIn(container)).toEqual(["Name of invalid"])
        expect(container.querySelector(".candidate-item")).toHaveAttribute("data-invalid", "true")
        rerender(
            <TrackerAudit
                audit={verified([
                    decoded("mayor", [
                        ["puig", 0],
                        ["invalid", -1],
                    ]),
                ])}
                ballotStyle={STYLE}
                language="en"
                publicBucketUrl={BUCKET}
            />
        )
        expect(titlesIn(container)).toEqual(["Name of puig"])
    })

    it("puts the candidates of a ranked contest in the order of the ranking, with their positions", () => {
        const {container} = renderAudit(
            verified([
                decoded("committee", [
                    ["a", 2],
                    ["b", 0],
                    ["c", 1],
                ]),
            ])
        )
        expect(titlesIn(container)).toEqual(["Name of b", "Name of c", "Name of a"])
        expect(
            Array.from(container.querySelectorAll(".candidate-position")).map((p) => p.textContent)
        ).toEqual(["position 1", "position 2", "position 3"])
    })

    it("shows what a write-in vote says in place of the candidate's description", () => {
        renderAudit(
            verified([
                {
                    ...decoded("mayor", [["writein", 0]]),
                    choices: [{id: "writein", selected: 0, write_in_text: "Rosa Vidal"}],
                },
            ])
        )
        expect(screen.getByText("Rosa Vidal")).toBeVisible()
        expect(screen.queryByText("About writein")).toBeNull()
    })

    it("leaves out a contest the ballot style doesn't have, and a choice the contest doesn't have", () => {
        const {container} = renderAudit(
            verified([
                decoded("unknown", [["x", 0]]),
                decoded("council", [
                    ["ghost", 0],
                    ["hill", 0],
                ]),
            ])
        )
        expect(screen.getAllByRole("heading", {level: 3})).toHaveLength(1)
        expect(titlesIn(container)).toEqual(["Name of hill"])
    })

    it("shows nothing for an audit without contests but the card itself", () => {
        renderAudit(verified([]))
        expect(screen.getByRole("region", {name: "ballotLocator.audit.title"})).toBeVisible()
        expect(screen.queryAllByRole("heading", {level: 3})).toHaveLength(0)
    })
})

describe("printing the selections", () => {
    it("prints the page, from the Print button", async () => {
        const print = jest.spyOn(window, "print").mockImplementation(() => undefined)
        renderAudit(verified([decoded("council", [["hill", 0]])]))
        await userEvent.click(screen.getByRole("button", {name: "auditScreen.printButton"}))
        expect(print).toHaveBeenCalledTimes(1)
        print.mockRestore()
    })

    it("leaves everything but the card out of the printed page", () => {
        const {container} = renderAudit(verified([decoded("council", [["hill", 0]])]))
        expect(container.querySelector(".tracker-selections")).not.toBeNull()
        const printed = Array.from(document.styleSheets)
            .flatMap((sheet) => Array.from(sheet.cssRules))
            .filter((rule) => rule.cssText.startsWith("@media print"))
            .map((rule) => rule.cssText)
            .join("")
        expect(printed).toContain(":not(:has(.tracker-selections))")
        expect(printed).toContain(":not(.tracker-selections *)")
        expect(printed).toContain("display: none")
    })
})

describe("an audited ballot that does not verify", () => {
    it("tells of the failure, and shows no selections", () => {
        const {container} = renderAudit({outcome: EAuditOutcome.MISMATCH})
        const alert = screen.getByRole("alert")
        expect(alert).toHaveTextContent("ballotLocator.audit.failed.title")
        expect(alert).toHaveTextContent("ballotLocator.audit.failed.text")
        expect(screen.queryByRole("button", {name: "auditScreen.printButton"})).toBeNull()
        expect(screen.queryByText("ballotLocator.audit.title")).toBeNull()
        expect(container.querySelector(".candidate-item")).toBeNull()
    })

    it("says neutrally that the audit could not be checked, and shows no selections", () => {
        const {container} = renderAudit({outcome: EAuditOutcome.NOT_VERIFIABLE})
        expect(screen.queryByRole("alert")).toBeNull()
        const note = screen.getByRole("status")
        expect(note).toHaveTextContent("ballotLocator.audit.unchecked.title")
        expect(note).toHaveTextContent("ballotLocator.audit.unchecked.text")
        expect(screen.queryByRole("button", {name: "auditScreen.printButton"})).toBeNull()
        expect(container.querySelector(".candidate-item")).toBeNull()
    })
})
