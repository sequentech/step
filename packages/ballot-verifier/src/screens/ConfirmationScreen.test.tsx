// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {render, screen, cleanup} from "@testing-library/react"
import {ConfirmationScreen} from "./ConfirmationScreen"
import {IConfirmationBallot} from "../services/BallotService"
import {groupBallotId} from "../services/vitrine/ballotId"

jest.mock("react-i18next", () => ({useTranslation: () => ({t: (key: string) => key})}))
jest.mock("react-router-dom", () => ({useNavigate: () => jest.fn()}))
jest.mock("./hooks/useElectionClassName", () => ({useElectionClassName: jest.fn()}))
jest.mock("../providers/SettingsContextProvider", () => ({
    SettingsContext: require("react").createContext({globalSettings: {PUBLIC_BUCKET_URL: ""}}),
}))
jest.mock("@sequentech/ui-core", () => ({
    sortContestList: (contests: unknown[]) => contests,
    isAcclaimedContest: () => false,
    EDeclineToVotePolicy: {ENABLED: "enabled"},
    EBlankBallotsPolicy: {ENABLED: "enabled"},
    EElectionEventContestEncryptionPolicy: {MULTIPLE_CONTESTS: "multiple-contests"},
}))
jest.mock("@sequentech/ui-essentials", () => ({
    theme: {
        palette: {
            customGrey: {main: "#666"},
            blue: {light: "#cce5ff", main: "#292f99"},
            red: {light: "#fecaca", main: "#991b1b"},
        },
    },
    PageLimit: ({children}: React.PropsWithChildren) => <div>{children}</div>,
    ActionsContainer: ({children}: React.PropsWithChildren) => <div>{children}</div>,
    StyledButton: require("@mui/material/Button").default,
    Icon: () => <span aria-hidden="true" />,
    IconButton: ({ariaLabel}: {ariaLabel?: string}) => <button aria-label={ariaLabel} />,
    Dialog: () => null,
    BreadCrumbSteps: () => <ol />,
    PlaintextVoteContest: () => null,
}))

afterEach(cleanup)

const DECODED = "a41c7e09e2c8913f07ba6d51f4a2c83915e79b0d62fa3c48a91e0d75be364f02"
const PROVIDED = "a41c7e095b3d28f6c0e19d743a58b2f06e1d84c90b7f5a23d816e4c279b013af"

const renderWith = (ballotId: string) =>
    render(
        <ConfirmationScreen
            confirmationBallot={
                {
                    ballot_hash: DECODED,
                    election_config: {description: "", contests: []},
                    decoded_questions: [],
                } as unknown as IConfirmationBallot
            }
            ballotId={ballotId}
        />
    )

describe("the confirmation screen's Ballot IDs", () => {
    it("shows both IDs in groups of four, the first two in bold", () => {
        renderWith(groupBallotId(DECODED))
        const decoded = screen.getByTestId("decoded-ballot-id")
        expect(decoded).toHaveTextContent(groupBallotId(DECODED))
        expect(Array.from(decoded.querySelectorAll("b")).map((b) => b.textContent)).toEqual([
            "a41c",
            "7e09",
        ])
        expect(screen.getByTestId("provided-ballot-id")).toHaveTextContent(groupBallotId(DECODED))
    })

    it("matches the ID whatever its spacing and case, and goes on to the selections", () => {
        renderWith(groupBallotId(DECODED).toUpperCase())
        expect(screen.queryByTestId("ballot-id-mismatch")).toBeNull()
        expect(screen.queryByTestId("provided-ballot-id")?.querySelector("mark")).toBeNull()
        expect(screen.getByText("confirmationScreen.verifySelectionsTitle")).toBeInTheDocument()
    })

    it("highlights the groups that differ, and holds back the selections", () => {
        renderWith(groupBallotId(PROVIDED))
        const provided = screen.getByTestId("provided-ballot-id")
        const different = Array.from(provided.querySelectorAll("mark")).map((m) => m.textContent)
        expect(different).toHaveLength(14)
        expect(different[0]).toBe("5b3d")
        expect(screen.getByTestId("decoded-ballot-id").querySelector("mark")).toBeNull()
        expect(screen.getByTestId("ballot-id-mismatch")).toHaveTextContent(
            "confirmationScreen.ballotIdError"
        )
        expect(screen.queryByText("confirmationScreen.verifySelectionsTitle")).toBeNull()
    })
})
