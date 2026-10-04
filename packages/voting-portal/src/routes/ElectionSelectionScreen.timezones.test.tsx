// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {render, screen} from "@testing-library/react"
import {createMemoryRouter, RouterProvider} from "react-router-dom"
import {ThemeProvider} from "@mui/material/styles"
import {
    EConsolidatedReportPolicy,
    EElectionEventDelegatedVotingPolicy,
    EVotingPortalDateTimeFormat,
    IElectionEventPresentation,
} from "@sequentech/ui-core"
import theme from "../../../ui-essentials/src/services/theme"
import {store, type RootState} from "../store/store"
import {
    OVERSEAS,
    TEST_VOTING_DUBAI,
    zonedElectionDates,
} from "../../../ui-essentials/src/components/SelectElection/__stories__/zonedFixtures"
import ElectionSelectionScreen from "./ElectionSelectionScreen"

// The production wiring: the real ElectionWrapper and SelectElection, with the
// store, the queries and the translations stubbed. `t` echoes the key, so the
// timezone texts are ui-core's bundled defaults.
jest.mock("@sequentech/ui-core", () => ({
    ...jest.requireActual("@sequentech/ui-core"),
    ...jest.requireActual("../../../ui-core/src/services/cssClassNameFormatter"),
    sortElectionList: (elections: unknown[]) => elections,
}))
jest.mock("react-i18next", () => ({
    ...jest.requireActual("react-i18next"),
    Trans: () => null,
    useTranslation: () => ({
        t: (key: string, values?: Record<string, string>) =>
            values && "close" in values ? `${key}|${values.close}|${values.localClose ?? ""}` : key,
        i18n: {language: "en"},
    }),
}))
jest.mock("@sequentech/ui-essentials", () => ({
    ...jest.requireActual("@sequentech/ui-essentials"),
    IconButton: () => null,
    Dialog: () => null,
}))
jest.mock("../store/hooks", () => ({
    useAppSelector: (selector: (state: RootState) => unknown) => selector(mockState),
    useAppDispatch: () => jest.fn(),
}))
jest.mock("../components/Stepper", () => ({__esModule: true, default: () => null}))
jest.mock("../providers/SettingsContextProvider", () => ({
    SettingsContext: jest
        .requireActual<typeof React>("react")
        .createContext({globalSettings: {DISABLE_AUTH: false}}),
}))
jest.mock("@apollo/client/react", () => ({
    useQuery: () => ({data: undefined}),
    useMutation: () => [jest.fn()],
}))
jest.mock("../providers/AuthContextProvider", () => ({
    AuthContext: jest.requireActual<typeof React>("react").createContext({isKiosk: () => false}),
}))
jest.mock("../hooks/useVoterContext", () => ({useVoterContext: () => ({loading: false})}))

let mockState: RootState

const [DUBAI] = OVERSEAS.posts
const FORMAT = EVotingPortalDateTimeFormat.ISO_LOCAL

const eventPresentation = (zoned: boolean) =>
    ({
        delegated_voting_policy: EElectionEventDelegatedVotingPolicy.DISABLED,
        voting_portal_datetime_format: FORMAT,
        ...(zoned ? {timezones: {configured: OVERSEAS.configured, primary: OVERSEAS.primary}} : {}),
    }) as IElectionEventPresentation

const ballot = (id: string, name: string, dates: object, status: string) => ({
    election: {
        id,
        tenant_id: "tenant-1",
        election_event_id: "event-1",
        name,
        image_document_id: "",
        contests: [],
        presentation: {
            i18n: {en: {name}},
            consolidated_report_policy: EConsolidatedReportPolicy.DO_NOT_GENERATE,
            timezone: DUBAI.timeZone,
        },
        status: {voting_status: status} as unknown as string,
        num_allowed_revotes: 0,
    },
    style: {
        id: `style-${id}`,
        tenant_id: "tenant-1",
        election_event_id: "event-1",
        election_id: id,
        created_at: "",
        last_updated_at: "",
        ballot_eml: {election_dates: dates},
    },
})

const setUp = (zoned: boolean, eventStatus: string, ballots: Array<ReturnType<typeof ballot>>) => {
    mockState = {
        ...store.getState(),
        electionEvent: {
            "event-1": {
                id: "event-1",
                presentation: eventPresentation(zoned),
                status: {voting_status: eventStatus} as unknown as string,
            },
        },
        elections: Object.fromEntries(ballots.map(({election}) => [election.id, election])),
        ballotStyles: Object.fromEntries(
            ballots.map(({style}) => [style.election_id, style])
        ) as unknown as RootState["ballotStyles"],
    }
}

const show = async () => {
    const router = createMemoryRouter(
        [
            {
                path: "/tenant/:tenantId/event/:eventId/election-chooser",
                element: <ElectionSelectionScreen />,
            },
        ],
        {initialEntries: ["/tenant/tenant-1/event/event-1/election-chooser"]}
    )
    const {container} = render(
        <ThemeProvider theme={theme}>
            <RouterProvider router={router} />
        </ThemeProvider>
    )
    await screen.findAllByRole("listitem")
    const card = (title: string) => {
        const item = screen.getByRole("heading", {name: title}).closest(".election-item")!
        const text = (selector: string) =>
            item.querySelector(selector)?.textContent?.replace(/\s+/g, " ") ?? null
        return {
            open: text(".election-open-date-value"),
            close: text(".election-close-date-value"),
            closeLocal: text(".election-close-date-local"),
            device: text(".election-device-time"),
        }
    }
    const closed = container.querySelector(".election-selection-closed")?.textContent ?? null
    return {card, closed}
}

const at = (instant: string) => jest.useFakeTimers({now: new Date(instant), advanceTimers: true})
afterEach(() => jest.useRealTimers())

// The Post's opening carries its zone, the event-wide close the primary, as
// get_election_dates writes them.
const MAIN_DATES = zonedElectionDates(DUBAI.opensAt, OVERSEAS.closesAt, {
    openZone: DUBAI.timeZone,
    closeZone: OVERSEAS.primary,
})

describe("ElectionSelectionScreen with timezones", () => {
    it("an event-wide close shows in the primary with the Post's time; a Post's own close in its zone", async () => {
        at(OVERSEAS.now.open)
        setUp(true, "OPEN", [
            ballot("election-1", "Dubai PCG", MAIN_DATES, "OPEN"),
            ballot("election-2", "Dubai PCG test", TEST_VOTING_DUBAI, "CLOSED"),
        ])
        const {card} = await show()
        expect(card("Dubai PCG")).toMatchObject({
            open: "2028-04-09 00:00 Gulf Standard Time",
            close: "2028-05-08 19:00 Philippine Standard Time",
            closeLocal: "2028-05-08 15:00 Gulf Standard Time",
        })
        expect(card("Dubai PCG test")).toMatchObject({
            open: "2028-02-09 00:00 Gulf Standard Time",
            close: "2028-04-08 23:59 Gulf Standard Time",
            closeLocal: null,
        })
    })

    it("after the close, the closed message gives the close in the primary and the Post's", async () => {
        at(OVERSEAS.now.closed)
        setUp(true, "CLOSED", [ballot("election-1", "Dubai PCG", MAIN_DATES, "CLOSED")])
        const {closed} = await show()
        expect(closed?.replace(/\s+/g, " ")).toBe(
            "electionSelectionScreen.votingClosedAt|2028-05-08 19:00 Philippine Standard Time|2028-05-08 15:00 Gulf Standard Time"
        )
    })
})

describe("ElectionSelectionScreen without timezones (an event published before them)", () => {
    it("renders the dates as before: plain times in the browser's zone, no zone, no device line", async () => {
        at(OVERSEAS.now.open)
        setUp(false, "OPEN", [ballot("election-1", "Dubai PCG", MAIN_DATES, "OPEN")])
        const {card} = await show()
        const local = (instant: string) => {
            const date = new Date(instant)
            const pad = (value: number) => String(value).padStart(2, "0")
            return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(
                date.getDate()
            )} ${pad(date.getHours())}:${pad(date.getMinutes())}`
        }
        expect(card("Dubai PCG")).toEqual({
            open: local(DUBAI.opensAt),
            close: local(OVERSEAS.closesAt),
            closeLocal: null,
            device: null,
        })
    })

    it("shows no closed message", async () => {
        at(OVERSEAS.now.closed)
        setUp(false, "CLOSED", [ballot("election-1", "Dubai PCG", MAIN_DATES, "CLOSED")])
        const {closed} = await show()
        expect(closed).toBeNull()
    })
})
