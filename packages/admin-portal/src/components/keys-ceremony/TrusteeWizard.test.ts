/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {cleanup, render, screen} from "@testing-library/react"
import {Sequent_Backend_Election_Event, Sequent_Backend_Keys_Ceremony} from "@/gql/graphql"
import {
    IKeysCeremonyExecutionStatus as EStatus,
    IKeysCeremonyTrusteeStatus as TStatus,
} from "@/services/KeyCeremony"
import {CeremonyStepProps} from "./CeremonyStep"
import {TrusteeWizard, WizardStep} from "./TrusteeWizard"

interface BreadCrumbStepsProps {
    selected: number
}

const TRUSTEE_NAME = "trustee-1"
const CEREMONY_ID = "ceremony-1"

const mockBreadCrumbSteps = jest.fn((props: BreadCrumbStepsProps): null => null)
const mockCeremonyStep = jest.fn((props: CeremonyStepProps): null => null)

jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key}),
}))
jest.mock("@/providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({trustee: "trustee-1"}),
}))
jest.mock("@sequentech/ui-core", () => ({
    EElectionEventCeremoniesPolicy: {
        MANUAL_CEREMONIES: "MANUAL_CEREMONIES",
        AUTOMATED_CEREMONIES: "AUTOMATED_CEREMONIES",
    },
}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({
        BreadCrumbSteps: (props: BreadCrumbStepsProps) => mockBreadCrumbSteps(props),
        BreadCrumbStepsVariant: {Circle: "Circle"},
    }),
    {virtual: true}
)
jest.mock("@/components/styles/WizardStyles", () => ({
    WizardStyles: {
        WizardWrapper: ({children}: {children: React.ReactNode}) =>
            require("react").createElement("div", null, children),
    },
}))
jest.mock("./CeremonyStep", () => ({
    CeremonyStep: (props: CeremonyStepProps) => mockCeremonyStep(props),
}))
jest.mock("./StartStep", () => ({
    StartStep: () => require("react").createElement("div", {"data-testid": "start-step"}),
}))
jest.mock("./DownloadStep", () => ({
    DownloadStep: () => require("react").createElement("div", {"data-testid": "download-step"}),
}))
jest.mock("./CheckStep", () => ({
    CheckStep: () => require("react").createElement("div", {"data-testid": "check-step"}),
}))

const electionEvent = {
    id: "election-event-1",
    presentation: {},
} as unknown as Sequent_Backend_Election_Event

const buildCeremony = (
    executionStatus: EStatus,
    trustees: Array<{name: string; status: TStatus}>
): Sequent_Backend_Keys_Ceremony =>
    ({
        id: CEREMONY_ID,
        execution_status: executionStatus,
        settings: {},
        status: {
            public_key: "public-key",
            logs: [],
            trustees,
        },
    }) as unknown as Sequent_Backend_Keys_Ceremony

const renderWizard = (currentCeremony: Sequent_Backend_Keys_Ceremony) =>
    render(
        React.createElement(TrusteeWizard, {
            electionEvent,
            currentCeremony,
            setCurrentCeremony: jest.fn(),
            goBack: jest.fn(),
        })
    )

const lastCeremonyStepProps = (): CeremonyStepProps => {
    const props = mockCeremonyStep.mock.lastCall?.[0]
    if (!props) {
        throw new Error("CeremonyStep was not rendered")
    }
    return props
}

const lastSelectedStep = (): number | undefined => mockBreadCrumbSteps.mock.lastCall?.[0].selected

beforeEach(() => {
    mockBreadCrumbSteps.mockClear()
    mockCeremonyStep.mockClear()
})

afterEach(cleanup)

describe("TrusteeWizard terminal routing", () => {
    it.each([
        [EStatus.SUCCESS, TStatus.KEY_CHECKED],
        [EStatus.CANCELLED, TStatus.KEY_GENERATED],
    ])(
        "shows the status view without a Next action for a %s ceremony",
        (executionStatus, trusteeStatus) => {
            renderWizard(
                buildCeremony(executionStatus, [{name: TRUSTEE_NAME, status: trusteeStatus}])
            )

            expect(lastSelectedStep()).toBe(WizardStep.Status)
            expect(lastCeremonyStepProps().currentCeremonyId).toBe(CEREMONY_ID)
            expect(lastCeremonyStepProps().goNext).toBeUndefined()
            expect(screen.queryByTestId("start-step")).toBeNull()
            expect(screen.queryByTestId("download-step")).toBeNull()
            expect(screen.queryByTestId("check-step")).toBeNull()
        }
    )

    it("keeps the Next action for a participating trustee whose keys are still generating", () => {
        renderWizard(
            buildCeremony(EStatus.IN_PROGRESS, [
                {name: TRUSTEE_NAME, status: TStatus.KEY_GENERATED},
                {name: "trustee-2", status: TStatus.WAITING},
            ])
        )

        expect(lastSelectedStep()).toBe(WizardStep.Not_Generated)
        expect(lastCeremonyStepProps().goNext).toBeDefined()
        expect(lastCeremonyStepProps().isNextDisabled).toBe(true)
    })

    it("sends a participating trustee with generated keys to the start step", () => {
        renderWizard(
            buildCeremony(EStatus.IN_PROGRESS, [
                {name: TRUSTEE_NAME, status: TStatus.KEY_RETRIEVED},
            ])
        )

        expect(lastSelectedStep()).toBe(WizardStep.Start)
        expect(screen.getByTestId("start-step")).not.toBeNull()
        expect(mockCeremonyStep).not.toHaveBeenCalled()
    })
})
