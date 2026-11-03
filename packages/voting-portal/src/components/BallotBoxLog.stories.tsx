// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {atViewport} from "../../../test-support/storybook/viewport"
import React from "react"
import {Box} from "@mui/material"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {BallotBoxLogView} from "./BallotBoxLog"

const ballot = {
    ballot_id: "FTBE-MHRX",
    pseudonym: "7QKM-2HXD",
    cast_at: "2028-05-08T03:00:00Z",
    voting_channel: "ONLINE",
    election_id: "election",
}
const meta = {
    title: "Voting/Ballot box log",
    component: BallotBoxLogView,
    decorators: [
        (Story) => (
            <Box sx={{maxWidth: 1152, mx: "auto", px: {xs: 1, sm: 3}}}>
                <Story />
            </Box>
        ),
    ],
    args: {
        page: {
            ballots: [
                ballot,
                {
                    ...ballot,
                    ballot_id: "E9KM-3RTW",
                    pseudonym: "2BFG-4HJK",
                    voting_channel: "KIOSK",
                },
            ],
            next_cursor: "next-page",
            total: 1284,
            match_count: 1284,
            match_count_capped: false,
        },
        ownBallots: [ballot],
        loading: false,
        error: false,
        search: "",
        limit: 10,
        canPrevious: false,
        onSearch: fn(),
        onLimit: fn(),
        onPrevious: fn(),
        onNext: fn(),
        onRetry: fn(),
        onOpen: fn(),
        formatTime: () => "08 May 2028, 07:00 GST",
        timeZoneLabel: "Gulf Standard Time",
    },
} satisfies Meta<typeof BallotBoxLogView>
export default meta
type Story = StoryObj<typeof meta>

export const Ballots: Story = {
    ...atViewport(1440),
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("heading", {name: "Ballot box log"})).toBeVisible()
        expect(canvas.getByText("Times in Gulf Standard Time")).toBeVisible()
        const rows = canvasElement.querySelector(
            window.innerWidth < 750 ? ".ballot-box-log-mobile-list" : ".ballot-box-log-table"
        )
        expect(rows).toBeVisible()
        const help = canvas.getByRole("button", {name: "About the ballot box log"})
        const title = canvas.getByRole("heading", {name: "Ballot box log"})
        expect(
            Math.abs(help.getBoundingClientRect().top - title.getBoundingClientRect().top)
        ).toBeLessThan(10)
        const list = canvasElement.querySelector(".ballot-box-log-list")
        if (!list) throw new Error("The search and rows need a shared list card")
        expect(
            within(list as HTMLElement).getByRole("textbox", {
                name: "Find a Ballot ID or pseudonym",
            })
        ).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Check my ballot"}))
        expect(args.onOpen).toHaveBeenCalledWith(ballot)
        await userEvent.click(canvas.getByRole("button", {name: "Next"}))
        expect(args.onNext).toHaveBeenCalled()
        expect(canvasElement.scrollWidth).toBeLessThanOrEqual(canvasElement.clientWidth + 1)
    },
}
export const BallotIdHelp: Story = {
    play: async ({canvasElement}) => {
        await userEvent.click(
            within(canvasElement).getByRole("button", {name: "About your Ballot ID"})
        )
        const dialog = await within(document.body).findByRole("dialog")
        await waitFor(() => expect(within(dialog).getByText(/About the Ballot ID/)).toBeVisible())
    },
}
export const CappedSearch: Story = {
    args: {
        search: "E9",
        page: {
            ballots: [],
            next_cursor: null,
            total: 20000,
            match_count: 10000,
            match_count_capped: true,
        },
    },
}
export const NoMatch: Story = {
    args: {
        ownBallots: [],
        search: "ZZ",
        page: {
            ballots: [],
            next_cursor: null,
            total: 1284,
            match_count: 0,
            match_count_capped: false,
        },
    },
}
export const LoadError: Story = {args: {error: true, page: undefined}}

export const BallotsPhone: Story = {
    ...Ballots,
    ...atViewport(390),
}

export const BallotsTablet: Story = {
    ...Ballots,
    ...atViewport(820),
}

export const BallotsSmallTablet: Story = {
    ...Ballots,
    ...atViewport(740),
}

export const CappedSearchPhone: Story = {
    ...CappedSearch,
    ...atViewport(390),
}

export const CappedSearchTablet: Story = {
    ...CappedSearch,
    ...atViewport(820),
}

export const NoMatchPhone: Story = {
    ...NoMatch,
    ...atViewport(390),
}

export const NoMatchTablet: Story = {
    ...NoMatch,
    ...atViewport(820),
}
