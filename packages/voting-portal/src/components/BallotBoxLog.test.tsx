// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {fireEvent, render, screen, waitFor} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import type {IBallotLogRow} from "@sequentech/ui-core"
import {BallotBoxLog, BallotBoxLogView, type BallotBoxLogViewProps} from "./BallotBoxLog"

jest.mock("react-i18next", () => ({useTranslation: () => ({t: (key: string) => key})}))
jest.mock("./BallotBoxEntry", () => ({BallotBoxEntry: () => null}))
jest.mock("@apollo/client/react", () => ({useQuery: (...args: unknown[]) => mockQuery(...args)}))
const mockQuery = jest.fn()

const FULL_TRACKING_ID = "FTBE-MHRX-0123-4567-89AB-CDEF-GHJK-MNPQ-RSTV-WXYZ-0123-4567-89AB"
const ballot: IBallotLogRow = {
    tracking_id: FULL_TRACKING_ID,
    pseudonym: "K7QM-2XWD",
    stage: "cast",
    last_step_at: "2028-05-08T03:00:00Z",
    voting_channel: "ONLINE",
    election_id: "election",
}
const received: IBallotLogRow = {
    tracking_id: "9ABC-DEFG-HJKM-NPQR-STVW-XYZ0-1234-5678-9ABC-DEFG-HJKM-NPQR-STVW",
    pseudonym: "Z9Y8-X7W6",
    stage: "received",
    last_step_at: "2028-05-08T04:00:00Z",
    voting_channel: "",
    election_id: "election",
}
const props = (): BallotBoxLogViewProps => ({
    page: {
        ballots: [ballot],
        next_cursor: "next",
        total: 12000,
        match_count: 10000,
        match_count_capped: true,
    },
    ownBallots: [ballot],
    loading: false,
    error: false,
    search: "FT",
    onSearch: jest.fn(),
    limit: 10,
    onLimit: jest.fn(),
    canPrevious: false,
    onPrevious: jest.fn(),
    onNext: jest.fn(),
    onRetry: jest.fn(),
    onOpen: jest.fn(),
    formatTime: () => "03:00 UTC",
})

test("own ballot checks open the public proof without identity data and bounded search wording", async () => {
    const input = props()
    render(<BallotBoxLogView {...input} />)
    expect(screen.getByText("ballotPrivacy.capped")).toBeInTheDocument()
    expect(screen.getByRole("button", {name: "ballotPrivacy.previous"})).toBeDisabled()
    await userEvent.click(screen.getByRole("button", {name: "ballotPrivacy.check"}))
    expect(input.onOpen).toHaveBeenCalledWith(ballot)
    // The Tracking ID is shown short.
    expect(document.querySelector(".ballot-box-log-own-id")).toHaveTextContent("FTBE-MHRX")
    await userEvent.click(screen.getByRole("button", {name: "ballotPrivacy.next"}))
    expect(input.onNext).toHaveBeenCalledTimes(1)
    expect(document.body.textContent).not.toContain("voter_id")
})

test("invalid prefix never displays stale rows or unrestricted results", () => {
    render(<BallotBoxLogView {...props()} ownBallots={[]} search="%" />)
    expect(screen.getByText("ballotPrivacy.invalidPrefix")).toBeInTheDocument()
    expect(screen.queryByText("FTBE-MHRX")).not.toBeInTheDocument()
})

test("a hyphen needs the 8 characters of a Tracking ID, and a colon is a pseudonym", () => {
    const {rerender} = render(<BallotBoxLogView {...props()} search="FTBE-MH" />)
    expect(screen.getByText("ballotPrivacy.invalidPrefix")).toBeInTheDocument()
    rerender(<BallotBoxLogView {...props()} search="K7QM:" />)
    expect(screen.queryByText("ballotPrivacy.invalidPrefix")).not.toBeInTheDocument()
    expect(screen.getAllByRole("button", {name: "FTBE-MHRX"}).length).toBeGreaterThan(0)
})

test("rows show the short Tracking ID, the pseudonym with a colon and the last step", () => {
    const input = props()
    render(
        <BallotBoxLogView
            {...input}
            search=""
            ownBallots={[]}
            page={{...input.page!, ballots: [ballot, received], match_count_capped: false}}
        />
    )
    const rows = document.querySelectorAll(".ballot-box-log-row")
    expect(rows).toHaveLength(2)
    expect(rows[0]).toHaveTextContent("FTBE-MHRX")
    expect(rows[0]).not.toHaveTextContent("FTBE-MHRX-0123")
    expect(rows[0]).toHaveTextContent("K7QM:2XWD")
    expect(rows[0]).not.toHaveTextContent("K7QM-2XWD")
    expect(rows[0].querySelector(".ballot-box-log-stage-cast")).toHaveTextContent(
        "ballotPrivacy.stages.cast.label"
    )
    expect(rows[0]).toHaveTextContent("ballotPrivacy.channels.online")
    expect(rows[1].querySelector(".ballot-box-log-stage-received")).toHaveTextContent(
        "ballotPrivacy.stages.received.label"
    )
    expect(rows[1]).toHaveTextContent("9ABC-DEFG")
    // A ballot that is not cast has no channel yet.
    expect(rows[1]).not.toHaveTextContent("ballotPrivacy.channels")
    expect(document.body.textContent).not.toContain(FULL_TRACKING_ID)
})

test("the last step is a neutral tag, not a warning colour", () => {
    render(<BallotBoxLogView {...props()} search="" ownBallots={[]} />)
    const tag = document.querySelector(".ballot-box-log-stage")!
    expect(tag).toBeInTheDocument()
    const style = getComputedStyle(tag)
    expect(style.backgroundColor).not.toMatch(/255, 193|255, 243|254, 243/)
    expect(tag.className).not.toMatch(/warning/i)
})

test("a ballot of the voter is marked by its Tracking ID", () => {
    const input = props()
    render(
        <BallotBoxLogView
            {...input}
            search=""
            page={{...input.page!, ballots: [ballot, received], match_count_capped: false}}
        />
    )
    const rows = document.querySelectorAll(".ballot-box-log-row")
    expect(rows[0].querySelector(".ballot-box-log-yours")).toBeInTheDocument()
    expect(rows[1].querySelector(".ballot-box-log-yours")).not.toBeInTheDocument()
})

test("a search that looks at both says how many match each way", () => {
    const input = props()
    render(
        <BallotBoxLogView
            {...input}
            search="K7QM2XWD"
            page={{
                ...input.page!,
                match_count: 3,
                match_count_capped: false,
                tracking_matches: 2,
                pseudonym_matches: 1,
            }}
        />
    )
    expect(screen.getByText("ballotPrivacy.matchesBoth")).toBeInTheDocument()
})

test("a search that looks at one thing shows no split of the matches", () => {
    const input = props()
    render(
        <BallotBoxLogView
            {...input}
            search="K7QM:2X"
            page={{...input.page!, match_count: 1, match_count_capped: false}}
        />
    )
    expect(screen.queryByText("ballotPrivacy.matchesBoth")).not.toBeInTheDocument()
})

test("the characters a search matched are marked in the Tracking ID or the pseudonym", () => {
    const {rerender} = render(<BallotBoxLogView {...props()} search="ftbe mhrx" />)
    const marked = () =>
        Array.from(document.querySelectorAll(".ballot-box-log-match")).map(
            (mark) => mark.textContent
        )
    // Both are searched with no separator, so the code of the other kind does not match.
    expect(marked()).toContain("FTBE-MHRX")
    rerender(<BallotBoxLogView {...props()} search="k7qm:2x" />)
    expect(marked()).toContain("K7QM:2X")
    expect(marked()).not.toContain("FTBE-MHRX")
    rerender(<BallotBoxLogView {...props()} search="ftbe-mhrx" />)
    expect(marked()).toContain("FTBE-MHRX")
    expect(marked().some((text) => text?.includes(":"))).toBe(false)
})

test("a cast ballot's help uses the confirmation explanation", async () => {
    render(<BallotBoxLogView {...props()} />)
    await userEvent.click(screen.getByRole("button", {name: "ballotPrivacy.ballotIdHelp"}))
    expect(screen.getByRole("dialog")).toHaveTextContent(
        "confirmationScreen.ballotIdHelpDialog.content"
    )
    expect(screen.getByRole("dialog")).not.toHaveTextContent(
        "reviewScreen.ballotIdHelpDialog.content"
    )
})

test("public rows remain usable when the private own-ballot lookup is unavailable", () => {
    mockQuery.mockImplementation((_query, options) =>
        "limit" in options.variables
            ? {data: {ballot_box_log: {payload: props().page}}, loading: false}
            : {error: new Error("Mapping no longer retained"), loading: false}
    )
    render(<BallotBoxLog electionEventId="event" formatTime={() => "03:00 UTC"} />)
    expect(screen.getAllByRole("button", {name: "FTBE-MHRX"}).length).toBeGreaterThan(0)
    expect(screen.queryByText("ballotPrivacy.own")).not.toBeInTheDocument()
})

test("copying an own Tracking ID reports success and clipboard failure without exposing identity", async () => {
    const user = userEvent.setup()
    const write = jest.spyOn(navigator.clipboard, "writeText")
    write.mockResolvedValueOnce(undefined).mockRejectedValueOnce(new Error("Clipboard denied"))
    try {
        render(<BallotBoxLogView {...props()} />)
        await user.click(screen.getByRole("button", {name: "ballotPrivacy.copy"}))
        expect(write).toHaveBeenCalledWith(FULL_TRACKING_ID)
        expect(await screen.findByText("ballotPrivacy.copied")).toBeVisible()
        await user.click(screen.getByRole("button", {name: "ballotPrivacy.copy"}))
        expect(await screen.findByText("ballotPrivacy.copyFailed")).toBeVisible()
    } finally {
        write.mockRestore()
    }
})

test("privacy help explains pseudonyms and signatures and closes without leaving the log", async () => {
    const user = userEvent.setup()
    render(<BallotBoxLogView {...props()} />)
    await user.click(screen.getByRole("button", {name: "ballotPrivacy.help"}))
    expect(screen.getByRole("dialog")).toHaveTextContent(
        "ballotPrivacy.helpSections.pseudonyms.text"
    )
    expect(screen.getByRole("dialog")).toHaveTextContent(
        "ballotPrivacy.helpSections.signatures.text"
    )
    await user.click(screen.getByRole("button", {name: "ballotPrivacy.close"}))
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument())
    expect(screen.getByRole("textbox", {name: "ballotPrivacy.search"})).toBeVisible()
})

test("search and page-size controls forward user choices and opening a row fires only once", async () => {
    const input = props()
    const user = userEvent.setup()
    render(<BallotBoxLogView {...input} />)
    fireEvent.change(screen.getByRole("textbox", {name: "ballotPrivacy.search"}), {
        target: {value: "K7QM:"},
    })
    expect(input.onSearch).toHaveBeenCalledWith("K7QM:")
    await user.click(screen.getByRole("combobox", {name: "ballotPrivacy.rows"}))
    await user.click(screen.getByRole("option", {name: "25"}))
    expect(input.onLimit).toHaveBeenCalledWith(25)
    // The nested button must not bubble to the clickable desktop row.
    fireEvent.click(document.querySelector(".ballot-box-log-check-button")!)
    expect(input.onOpen).toHaveBeenCalledTimes(1)
    fireEvent.click(document.querySelector(".ballot-box-log-row")!)
    expect(input.onOpen).toHaveBeenCalledTimes(2)
    expect(input.onOpen).toHaveBeenLastCalledWith(ballot)
})
