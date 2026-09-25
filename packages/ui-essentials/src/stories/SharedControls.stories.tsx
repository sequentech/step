// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, spyOn, userEvent, waitFor, within} from "storybook/test"
import Candidate, {CandidateProps} from "../components/Candidate/Candidate"
import CandidatesList from "../components/CandidatesList/CandidatesList"
import SelectElection from "../components/SelectElection/SelectElection"
import Dialog from "../components/Dialog/Dialog"
import BallotHash from "../components/BallotHash/BallotHash"
import LanguageMenu from "../components/LanguageMenu/LanguageMenu"
import Header from "../components/Header/Header"
import {EVotingPortalCountdownPolicy} from "@sequentech/ui-core"

interface Args {
    onAction: ReturnType<typeof fn>
}
const meta = {title: "contracts/Shared controls", args: {onAction: fn()}} satisfies Meta<Args>
export default meta
type Story = StoryObj<Args>

const SelectableCandidate = ({onAction, ...props}: Args & Partial<CandidateProps>) => {
    const [checked, setChecked] = useState(false)
    const [writeIn, setWriteIn] = useState("")
    const [position, setPosition] = useState<number | null>(null)
    return (
        <ul style={{listStyle: "none", padding: 0}}>
            <Candidate
                title="Ada"
                isSelectable
                checked={checked}
                {...props}
                setChecked={(value) => {
                    setChecked(value)
                    onAction(value)
                }}
                writeInValue={writeIn}
                setWriteInText={(value) => {
                    setWriteIn(value)
                    onAction(value)
                }}
                selectedPosition={position}
                handlePreferentialChange={(value) => {
                    setPosition(value)
                    onAction(value)
                }}
            />
        </ul>
    )
}

export const CandidateCheckbox: Story = {
    render: (args) => <SelectableCandidate {...args} />,
    play: async ({canvasElement, args}) => {
        const checkbox = within(canvasElement).getByRole("checkbox", {name: "Ada"})
        await userEvent.click(checkbox)
        await expect(checkbox).toBeChecked()
        await expect(args.onAction).toHaveBeenCalledTimes(1)
        await expect(args.onAction).toHaveBeenLastCalledWith(true)
    },
}
export const DisabledCandidate: Story = {
    render: (args) => <SelectableCandidate {...args} shouldDisable />,
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("checkbox", {name: "Ada"})).toBeDisabled()
        await userEvent.click(canvas.getByText("Ada"))
        await expect(args.onAction).not.toHaveBeenCalled()
    },
}
export const WriteInWithError: Story = {
    render: (args) => (
        <>
            <SelectableCandidate
                {...args}
                title="Other candidate"
                isWriteIn
                isInvalidWriteIn
                writeInErrorId="write-in-error"
            />
            <p id="write-in-error">The name exceeds the permitted length.</p>
        </>
    ),
    play: async ({canvasElement, args}) => {
        const input = within(canvasElement).getByRole("textbox", {name: /Other candidate/})
        await expect(input).toHaveAccessibleDescription("The name exceeds the permitted length.")
        await expect(input).toHaveAttribute("aria-invalid", "true")
        await userEvent.type(input, "A")
        await expect(input).toHaveValue("A")
        await expect(args.onAction).toHaveBeenCalledTimes(1)
        await expect(args.onAction).toHaveBeenLastCalledWith("A")
    },
}
export const PreferentialPositions: Story = {
    render: (args) => (
        <SelectableCandidate {...args} isPreferentialVote totalCandidates={5} maxVotes={2} />
    ),
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await userEvent.click(canvas.getByRole("combobox", {name: /Ada/}))
        const menu = within(document.body)
        await expect(menu.getAllByRole("option")).toHaveLength(3)
        await userEvent.click(menu.getByRole("option", {name: "2nd"}))
        await expect(args.onAction).toHaveBeenCalledTimes(1)
        await expect(args.onAction).toHaveBeenLastCalledWith(2)
        await expect(canvas.getByRole("combobox")).toHaveTextContent("2nd")
        await userEvent.click(canvas.getByRole("combobox"))
        await userEvent.click(menu.getByRole("option", {name: "None"}))
        await expect(args.onAction).toHaveBeenNthCalledWith(2, 0)
        await expect(canvas.getByRole("combobox")).toHaveTextContent("Position")
    },
}
export const ExpandingPreservesSelection: Story = {
    render: (args) => (
        <CandidatesList
            title="Council"
            isActive
            isCheckable
            checked={false}
            setChecked={args.onAction}
            isCollapsible
            defaultExpanded={false}
            showCandidatesLabel="Show candidates"
            hideCandidatesLabel="Hide candidates"
        >
            <Candidate title="Ada" />
        </CandidatesList>
    ),
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "Show candidates"}))
        await waitFor(() => expect(canvas.getByText("Ada")).toBeVisible())
        await expect(canvas.getByRole("button", {name: "Hide candidates"})).toHaveAttribute(
            "aria-expanded",
            "true"
        )
        await expect(canvas.getByRole("checkbox", {name: /Council/})).not.toBeChecked()
        await expect(args.onAction).not.toHaveBeenCalled()
    },
}
export const OpenElection: Story = {
    render: (args) => (
        <div role="list">
            <SelectElection
                title="Council"
                isOpen
                isStarted
                hasVoted={false}
                isActive
                onClickToVote={args.onAction}
            />
        </div>
    ),
    play: async ({canvasElement, args}) => {
        await userEvent.click(within(canvasElement).getByRole("button", {name: /Vote.*Council/}))
        await expect(args.onAction).toHaveBeenCalledTimes(1)
    },
}
export const ClosedElection: Story = {
    render: (args) => (
        <div role="list">
            <SelectElection
                title="Council"
                isOpen={false}
                isStarted
                isActive={false}
                hasVoted
                onClickToVote={args.onAction}
                onClickBallotLocator={args.onAction}
                resultsUrl="/results/council"
            />
        </div>
    ),
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("Voted")).toBeVisible()
        await expect(canvas.queryByRole("button", {name: /^Vote/})).not.toBeInTheDocument()
        await expect(canvas.getByRole("link", {name: /Results/})).toHaveAttribute(
            "href",
            "/results/council"
        )
        await userEvent.click(canvas.getByRole("button", {name: /Locate.*Council/}))
        await expect(args.onAction).toHaveBeenCalledTimes(1)
    },
}
export const ConfirmDialog: Story = {
    render: (args) => (
        <Dialog
            open
            title="Publish election"
            ok="Publish"
            cancel="Cancel"
            handleClose={args.onAction}
        >
            Ready to publish.
        </Dialog>
    ),
    play: async ({args}) => {
        const dialog = within(within(document.body).getByRole("dialog", {name: "Publish election"}))
        await userEvent.click(dialog.getByRole("button", {name: "Publish"}))
        await expect(args.onAction).toHaveBeenCalledTimes(1)
        await expect(args.onAction).toHaveBeenLastCalledWith(true)
    },
}
export const CancelDialog: Story = {
    ...ConfirmDialog,
    play: async ({args}) => {
        await userEvent.click(within(document.body).getByRole("button", {name: "Cancel"}))
        await expect(args.onAction).toHaveBeenCalledTimes(1)
        await expect(args.onAction).toHaveBeenLastCalledWith(false)
    },
}
export const DisabledConfirmation: Story = {
    render: (args) => (
        <Dialog
            open
            title="Publish election"
            ok="Publish"
            okEnabled={() => false}
            handleClose={args.onAction}
        >
            Select an election first.
        </Dialog>
    ),
    play: async ({args}) => {
        await expect(within(document.body).getByRole("button", {name: "Publish"})).toBeDisabled()
        await expect(args.onAction).not.toHaveBeenCalled()
    },
}
export const CopyBallotIdentifier: Story = {
    render: () => (
        <BallotHash
            hash="0123456789abcdef"
            copyLabels={{copy: "Copy ballot ID", copied: "Copied ballot ID", error: "Copy failed"}}
        />
    ),
    play: async ({canvasElement}) => {
        const writeText = spyOn(navigator.clipboard, "writeText").mockResolvedValue(undefined)
        const canvas = within(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "Copy ballot ID"}))
        await expect(writeText).toHaveBeenCalledTimes(1)
        await expect(writeText).toHaveBeenLastCalledWith("0123456789abcdef")
        await expect(canvas.getByRole("status", {name: ""})).toHaveTextContent("Copied ballot ID")
        writeText.mockRestore()
    },
}
export const ChangeLanguage: Story = {
    render: (args) => <LanguageMenu languagesList={["en", "es"]} onChange={args.onAction} />,
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "English"}))
        await userEvent.click(within(document.body).getByRole("menuitem", {name: "Español"}))
        await expect(canvas.getByRole("button", {name: "Español"})).toBeVisible()
        await expect(args.onAction).toHaveBeenCalledTimes(1)
        await expect(args.onAction).toHaveBeenLastCalledWith("es")
    },
}
export const LogoutConfirmation: Story = {
    render: (args) => <Header userProfile={{username: "Voter"}} logoutFn={args.onAction} />,
    play: async ({canvasElement, args}) => {
        await userEvent.click(within(canvasElement).getByRole("button", {name: /Voter/}))
        await userEvent.click(within(document.body).getByRole("menuitem", {name: "Logout"}))
        await expect(args.onAction).not.toHaveBeenCalled()
        const dialog = within(within(document.body).getByRole("dialog"))
        await userEvent.click(dialog.getByRole("button", {name: "OK"}))
        await expect(args.onAction).toHaveBeenCalledTimes(1)
    },
}
const CLOCK = Date.parse("2026-01-15T12:00:00Z")
const freezeClock = () => {
    const originalDate = Date
    globalThis.Date = new Proxy(originalDate, {
        construct: (target, args) => Reflect.construct(target, args.length ? args : [CLOCK]),
        apply: () => new originalDate(CLOCK).toString(),
        get: (target, property, receiver) =>
            property === "now" ? () => CLOCK : Reflect.get(target, property, receiver),
    })
    return () => {
        globalThis.Date = originalDate
    }
}

export const SessionExpiryAlert: Story = {
    beforeEach: freezeClock,
    render: (args) => (
        <Header
            userProfile={{username: "Voter"}}
            logoutFn={args.onAction}
            expiry={{
                endTime: new Date(CLOCK + 10000),
                duration: 60,
                alertAt: 30,
                countdownAt: 60,
                countdown: EVotingPortalCountdownPolicy.COUNTDOWN_WITH_ALERT,
            }}
        />
    ),
    play: async () => {
        await waitFor(() => expect(within(document.body).getByRole("dialog")).toBeVisible())
    },
}

export const ElectionNotStarted: Story = {
    beforeEach: freezeClock,
    render: (args) => (
        <div role="list">
            <SelectElection
                title="Council"
                isOpen={false}
                isStarted={false}
                isActive
                hasVoted={false}
                onClickToVote={args.onAction}
                electionDates={{first_started_at: "2026-01-15T12:05:00Z"}}
            />
        </div>
    ),
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText(/5 minutes/)).toBeVisible()
        await expect(canvas.queryByRole("button", {name: /^Vote/})).not.toBeInTheDocument()
        await userEvent.click(canvas.getByText("Council"))
        await expect(args.onAction).not.toHaveBeenCalled()
    },
}
export const SessionCountdownWithoutAlert: Story = {
    beforeEach: freezeClock,
    render: (args) => (
        <Header
            userProfile={{username: "Voter"}}
            logoutFn={args.onAction}
            expiry={{
                endTime: new Date(CLOCK + 10000),
                duration: 60,
                alertAt: 30,
                countdownAt: 60,
                countdown: EVotingPortalCountdownPolicy.COUNTDOWN,
            }}
        />
    ),
    play: async ({canvasElement}) => {
        await userEvent.click(within(canvasElement).getByRole("button", {name: /Voter/}))
        await waitFor(() =>
            expect(within(document.body).getByRole("menuitem", {name: "Logout"})).toBeVisible()
        )
        await expect(within(document.body).queryByRole("dialog")).not.toBeInTheDocument()
    },
}
export const SessionWithoutCountdown: Story = {
    ...SessionCountdownWithoutAlert,
    render: (args) => (
        <Header
            userProfile={{username: "Voter"}}
            logoutFn={args.onAction}
            expiry={{
                endTime: new Date(CLOCK + 10000),
                duration: 60,
                alertAt: 30,
                countdownAt: 60,
                countdown: EVotingPortalCountdownPolicy.NO_COUNTDOWN,
            }}
        />
    ),
}

export const InvalidVoteSelection: Story = {
    render: (args) => <SelectableCandidate {...args} isInvalidVote title="Invalidate my ballot" />,
    play: async ({canvasElement, args}) => {
        const checkbox = within(canvasElement).getByRole("checkbox", {name: "Invalidate my ballot"})
        await userEvent.click(checkbox)
        await expect(checkbox).toBeChecked()
        await expect(args.onAction).toHaveBeenCalledTimes(1)
        await expect(args.onAction).toHaveBeenLastCalledWith(true)
    },
}
