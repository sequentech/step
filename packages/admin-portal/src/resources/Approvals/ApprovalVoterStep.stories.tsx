// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within} from "storybook/test"
import type {Mock} from "storybook/test"
import {STORY_IDS} from "@/__stories__/fixtures"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {ApprovalVoterStep, NO_VOTER} from "./ApprovalVoterStep"
import {humanizeField} from "./approvalMatrix"
import {rankCandidates} from "./approvalReview"
import {REGISTRY_VOTERS, applicationRecord} from "./__stories__/ApprovalsFixture"
import {ApprovalsScreen, setUpApprovals} from "./__stories__/ApprovalsScreenFixture"

interface Scenario {
    /** The registry voters on offer: those named Example, or nobody. */
    found: "voters" | "nobody"
    /** The enrollment's identity document prints first and middle name together. */
    jointNames: boolean
    chosen: string | null
    search: string
    loading: boolean
    failed: boolean
    readOnly: boolean
    onChoose: Mock<(chosen: string) => void>
    onSearch: Mock<(search: string) => void>
}

const FIELDS = ["firstName", "lastName", "dateOfBirth", "embassy"]
const JOINT_FIELDS = ["firstName", "middleName", ...FIELDS.slice(1)]

/** Alice's enrollment compared with the registry voters whose last name is hers. */
const candidatesOf = ({found, jointNames}: Pick<Scenario, "found" | "jointNames">) => {
    const alice = applicationRecord()
    const application = jointNames
        ? {
              ...alice,
              applicant_data: {
                  ...alice.applicant_data,
                  "middleName": "Marie",
                  "sequent.read-only.id-card-type": "driversLicense",
              },
          }
        : alice
    return rankCandidates(
        application,
        found === "voters" ? REGISTRY_VOTERS.filter((voter) => voter.last_name === "Example") : [],
        jointNames ? JOINT_FIELDS : FIELDS,
        ["email"]
    )
}

const meta = {
    title: "Admin/Approvals/ApprovalVoterStep",
    component: ApprovalVoterStep,
    args: {
        found: "voters",
        jointNames: false,
        chosen: null,
        search: "",
        loading: false,
        failed: false,
        readOnly: false,
        onChoose: fn(),
        onSearch: fn(),
    },
    argTypes: {found: {control: "inline-radio", options: ["voters", "nobody"]}},
    beforeEach: () => setUpApprovals({reads: "records", empty: false}),
    render: ({found, jointNames, ...props}) => (
        <ApprovalsScreen>
            <ApprovalVoterStep
                candidates={candidatesOf({found, jointNames})}
                comparedFields={jointNames ? JOINT_FIELDS : FIELDS}
                fieldLabel={humanizeField}
                jointNames={jointNames}
                {...props}
            />
        </ApprovalsScreen>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const SEARCH = "Not in the list? Search the registry by name or email"

const voters = (canvasElement: HTMLElement) =>
    within(within(canvasElement).getByRole("radiogroup", {name: "Voters in the registry"}))

/** What each choice says, in order. */
const choices = (canvasElement: HTMLElement) =>
    voters(canvasElement)
        .getAllByRole("radio")
        .map((radio) => radio.closest("label")?.textContent)

export const ClosestVoters: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await expect(
            canvas.getByText(
                "We looked for voters with the same first name, last name, date of birth and " +
                    "embassy. Choose the one this enrollment belongs to."
            )
        ).toBeVisible()
        // The closest voter first, and the voters who are already enrolled last.
        expect(choices(canvasElement)).toEqual([
            "AEAlice ExampleMay 7, 1990 · MadridBest match · 3 of 4 details match",
            "AEAlicia ExampleJun 18, 1991 · Lisbon1 of 4 details match",
            "RERobert ExampleFeb 3, 1985 · LisbonAlready enrolled",
            "None of these is the voterThe enrollment can then only be rejected, for no matching voter.",
        ])
        await expect(
            voters(canvasElement).getByRole("radio", {name: /^Robert Example/})
        ).toBeDisabled()
        for (const radio of voters(canvasElement).getAllByRole("radio")) {
            await expect(radio).not.toBeChecked()
        }

        // Until a voter is chosen, the enrollment is compared with the closest one.
        await expect(
            canvas.getByRole("heading", {name: "Compared with Alice Example in the registry"})
        ).toBeVisible()
        expect(
            within(canvas.getByRole("table"))
                .getAllByRole("row")
                .map((row) => row.textContent)
        ).toEqual([
            "DetailOn the enrollmentIn the registryResult",
            "First NameAliceAliceSame",
            "Last NameExampleExampleSame",
            "Date Of BirthMay 17, 1990May 7, 1990Different",
            "EmbassyMadridMadridSame",
        ])
        await expect(
            canvas.getByText("Names ignore capital letters, accents and hyphens.")
        ).toBeVisible()

        await userEvent.click(voters(canvasElement).getByRole("radio", {name: /^Alicia Example/}))
        expect(args.onChoose).toHaveBeenLastCalledWith(STORY_IDS.secondUser)
        await userEvent.click(
            voters(canvasElement).getByRole("radio", {name: /^None of these is the voter/})
        )
        expect(args.onChoose).toHaveBeenLastCalledWith(NO_VOTER)
        await userEvent.type(canvas.getByRole("textbox", {name: SEARCH}), "x")
        expect(args.onSearch).toHaveBeenLastCalledWith("x")
    },
}

export const ChosenVoter: Story = {
    args: {chosen: STORY_IDS.secondUser},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            voters(canvasElement).getByRole("radio", {name: /^Alicia Example/})
        ).toBeChecked()
        await expect(
            canvas.getByRole("heading", {name: "Compared with Alicia Example in the registry"})
        ).toBeVisible()
        await expect(
            canvas.getByRole("row", {name: "Embassy Madrid Lisbon Different"})
        ).toBeVisible()
    },
}

export const NoneIsTheVoter: Story = {
    args: {chosen: NO_VOTER},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            voters(canvasElement).getByRole("radio", {name: /^None of these is the voter/})
        ).toBeChecked()
        expect(canvas.queryByRole("table")).toBeNull()
    },
}

export const SearchResults: Story = {
    args: {search: "example"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            canvas.getByText(
                "These are the voters in the registry that match your search. Choose the one " +
                    "this enrollment belongs to."
            )
        ).toBeVisible()
        await expect(canvas.getByRole("textbox", {name: SEARCH})).toHaveValue("example")
        // The results of a search have no best match.
        expect(canvas.queryByText(/Best match/)).toBeNull()
        expect(choices(canvasElement)[0]).toBe(
            "AEAlice ExampleMay 7, 1990 · Madrid3 of 4 details match"
        )
    },
}

export const NamesComparedTogether: Story = {
    args: {jointNames: true},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            canvas.getByRole("row", {
                name: "First Name and Middle Name Alice Marie Alice Different",
            })
        ).toBeVisible()
        expect(canvas.queryByRole("row", {name: /^Middle Name/})).toBeNull()
        await expect(
            canvas.getByText(
                "Names ignore capital letters, accents and hyphens. For driver's licenses and " +
                    "seafarer's books, first and middle name are compared together."
            )
        ).toBeVisible()
        expect(choices(canvasElement)[0]).toMatch(/Best match · 2 of 4 details match$/)
    },
}

export const LookingInTheRegistry: Story = {
    args: {found: "nobody", loading: true},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            canvas.getByRole("progressbar", {name: "Looking in the registry"})
        ).toBeVisible()
        expect(
            canvas.queryByText("No voter in the registry matches. Try searching by name or email.")
        ).toBeNull()
    },
}

export const NobodyFound: Story = {
    args: {found: "nobody"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            canvas.getByText("No voter in the registry matches. Try searching by name or email.")
        ).toBeVisible()
        expect(choices(canvasElement)).toHaveLength(1)
        expect(canvas.queryByRole("table")).toBeNull()
    },
}

export const RegistryFails: Story = {
    args: {found: "nobody", failed: true},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("alert")).toHaveTextContent(
            "The registry could not be searched."
        )
        expect(
            canvas.queryByText("No voter in the registry matches. Try searching by name or email.")
        ).toBeNull()
    },
}

export const ReadOnly: Story = {
    args: {readOnly: true, chosen: STORY_IDS.user},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        // A decided enrollment shows the voters without the choice or the search.
        expect(choices(canvasElement)).toHaveLength(3)
        for (const radio of voters(canvasElement).getAllByRole("radio")) {
            await expect(radio).toBeDisabled()
        }
        expect(canvas.queryByRole("textbox", {name: SEARCH})).toBeNull()
        await expect(
            canvas.getByRole("heading", {name: "Compared with Alice Example in the registry"})
        ).toBeVisible()
    },
}
