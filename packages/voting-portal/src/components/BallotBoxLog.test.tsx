// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {fireEvent, render, screen, waitFor} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import {BallotBoxLog, BallotBoxLogView, type BallotBoxLogViewProps} from "./BallotBoxLog"

jest.mock("react-i18next", () => ({useTranslation: () => ({t: (key: string) => key})}))
jest.mock("./BallotBoxEntry", () => ({BallotBoxEntry: () => null}))
jest.mock("@apollo/client/react", () => ({useQuery: (...args: unknown[]) => mockQuery(...args)}))
const mockQuery = jest.fn()

const ballot = {
    ballot_id: "FTBE-MHRX",
    pseudonym: "7QKM-2HXD",
    cast_at: "2028-05-08T03:00:00Z",
    voting_channel: "ONLINE",
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
    await userEvent.click(screen.getByRole("button", {name: "ballotPrivacy.next"}))
    expect(input.onNext).toHaveBeenCalledTimes(1)
    expect(document.body.textContent).not.toContain("voter_id")
})

test("invalid prefix never displays stale rows or unrestricted results", () => {
    render(<BallotBoxLogView {...props()} ownBallots={[]} search="%" />)
    expect(screen.getByText("ballotPrivacy.invalidPrefix")).toBeInTheDocument()
    expect(screen.queryByText("FTBE-MHRX")).not.toBeInTheDocument()
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

test("copying an own Ballot ID reports success and clipboard failure without exposing identity", async () => {
    const user = userEvent.setup()
    const write = jest.spyOn(navigator.clipboard, "writeText")
    write.mockResolvedValueOnce(undefined).mockRejectedValueOnce(new Error("Clipboard denied"))
    try {
        render(<BallotBoxLogView {...props()} />)
        await user.click(screen.getByRole("button", {name: "ballotPrivacy.copy"}))
        expect(write).toHaveBeenCalledWith("FTBE-MHRX")
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
        target: {value: "7QKM"},
    })
    expect(input.onSearch).toHaveBeenCalledWith("7QKM")
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
