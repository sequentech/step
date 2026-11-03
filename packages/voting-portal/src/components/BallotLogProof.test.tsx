// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {render, screen} from "@testing-library/react"
import {BallotLogProof} from "../../../ui-essentials/src/components/BallotLogProof/BallotLogProof"

const mockVerify = jest.fn()
jest.mock("@sequentech/ui-core", () => ({
    verifyBallotLogProof: (...args: unknown[]) => mockVerify(...args),
}))
jest.mock("react-i18next", () => ({useTranslation: () => ({t: (key: string) => key})}))
const trust = {
    tenant_id: "tenant",
    election_event_id: "event",
    election_id: "election",
    system_public_key: "system",
    ballot_box_key: {key_id: "key", public_key: "box"},
}
const proof = {
    ballot_id: "FTBE-MHRX",
    pseudonym: "7QKM-2HXD",
    voting_channel: "ONLINE",
    received_entry: null,
    cast_entry: null,
}

test("missing signed entry waits while existing successful checks stay visible", () => {
    mockVerify.mockReturnValue({
        checks: {binding: "passed", received: "passed", ballotId: "passed", cast: "pending"},
    })
    render(<BallotLogProof trust={trust} proof={proof} />)
    expect(mockVerify).toHaveBeenCalledWith(trust, proof)
    expect(screen.getByText("ballotPrivacy.pending")).toBeInTheDocument()
    expect(screen.queryByText("ballotPrivacy.passed")).not.toBeInTheDocument()
    expect(screen.getAllByText("ballotPrivacy.states.passed")).toHaveLength(3)
})

test("a failed check takes priority over another pending check", () => {
    mockVerify.mockReturnValue({
        checks: {binding: "failed", received: "pending", ballotId: "pending", cast: "pending"},
    })
    render(<BallotLogProof trust={trust} proof={proof} />)
    expect(screen.getByText("ballotPrivacy.failedHelp")).toBeInTheDocument()
    expect(screen.queryByText("ballotPrivacy.pendingHelp")).not.toBeInTheDocument()
})

test("verification failure or missing report cannot be presented as success", () => {
    mockVerify.mockImplementation(() => {
        throw new Error("Invalid signed bytes")
    })
    render(<BallotLogProof trust={trust} proof={proof} />)
    expect(screen.getByText("ballotPrivacy.failed")).toBeInTheDocument()
    expect(screen.queryByText("ballotPrivacy.passed")).not.toBeInTheDocument()
})
