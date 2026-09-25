// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {render, screen, waitFor, within} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import {configureStore} from "@reduxjs/toolkit"
import {Provider} from "react-redux"
import {MemoryRouter, useLocation} from "react-router-dom"
import {ThemeProvider} from "@mui/material"
import {theme} from "@sequentech/ui-essentials"
import {ELanguageDetectionPolicy} from "@sequentech/ui-core"
import {electionFixture, FIXED_TIME} from "@sequentech/ui-test-kit/fixtures"
import App from "./App"
import {KeycloakProviderContainer} from "./index"
import {WasmWrapper} from "./providers/WasmWrapper"
import {GlobalSettings, SettingsWrapper} from "./providers/SettingsContextProvider"
import ballotStyles from "./store/ballotStyles/ballotStylesSlice"
import FakeKeycloak, {resetKeycloak} from "./__mocks__/keycloak"
import {resetSequentCore} from "./__mocks__/sequentCore"
import {IDS, recordSequentCore, singleContestBallot} from "./__mocks__/auditableBallots"

jest.mock("keycloak-js", () => jest.requireActual("./__mocks__/keycloak"))
jest.mock("react-dom/client", () => {
    const client = jest.requireActual<typeof import("react-dom/client")>("react-dom/client")
    return {
        ...client,
        // index.tsx mounts the whole app into #root as soon as it loads, and
        // App imports it. The test DOM has no #root, so that mount does nothing
        // and each test mounts the same providers itself.
        createRoot: (...args: Parameters<typeof client.createRoot>) =>
            args[0]
                ? client.createRoot(...args)
                : {render: () => undefined, unmount: () => undefined},
    }
})

// The shipped global-settings.json default event, which is not the voter's.
const defaultEvent = {
    tenant: "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
    event: "33f18502-a67c-4853-8333-a58630663559",
}
const settings: GlobalSettings = {
    DISABLE_AUTH: false,
    QUERY_POLL_INTERVAL_MS: 60000,
    DEFAULT_TENANT_ID: defaultEvent.tenant,
    DEFAULT_EVENT_ID: defaultEvent.event,
    ONLINE_VOTING_CLIENT_ID: "ballot-verifier",
    KEYCLOAK_URL: "https://keycloak.test/",
    HASURA_URL: "https://hasura.test/v1/graphql",
    APP_VERSION: "test",
    APP_HASH: "test",
    PUBLIC_BUCKET_URL: "https://s3.test/public/",
}
const eventPath = (tenant: string, event: string) => `/tenant/${tenant}/event/${event}`
const voterEvent = eventPath(IDS.tenant, IDS.event)

/** Answers the verifier's HTTP requests as the journeys' S3 and Hasura mocks do. */
function serve(
    overrides: Partial<GlobalSettings> = {},
    {eventConfig = true, eventPresentation = {}} = {}
) {
    const served = {...settings, ...overrides}
    const unexpected: string[] = []
    const graphql: Array<{operationName: string; authorization?: string}> = []
    const reply = (body: unknown, status = 200) =>
        ({
            ok: status < 300,
            status,
            headers: new Map([["content-type", "application/json"]]),
            json: async () => body,
            text: async () => JSON.stringify(body),
        }) as unknown as Response
    const configPath =
        /^https:\/\/s3\.test\/public\/tenant-([^/]+)\/event-([^/]+)\/election_event_config\.json$/
    jest.spyOn(globalThis, "fetch").mockImplementation(async (input, init) => {
        const url = String(input)
        const config = configPath.exec(url)
        if (url === "/global-settings.json") return reply(served)
        if (config && eventConfig) {
            return reply({
                id: config[2],
                tenant_id: config[1],
                election_event_id: config[2],
                election_event_presentation: {
                    ...electionFixture().event.presentation,
                    ...eventPresentation,
                },
            })
        }
        if (config) return reply({message: "Not Found"}, 404)
        if (url === served.HASURA_URL && init?.method === "POST") {
            const {operationName} = JSON.parse(String(init.body))
            const headers = init.headers as Record<string, string>
            graphql.push({operationName, authorization: headers.authorization})
            if (operationName === "GetBallotStyles") return reply({data: publishedBallotStyles()})
        }
        unexpected.push(`${init?.method ?? "GET"} ${url}`)
        throw new Error(`Unexpected request: ${url}`)
    })
    return {unexpected, graphql}
}

const publishedBallotStyles = () => ({
    sequent_backend_ballot_publication: [
        {
            __typename: "sequent_backend_ballot_publication",
            id: "publication",
            published_at: FIXED_TIME,
        },
    ],
    sequent_backend_ballot_style: [
        {
            __typename: "sequent_backend_ballot_style",
            ...electionFixture().style,
            ballot_publication_id: "publication",
            status: null,
            deleted_at: null,
        },
    ],
})

const CurrentLocation = () => <p aria-label="Current location">{useLocation().pathname}</p>

/** The provider tree of index.tsx, with a memory router starting at the given address. */
function launch(path: string) {
    render(
        <WasmWrapper>
            <SettingsWrapper>
                <KeycloakProviderContainer>
                    <Provider store={configureStore({reducer: {ballotStyles}})}>
                        <MemoryRouter initialEntries={[path]}>
                            <ThemeProvider theme={theme}>
                                <App />
                                <CurrentLocation />
                            </ThemeProvider>
                        </MemoryRouter>
                    </Provider>
                </KeycloakProviderContainer>
            </SettingsWrapper>
        </WasmWrapper>
    )
}

const location = () => screen.getByLabelText("Current location").textContent
const expectLocation = (path: string) => waitFor(() => expect(location()).toBe(path))
const importStep = () => screen.findByRole("heading", {name: /Step 1: Import your ballot/})

async function importAndContinue() {
    const single = singleContestBallot()
    recordSequentCore(single)
    await importStep()
    userEvent.upload(
        screen.getByTestId("drop-input-file"),
        new File([JSON.stringify(single.ballot)], "audited-ballot.json", {type: "application/json"})
    )
    expect(await screen.findByText("Uploaded", {exact: true})).toBeVisible()
    userEvent.type(screen.getByRole("textbox", {name: "Ballot ID"}), single.ballotId)
    userEvent.click(screen.getByRole("button", {name: "Next"}))
    return single
}

let services: ReturnType<typeof serve>
beforeEach(() => {
    resetKeycloak()
    resetSequentCore()
    jest.spyOn(console, "log").mockImplementation(() => undefined)
    jest.spyOn(console, "info").mockImplementation(() => undefined)
})
afterEach(() => {
    expect(services.unexpected).toEqual([])
    jest.restoreAllMocks()
    document.cookie = "USER_LANGUAGE=; expires=Thu, 01 Jan 1970 00:00:00 GMT; path=/"
    window.history.replaceState(null, "", "/?lang=en")
})

describe("event routes", () => {
    it("signs in to the event from its login route and continues to its import step", async () => {
        services = serve()
        launch(`${voterEvent}/login`)

        await expectLocation(`${voterEvent}/start`)
        expect(await importStep()).toBeVisible()
        expect(FakeKeycloak.instances.map(({config}) => config)).toEqual([
            {
                realm: `tenant-${IDS.tenant}-event-${IDS.event}`,
                url: settings.KEYCLOAK_URL,
                clientId: "ballot-verifier",
            },
        ])
        await waitFor(() =>
            expect(services.graphql).toContainEqual({
                operationName: "GetBallotStyles",
                authorization: "Bearer voter-access-token",
            })
        )
    })

    it("opens the default event from the root address", async () => {
        services = serve()
        launch("/")

        await expectLocation(`${eventPath(defaultEvent.tenant, defaultEvent.event)}/start`)
        expect(await importStep()).toBeVisible()
        expect(FakeKeycloak.instances[0].config).toMatchObject({
            realm: `tenant-${defaultEvent.tenant}-event-${defaultEvent.event}`,
        })
    })

    it("verifies an imported ballot on the event's confirmation route", async () => {
        services = serve()
        launch(`${voterEvent}/start`)

        const {ballotId} = await importAndContinue()

        await expectLocation(`${voterEvent}/confirmation`)
        expect(await screen.findByText("Alice Example", {exact: true})).toBeVisible()
        expect(screen.getAllByText(ballotId)).toHaveLength(2)
        expect(screen.queryByText("Bob Example", {exact: true})).not.toBeInTheDocument()
    })

    // With no lang parameter or saved choice, a forced default language applies
    // (docs: election managers' Languages reference).
    it("signs in to Keycloak in the event's forced default language", async () => {
        window.history.replaceState(null, "", "/")
        services = serve(
            {},
            {
                eventPresentation: {
                    language_conf: {
                        language_detection_policy: ELanguageDetectionPolicy.FORCE_DEFAULT,
                        default_language_code: "es",
                        enabled_language_codes: ["en", "es"],
                    },
                },
            }
        )
        launch(`${voterEvent}/start`)

        await waitFor(() =>
            expect(FakeKeycloak.instances[0]?.init).toHaveBeenCalledWith(
                expect.objectContaining({locale: "es"})
            )
        )
    })

    it("still signs in when the event's configuration cannot be downloaded", async () => {
        services = serve({}, {eventConfig: false})
        jest.spyOn(console, "error").mockImplementation(() => undefined)
        launch(`${voterEvent}/start`)

        expect(await importStep()).toBeVisible()
        expect(FakeKeycloak.instances).toHaveLength(1)
    })

    it("shows the not-found page for other addresses", async () => {
        services = serve()
        launch("/elsewhere")

        expect(await screen.findByRole("heading", {name: "Page not found"})).toBeVisible()
    })

    it.each([
        [
            "goes back",
            async () => {
                await importAndContinue()
                userEvent.click(await screen.findByRole("link", {name: "Back"}))
            },
        ],
        ["reloads the confirmation route", async () => undefined],
    ])("returns to the voter's own event when the voter %s", async (action, leave) => {
        services = serve()
        launch(action === "goes back" ? `${voterEvent}/start` : `${voterEvent}/confirmation`)

        await leave()

        await expectLocation(`${voterEvent}/start`)
    })
})

describe("the header", () => {
    it("offers the published event's languages and remembers the voter's choice", async () => {
        services = serve()
        launch(`${voterEvent}/start`)
        await importStep()

        userEvent.click(screen.getByTestId("lang-button-test"))
        const menu = await screen.findByRole("menu")
        expect(
            within(menu)
                .getAllByRole("menuitem")
                .map((item) => item.textContent)
        ).toEqual(["English", "Español"])
        userEvent.click(within(menu).getByRole("menuitem", {name: "Español"}))

        expect(
            await screen.findByRole("heading", {name: /Paso 1: Importa tu papeleta electoral/})
        ).toBeVisible()
        expect(document.cookie).toContain("USER_LANGUAGE=es")
    })
})

describe("with authentication disabled", () => {
    it("sends the voter to the default event without contacting Keycloak", async () => {
        services = serve({DISABLE_AUTH: true})
        launch("/")

        await expectLocation(`${eventPath(defaultEvent.tenant, defaultEvent.event)}/start`)
        expect(FakeKeycloak.instances).toHaveLength(0)
    })

    // Expected failure: ApolloContextProvider.tsx:458-468 creates a client only
    // for a signed-in voter, so ApolloWrapper shows a spinner instead.
    it.failing("opens the import step", async () => {
        services = serve({DISABLE_AUTH: true})
        launch("/")

        expect(await importStep()).toBeVisible()
    })
})
