// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {Provider} from "react-redux"
import {configureStore} from "@reduxjs/toolkit"
import {i18n, initCore} from "@sequentech/ui-core"
import * as sequentCore from "sequent-core"
import {electionFixture, FIXED_TIME, IDS} from "@sequentech/ui-test-kit/fixtures"
import {routeFetch} from "@sequentech/ui-test-kit/adapters/fetch"
import {json} from "@sequentech/ui-test-kit/mocks/http"
import {S3Mock} from "@sequentech/ui-test-kit/mocks/s3"
import {ViolationLog} from "@sequentech/ui-test-kit/mocks/violations"
import App from "../../App"
import {AuthContext, type AuthContextValues} from "../../providers/AuthContextProvider"
import {SettingsContext, type GlobalSettings} from "../../providers/SettingsContextProvider"
import ballotStyles from "../../store/ballotStyles/ballotStylesSlice"
import {encryptAndSign} from "../../../tests/fixtures/signedBallot"
import {
    EStoryLocale,
    EStoryTenant,
    STORY_BRANDING,
    readStoryGlobals,
} from "../../../../ui-essentials/.storybook/globals"
import {EStoryDataState} from "../../../../ui-essentials/.storybook/screens"

const S3_ORIGIN = "https://s3.story.test"
const PUBLICATION_ID = "90000000-0000-4000-8000-000000000001"
const eventPath = `/tenant/${IDS.tenant}/event/${IDS.event}`
const settings: GlobalSettings = {
    DISABLE_AUTH: false,
    QUERY_POLL_INTERVAL_MS: 3_600_000,
    DEFAULT_TENANT_ID: IDS.tenant,
    DEFAULT_EVENT_ID: IDS.event,
    ONLINE_VOTING_CLIENT_ID: "ballot-verifier",
    KEYCLOAK_URL: "https://keycloak.story.test/",
    HASURA_URL: "https://hasura.story.test/v1/graphql",
    APP_VERSION: "story",
    APP_HASH: "story",
    PUBLIC_BUCKET_URL: new S3Mock({
        origin: S3_ORIGIN,
        violations: new ViolationLog(),
    }).publicBucketUrl(),
}

/** The event's published ballot style, whose presentation carries the tenant's branding. */
function publishedBallotStyle(tenant: EStoryTenant) {
    const {ballot, style} = electionFixture()
    const {logo_url: _sequentLogo, ...presentation} = ballot.election_event_presentation
    return {
        ...style,
        __typename: "sequent_backend_ballot_style",
        ballot_publication_id: PUBLICATION_ID,
        status: null,
        deleted_at: null,
        ballot_eml: JSON.stringify({
            ...ballot,
            election_event_presentation: {
                ...presentation,
                ...STORY_BRANDING[tenant],
                language_conf: {
                    ...presentation.language_conf,
                    enabled_language_codes: Object.values(EStoryLocale),
                },
            },
        }),
    }
}

interface Scenario {
    data: EStoryDataState
}
let auth: AuthContextValues

function VerifierScreen() {
    const [store] = useState(() => configureStore({reducer: {ballotStyles}}))
    return (
        <SettingsContext.Provider value={{loaded: true, globalSettings: settings}}>
            <AuthContext.Provider value={auth}>
                <Provider store={store}>
                    <App />
                </Provider>
            </AuthContext.Provider>
        </SettingsContext.Provider>
    )
}

const meta = {
    title: "Screens/Verifier/Ballot verification",
    component: VerifierScreen,
    args: {data: EStoryDataState.POPULATED},
    parameters: {router: {initialEntries: [`${eventPath}/start`]}},
    loaders: [
        async () => {
            await initCore()
            // A real voter's audit download, encrypted and signed like the journeys' ballots.
            return {voterBallot: encryptAndSign(sequentCore)}
        },
    ],
    beforeEach: ({args, globals}) => {
        const signedIn = args.data !== EStoryDataState.LOADING
        auth = {
            isAuthenticated: signedIn,
            userId: IDS.voter,
            username: "voter",
            email: "voter@example.test",
            firstName: "Pat",
            logout: fn(),
            login: fn(),
            hasRole: () => false,
            getAccessToken: () => (signedIn ? "synthetic-voter-token" : undefined),
            openProfileLink: async () => undefined,
        }
        const violations = new ViolationLog()
        const s3 = new S3Mock({origin: S3_ORIGIN, violations})
        const style = publishedBallotStyle(readStoryGlobals(globals).tenant)
        s3.putJson("public", `tenant-${IDS.tenant}/event-${IDS.event}/election_event_config.json`, {
            id: IDS.event,
            tenant_id: IDS.tenant,
            election_event_id: IDS.event,
            election_event_presentation: JSON.parse(style.ballot_eml).election_event_presentation,
        })
        const hasura = {
            handles: (url: URL) => url.href === settings.HASURA_URL,
            handle: ({body}: {body?: string}) => {
                const {operationName} = JSON.parse(body ?? "{}") as {operationName?: string}
                if (operationName !== "GetBallotStyles") {
                    violations.add(`Unexpected GraphQL operation: ${operationName}`)
                    return json(200, {errors: [{message: "Unexpected operation"}]})
                }
                return json(200, {
                    data: {
                        sequent_backend_ballot_publication: [
                            {
                                __typename: "sequent_backend_ballot_publication",
                                id: PUBLICATION_ID,
                                published_at: FIXED_TIME,
                            },
                        ],
                        sequent_backend_ballot_style: [style],
                    },
                })
            },
        }
        const restore = routeFetch([s3, hasura], violations)
        return () => {
            restore()
            expect(violations.list()).toEqual([])
        }
    },
    render: (_args, {globals}) => <VerifierScreen key={JSON.stringify(globals)} />,
} satisfies Meta<Scenario>
export default meta
type Story = StoryObj<typeof meta>

/** Translated in the story's locale, so the interactions also run after a toolbar change. */
const text = (key: string) => i18n.t(key)

export const Loading: Story = {
    args: {data: EStoryDataState.LOADING},
    parameters: {
        expectedFailure: {
            reason: "The spinner shown while signing in has no accessible name.",
            a11y: ["aria-progressbar-name"],
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await waitFor(() =>
            expect(auth.login).toHaveBeenCalledWith(IDS.tenant, IDS.event, undefined)
        )
        await expect(canvas.getByRole("progressbar")).toBeVisible()
        await expect(canvas.queryByText(text("homeScreen.step1"))).not.toBeInTheDocument()
    },
}

export const Empty: Story = {
    args: {data: EStoryDataState.EMPTY},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText(text("homeScreen.step1"))).toBeVisible()
        await expect(
            canvas.getByRole("button", {name: text("homeScreen.nextButton")})
        ).toBeDisabled()
    },
}

export const Populated: Story = {
    play: async ({canvasElement, loaded}) => {
        const {ballot, hash} = (loaded as {voterBallot: ReturnType<typeof encryptAndSign>})
            .voterBallot
        const canvas = within(canvasElement)
        await canvas.findByText(text("homeScreen.step1"))
        const file = new File([JSON.stringify(ballot)], "ballot.json", {type: "application/json"})
        await userEvent.upload(canvas.getByTestId<HTMLInputElement>("drop-input-file"), file)
        await userEvent.type(
            canvas.getByRole("textbox", {name: text("homeScreen.ballotIdLabel")}),
            hash
        )
        const next = canvas.getByRole("button", {name: text("homeScreen.nextButton")})
        await waitFor(() => expect(next).toBeEnabled())
        await userEvent.click(next)
        await expect(
            await canvas.findByRole("heading", {
                name: text("confirmationScreen.verifySelectionsTitle"),
            })
        ).toBeVisible()
        await expect(canvas.getByText("Alice Example", {exact: true})).toBeVisible()
        await expect(canvas.getByLabelText("Current location")).toHaveTextContent(
            `${eventPath}/confirmation`
        )
    },
}

export const LoadError: Story = {
    args: {data: EStoryDataState.ERROR},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByText(text("homeScreen.step1"))
        const file = new File(["{invalid JSON"], "ballot.json", {type: "application/json"})
        await userEvent.upload(canvas.getByTestId<HTMLInputElement>("drop-input-file"), file)
        const alert = await canvas.findByRole("alert")
        await expect(alert).toHaveTextContent(text("homeScreen.importErrorTitle"))
        await expect(alert).toHaveTextContent(
            text("homeScreen.importErrorDescription").replace(/\s+/g, " ")
        )
        await expect(
            canvas.getByRole("button", {name: text("homeScreen.nextButton")})
        ).toBeDisabled()
    },
}

export const CustomBranding: Story = {
    args: {data: EStoryDataState.EMPTY},
    globals: {tenant: EStoryTenant.CUSTOM},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const heading = await canvas.findByText(text("homeScreen.step1"))
        await waitFor(() =>
            expect(canvasElement.querySelector(".header-logo")?.getAttribute("src")).toBe(
                STORY_BRANDING[EStoryTenant.CUSTOM].logo_url
            )
        )
        await waitFor(() => expect(getComputedStyle(heading).color).toBe("rgb(11, 79, 58)"))
    },
}
