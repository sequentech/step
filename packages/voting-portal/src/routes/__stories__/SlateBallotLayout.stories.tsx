// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Provider} from "react-redux"
import {ApolloClient, ApolloLink, InMemoryCache} from "@apollo/client"
import {ApolloProvider} from "@apollo/client/react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {
    initCore,
    encryptBallotSelection,
    type BallotSelection,
    type IBallotStyle as BallotDefinition,
    CandidatesOrder,
    EBlankVotePolicy,
    EConsolidatedReportPolicy,
    EBlankBallotsPolicy,
    EInvalidVotePolicy,
    EMobileCandidateLists,
    type IContest,
    type ISlatesConfig,
    SLATES_ANNOTATION,
} from "@sequentech/ui-core"
import {electionFixture, IDS, FIXED_TIME} from "@sequentech/ui-test-kit/fixtures"
import VotingScreen, {action} from "../VotingScreen"
import {clearVoterSession, store} from "../../store/store"
import {setElection, type IElectionExtended} from "../../store/elections/electionsSlice"
import {setBallotStyle, type IBallotStyle} from "../../store/ballotStyles/ballotStylesSlice"
import {
    resetBallotSelection,
    setBallotSelection,
} from "../../store/ballotSelections/ballotSelectionsSlice"
import {setAuditableBallot} from "../../store/auditableBallots/auditableBallotsSlice"
import ReviewScreen from "../ReviewScreen"
import {BallotSelectionAdapter} from "../../components/BallotSelectionAdapter"
import {ErrorPage} from "../ErrorPage"

const MIN_TOUCH_TARGET = 44

const OFFICES: Array<{id: string; name: string; seats: number; candidates: Array<string>}> = [
    {
        id: "president",
        name: "President",
        seats: 1,
        candidates: ["Jordan Ellis", "Morgan Hayes", "Avery Brooks"],
    },
    {
        id: "vice-president",
        name: "Vice President",
        seats: 1,
        candidates: ["Taylor Morgan", "Casey Rivera-Montgomery de la Fuente y Aguirre"],
    },
    {
        id: "secretary-treasurer",
        name: "Secretary-Treasurer",
        seats: 1,
        candidates: ["Riley Chen", "Jamie Patel"],
    },
    {
        id: "recording-secretary",
        name: "Recording Secretary",
        seats: 1,
        candidates: ["Alex Nguyen", "Sam Okafor"],
    },
    {
        id: "trustees",
        name: "Trustees",
        seats: 3,
        candidates: [
            "Rowan Scott",
            "Charlie Kim",
            "Dakota Reed",
            "Skyler James",
            "Finley Ross",
            "Emerson Wright",
            "Harper Lane",
            "Blair Lewis",
        ],
    },
]

const candidateId = (office: string, index: number) => `${office}-${index}`

const slatesConfig = (mobileCandidateLists: EMobileCandidateLists): ISlatesConfig => ({
    version: 1,
    mobile_candidate_lists: mobileCandidateLists,
    slates: [
        {
            id: "forward",
            name: {en: "Forward Together"},
            members: {
                "president": [candidateId("president", 0)],
                "vice-president": [candidateId("vice-president", 0)],
                "secretary-treasurer": [candidateId("secretary-treasurer", 0)],
                "recording-secretary": [candidateId("recording-secretary", 0)],
                "trustees": [0, 1, 2].map((index) => candidateId("trustees", index)),
            },
        },
        {
            id: "members",
            name: {en: "Members First: a voice for every member of the association"},
            members: {
                "president": [candidateId("president", 1)],
                "vice-president": [candidateId("vice-president", 1)],
                "secretary-treasurer": [candidateId("secretary-treasurer", 1)],
                "recording-secretary": [candidateId("recording-secretary", 1)],
                "trustees": [3, 4, 5].map((index) => candidateId("trustees", index)),
            },
        },
        {
            id: "voices",
            name: {en: "Independent Voices"},
            members: {trustees: [candidateId("trustees", 6)]},
        },
    ],
})

/** The second slate, whose long names have to wrap, chosen in full. */
const membersFirstSelection = (contests: Array<IContest>): BallotSelection =>
    contests.map((contest) => ({
        contest_id: contest.id,
        choices: contest.candidates.map((candidate, index) => ({
            id: candidate.id,
            selected: (contest.id === "trustees" ? index >= 3 && index <= 5 : index === 1) ? 0 : -1,
        })),
        is_explicit_invalid: false,
        is_decline_to_vote: false,
        is_blank_ballot: false,
        invalid_errors: [],
        invalid_alerts: [],
    }))

function prepare({mobileCandidateLists, screen}: LayoutArgs) {
    const ballot = electionFixture().ballot as unknown as BallotDefinition
    const template = ballot.contests[0]
    ballot.contests = OFFICES.map(
        (office, order): IContest => ({
            ...template,
            id: office.id,
            name: office.name,
            name_i18n: {en: office.name},
            min_votes: 0,
            max_votes: office.seats,
            winning_candidates_num: office.seats,
            presentation: {
                candidates_order: CandidatesOrder.CUSTOM,
                sort_order: order,
                invalid_vote_policy: EInvalidVotePolicy.NOT_ALLOWED,
                blank_vote_policy: EBlankVotePolicy.ALLOWED,
            },
            candidates: office.candidates.map((name, index) => ({
                ...template.candidates[0],
                id: candidateId(office.id, index),
                contest_id: office.id,
                name,
                name_i18n: {en: name},
                presentation: {...template.candidates[0].presentation, sort_order: index},
            })),
        })
    )
    ballot.election_annotations = {
        ...ballot.election_annotations,
        [SLATES_ANNOTATION]: JSON.stringify(slatesConfig(mobileCandidateLists)),
    }
    const ballotStyle: IBallotStyle = {
        id: ballot.id,
        tenant_id: IDS.tenant,
        election_event_id: IDS.event,
        election_id: IDS.election,
        ballot_eml: ballot,
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
    }
    const election: IElectionExtended = {
        id: IDS.election,
        name: "Association officers",
        tenant_id: IDS.tenant,
        election_event_id: IDS.event,
        image_document_id: "",
        contests: ballot.contests,
        num_allowed_revotes: 0,
        presentation: {
            blank_ballots_policy: EBlankBallotsPolicy.DISABLED,
            consolidated_report_policy: EConsolidatedReportPolicy.DO_NOT_GENERATE,
        },
    }
    store.dispatch(clearVoterSession())
    store.dispatch(setElection(election))
    store.dispatch(setBallotStyle(ballotStyle))
    store.dispatch(resetBallotSelection({ballotStyle, force: true}))
    if (screen === "review") {
        const ballotSelection = membersFirstSelection(ballot.contests)
        store.dispatch(setBallotSelection({ballotStyle, ballotSelection}))
        store.dispatch(
            setAuditableBallot({
                electionId: IDS.election,
                auditableBallot: encryptBallotSelection(ballotSelection, ballot),
                isBlankBallot: false,
            })
        )
    }
}

const electionPath = `/tenant/${IDS.tenant}/event/${IDS.event}/election/${IDS.election}`

const viewport = (width: number) => ({
    name: `${width}px`,
    styles: {width: `${width}px`, height: "900px"},
})

const VIEWPORTS = {
    w320: viewport(320),
    w390: viewport(390),
    w430: viewport(430),
    w768: viewport(768),
    w1024: viewport(1024),
    w1440: viewport(1440),
}

interface LayoutArgs {
    mobileCandidateLists: EMobileCandidateLists
    screen: "vote" | "review"
}

const apolloClient = new ApolloClient({
    cache: new InMemoryCache(),
    link: new ApolloLink(() => {
        throw new Error("Unexpected GraphQL request from the slate layout story")
    }),
})

const meta = {
    title: "Voting/Slate ballot layout",
    args: {mobileCandidateLists: EMobileCandidateLists.COLLAPSED, screen: "vote"},
    parameters: {
        viewport: {options: VIEWPORTS},
        router: {
            parentPath: "/tenant/:tenantId/event/:eventId/election/:electionId",
            path: "vote",
            initialEntries: [`${electionPath}/vote?lang=en`],
            action,
            errorElement: <ErrorPage />,
        },
    },
    loaders: [
        async ({args}) => {
            await initCore()
            prepare(args)
        },
    ],
    render: ({screen}) => (
        <ApolloProvider client={apolloClient}>
            <Provider store={store}>
                <BallotSelectionAdapter>
                    <main>{screen === "review" ? <ReviewScreen /> : <VotingScreen />}</main>
                </BallotSelectionAdapter>
            </Provider>
        </ApolloProvider>
    ),
} satisfies Meta<LayoutArgs>
export default meta
type Story = StoryObj<typeof meta>

const all = (root: ParentNode, selector: string): Array<HTMLElement> =>
    Array.from(root.querySelectorAll<HTMLElement>(selector))

const isShown = (element: HTMLElement) => element.getClientRects().length > 0

const top = (element: HTMLElement) => Math.round(element.getBoundingClientRect().top)

const selectedCandidates = () =>
    (store.getState().ballotSelections[IDS.election] ?? []).flatMap((contest) =>
        contest.choices.filter((choice) => choice.selected > -1).map((choice) => choice.id)
    )

const label = (element: HTMLElement) =>
    `${element.tagName.toLowerCase()}.${String(element.className).split(" ").slice(-2).join(".")}`

/** The elements that stick out of the page, which is what makes it scroll sideways. */
async function expectNoHorizontalOverflow(canvasElement: HTMLElement) {
    const page = canvasElement.ownerDocument.documentElement
    const outside = all(canvasElement.ownerDocument, "body *")
        .filter(isShown)
        .filter((element) => {
            const box = element.getBoundingClientRect()
            return box.left < 0 || Math.floor(box.right) > page.clientWidth
        })
        .map((element) => `${label(element)} ${Math.round(element.getBoundingClientRect().right)}`)
    await expect(outside).toEqual([])
    await expect(page.scrollWidth).toBeLessThanOrEqual(page.clientWidth)
}

async function expectTouchTargetsOf(controls: Array<HTMLElement>) {
    await expect(controls.length).toBeGreaterThan(0)
    for (const control of controls) {
        await expect({
            [label(control)]:
                Math.round(control.getBoundingClientRect().height) >= MIN_TOUCH_TARGET,
        }).toEqual({[label(control)]: true})
    }
}

/** Where each card has the given part; one value when the cards line up. */
const tops = (cards: Array<HTMLElement>, selector: string): Array<number> =>
    cards.map((card) => {
        const part = card.querySelector<HTMLElement>(selector)
        if (!part) {
            throw new Error(`${selector} is missing from ${card.dataset.slateId}`)
        }
        return top(part)
    })

async function expectAlignedCards(cards: Array<HTMLElement>) {
    for (const office of OFFICES) {
        const officeTops = tops(cards, `[data-contest-id="${office.id}"]`)
        await expect({[office.id]: officeTops}).toEqual({
            [office.id]: officeTops.map(() => officeTops[0]),
        })
    }
    const actionTops = tops(cards, ".slate-actions")
    await expect({actions: actionTops}).toEqual({actions: actionTops.map(() => actionTops[0])})
}

async function expectTouchTargets(canvasElement: HTMLElement) {
    const controls = all(
        canvasElement,
        ".slate-ballot-tabs [role='tab'], .slate-candidate-list-toggle, .slate-apply-button, .slate-edit-selections-button"
    ).filter(isShown)
    await expectTouchTargetsOf(controls)
}

async function cardsOf(canvasElement: HTMLElement) {
    const canvas = within(canvasElement)
    await canvas.findByRole("tab", {name: "Choose a slate"})
    await waitFor(() => expect(all(canvasElement, ".slate-card")).toHaveLength(3))
    return all(canvasElement, ".slate-card")
}

async function expectIndividualCandidatesFit(canvasElement: HTMLElement) {
    const canvas = within(canvasElement)
    await userEvent.click(canvas.getByRole("tab", {name: "Individual candidates"}))
    await waitFor(() => expect(canvas.getAllByText("Rowan Scott").some(isShown)).toBe(true))
    await expectNoHorizontalOverflow(canvasElement)
    await userEvent.click(canvas.getByRole("tab", {name: "Choose a slate"}))
}

const desktop = (width: number): Story => ({
    globals: {viewport: {value: `w${width}`}},
    play: async ({canvasElement}) => {
        const cards = await cardsOf(canvasElement)
        await expect(window.innerWidth).toBe(width)

        // Every candidate of every slate is on screen without a click.
        await expect(all(canvasElement, ".slate-candidate-list-toggle")).toHaveLength(0)
        for (const member of all(canvasElement, ".slate-member")) {
            await expect(member).toBeVisible()
        }

        // The cards are columns, and each office starts at the same height in all of them.
        await expect(new Set(cards.map(top)).size).toBe(1)
        await expect(new Set(cards.map((card) => card.getBoundingClientRect().left)).size).toBe(3)
        await expectAlignedCards(cards)

        // The offices follow each other without overlapping, also where names wrap.
        for (const card of cards) {
            const rows = all(card, ".slate-contest")
            for (let index = 1; index < rows.length; index++) {
                await expect(rows[index].getBoundingClientRect().top).toBeGreaterThanOrEqual(
                    rows[index - 1].getBoundingClientRect().bottom
                )
            }
        }

        const voices = cards[2]
        await expect(all(voices, ".slate-contest-empty")).toHaveLength(OFFICES.length - 1)
        await expect(
            within(voices.querySelector<HTMLElement>('[data-contest-id="president"]')!).getByText(
                "No candidate"
            )
        ).toBeVisible()

        await expectTouchTargets(canvasElement)
        await expectNoHorizontalOverflow(canvasElement)
        await expectIndividualCandidatesFit(canvasElement)
    },
})

const phone = (width: number, mobileCandidateLists: EMobileCandidateLists): Story => ({
    args: {mobileCandidateLists},
    globals: {viewport: {value: `w${width}`}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const cards = await cardsOf(canvasElement)
        await expect(window.innerWidth).toBe(width)
        const startsExpanded = mobileCandidateLists === EMobileCandidateLists.EXPANDED

        // The cards are stacked, one per row.
        await expect(new Set(cards.map((card) => card.getBoundingClientRect().left)).size).toBe(1)
        await expect(new Set(cards.map(top)).size).toBe(3)

        const toggles = all(canvasElement, ".slate-candidate-list-toggle")
        await expect(toggles).toHaveLength(3)
        for (const toggle of toggles) {
            await expect(toggle).toHaveAttribute("aria-expanded", String(startsExpanded))
        }
        await expect(canvas.queryByText("No candidate")).toBeNull()
        await expect(all(cards[2], ".slate-contest")).toHaveLength(1)

        // The keyboard reaches the toggle and operates it, and no vote changes.
        const before = selectedCandidates()
        const member = within(cards[0]).getByText("Jordan Ellis")
        toggles[0].focus()
        await expect(toggles[0]).toHaveFocus()
        await userEvent.keyboard("{Enter}")
        await expect(toggles[0]).toHaveAttribute("aria-expanded", String(!startsExpanded))
        await expect(isShown(member)).toBe(!startsExpanded)
        await expect(toggles[1]).toHaveAttribute("aria-expanded", String(startsExpanded))
        await userEvent.keyboard(" ")
        await expect(toggles[0]).toHaveAttribute("aria-expanded", String(startsExpanded))
        await userEvent.keyboard("{Enter}")
        await expect(selectedCandidates()).toEqual(before)

        await expectTouchTargets(canvasElement)
        await expectNoHorizontalOverflow(canvasElement)
        await expectIndividualCandidatesFit(canvasElement)

        // The voter's toggle is kept after visiting the individual candidates.
        await expect(toggles[0]).toHaveAttribute("aria-expanded", String(!startsExpanded))

        // With every list open the long names wrap inside their card.
        for (const toggle of all(canvasElement, ".slate-candidate-list-toggle")) {
            if (toggle.getAttribute("aria-expanded") === "false") {
                await userEvent.click(toggle)
            }
        }
        for (const name of all(canvasElement, ".slate-member, .slate-name")) {
            const box = name.getBoundingClientRect()
            const card = name.closest<HTMLElement>(".slate-card")!.getBoundingClientRect()
            await expect(Math.floor(box.right)).toBeLessThanOrEqual(Math.ceil(card.right))
        }
        await expectNoHorizontalOverflow(canvasElement)
    },
})

const review = (width: number): Story => ({
    args: {screen: "review"},
    globals: {viewport: {value: `w${width}`}},
    parameters: {
        router: {
            path: "/tenant/:tenantId/event/:eventId/election/:electionId/review",
            initialEntries: [`${electionPath}/review?lang=en`],
            errorElement: <ErrorPage />,
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("button", {name: "Cast ballot"})
        await expect(window.innerWidth).toBe(width)

        // Every office is listed with its selected candidates and their slate.
        for (const office of OFFICES) {
            await expect(canvas.getAllByText(office.name).some(isShown)).toBe(true)
        }
        const longName = canvas.getByText("Casey Rivera-Montgomery de la Fuente y Aguirre")
        await expect(longName).toBeVisible()
        await expect(
            canvas
                .getAllByText("Members First: a voice for every member of the association")
                .filter(isShown)
        ).toHaveLength(7)

        await expectTouchTargetsOf(
            all(
                canvasElement,
                "main button:not(.MuiIconButton-root), main a.MuiButton-root"
            ).filter(isShown)
        )
        await expectNoHorizontalOverflow(canvasElement)
    },
})

export const Desktop768: Story = desktop(768)
export const Desktop1024: Story = desktop(1024)
export const Desktop1440: Story = desktop(1440)
export const Phone320Collapsed: Story = phone(320, EMobileCandidateLists.COLLAPSED)
export const Phone390Collapsed: Story = phone(390, EMobileCandidateLists.COLLAPSED)
export const Phone430Collapsed: Story = phone(430, EMobileCandidateLists.COLLAPSED)
export const Phone320Expanded: Story = phone(320, EMobileCandidateLists.EXPANDED)
export const Phone430Expanded: Story = phone(430, EMobileCandidateLists.EXPANDED)

export const Review320: Story = review(320)
export const Review430: Story = review(430)
export const Review768: Story = review(768)
export const Review1440: Story = review(1440)

export const ChosenSlateStaysAligned: Story = {
    globals: {viewport: {value: "w1024"}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const cards = await cardsOf(canvasElement)

        await userEvent.click(canvas.getByRole("button", {name: "Choose slate Forward Together"}))
        await waitFor(() => expect(canvas.getByText("All 7 selected")).toBeVisible())

        await expectAlignedCards(cards)
        await expectTouchTargets(canvasElement)
        await expectNoHorizontalOverflow(canvasElement)
    },
}
