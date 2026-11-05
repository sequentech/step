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
    ballot_id: "QR7T-K2NM",
    tracking_id: "FTBE-MHRX-0123-4567-89AB-CDEF-GHJK-MNPQ-RSTV-WXYZ-0123-4567-89AB",
    pseudonym: "K7QM-2XWD",
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

test("the full Tracking ID is shown with how it is computed, and the pseudonym with a colon", () => {
    mockVerify.mockReturnValue({
        checks: {binding: "passed", received: "passed", ballotId: "passed", cast: "passed"},
    })
    const {container} = render(<BallotLogProof trust={trust} proof={proof} />)
    expect(container.querySelector(".ballot-proof-tracking-id")).toHaveTextContent(
        proof.tracking_id
    )
    expect(screen.getByText("ballotPrivacy.trackingIdHelp")).toBeInTheDocument()
    expect(screen.getByText("K7QM:2XWD")).toBeInTheDocument()
    expect(screen.queryByText("K7QM-2XWD")).not.toBeInTheDocument()
})

test.each(["received", "audited"] as const)(
    "a %s ballot has no cast entry to wait for, and says what that means",
    (stage) => {
        mockVerify.mockReturnValue({
            checks: {binding: "passed", received: "passed", ballotId: "passed", cast: "pending"},
        })
        render(<BallotLogProof trust={trust} proof={{...proof, stage}} />)
        expect(screen.getByText(`ballotPrivacy.stages.${stage}.title`)).toBeInTheDocument()
        expect(screen.getByText(`ballotPrivacy.stages.${stage}.help`)).toBeInTheDocument()
        expect(screen.getByText(`ballotPrivacy.stages.${stage}.castState`)).toBeInTheDocument()
        expect(screen.queryByText("ballotPrivacy.pending")).not.toBeInTheDocument()
        expect(screen.queryByText("ballotPrivacy.states.pending")).not.toBeInTheDocument()
    }
)

test("an early step that fails is still a failure for a received ballot", () => {
    mockVerify.mockReturnValue({
        checks: {binding: "passed", received: "failed", ballotId: "pending", cast: "pending"},
    })
    render(<BallotLogProof trust={trust} proof={{...proof, stage: "received"}} />)
    expect(screen.getByText("ballotPrivacy.failed")).toBeInTheDocument()
    expect(screen.queryByText("ballotPrivacy.stages.received.title")).not.toBeInTheDocument()
})

test("a cast ballot whose cast entry is missing still waits for it", () => {
    mockVerify.mockReturnValue({
        checks: {binding: "passed", received: "passed", ballotId: "passed", cast: "pending"},
    })
    render(<BallotLogProof trust={trust} proof={{...proof, stage: "cast"}} />)
    expect(screen.getByText("ballotPrivacy.pending")).toBeInTheDocument()
})
