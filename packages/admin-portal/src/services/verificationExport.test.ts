// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {verificationBoards, ProtocolBoardRecord} from "./verificationExport"

const tenant = "10000000-0000-4000-8000-000000000001"
const event = "20000000-0000-4000-8000-000000000002"
const keys = "30000000-0000-4000-8000-000000000003"
const session = "40000000-0000-4000-8000-000000000004"
const parent: ProtocolBoardRecord = {
    id: "50000000-0000-4000-8000-000000000005",
    tenant_id: tenant,
    election_event_id: event,
    name: `dkg_${keys.replace(/-/g, "")}`,
    parent_id: null,
    keys_ceremony_id: keys,
    tally_session_id: null,
    batch: null,
    manager_message: "\\x0102ff",
}
const child: ProtocolBoardRecord = {
    ...parent,
    id: "ffffffff-ffff-4fff-bfff-ffffffffffff",
    name: `tally_${session.replace(/-/g, "")}_0`,
    parent_id: parent.id,
    tally_session_id: session,
    batch: 0,
    manager_message: "\\x0304",
}
const hash = "ab".repeat(64)
const verifier = () => ({
    configurationHash: jest.fn(() => hash),
    fetchTranscript: jest.fn(async () => '{"unchanged":"wire transcript"}\n'),
})
const scope = {tenantId: tenant, eventId: event, tallySessionId: session}

it("pins each batch to its trusted parent configuration and full u128 row ID", () => {
    const second = {
        ...child,
        id: "ffffffff-ffff-4fff-bfff-fffffffffffe",
        batch: 1,
        name: `tally_${session.replace(/-/g, "")}_1`,
    }
    const wasm = verifier()
    const rows = verificationBoards([parent, second, child], scope, wasm)
    expect(rows).toHaveLength(2)
    expect(rows[0].configurationHash).toBe(hash)
    expect(wasm.configurationHash).toHaveBeenCalledWith("AQL/")
    expect(rows[0].tallyId).toBe(BigInt("0xffffffffffff4fffbfffffffffffffff").toString(10))
    expect(rows[0].tallyId).not.toBe(rows[1].tallyId)
    expect(rows[0].tallyId).not.toBe(BigInt("0x" + session.replace(/-/g, "")).toString(10))
})

it.each([
    ["foreign tenant", {...child, tenant_id: "other"}],
    ["foreign event", {...child, election_event_id: "other"}],
    ["missing parent", {...child, parent_id: "missing"}],
    ["wrong key ceremony", {...child, keys_ceremony_id: "other"}],
    ["wrong board name", {...child, name: "tally_unrelated_0"}],
    ["unsafe batch", {...child, batch: Number.MAX_SAFE_INTEGER + 1}],
    ["invalid row UUID", {...child, id: "not-a-uuid"}],
])("refuses %s instead of exporting a mismatched board", (_label, changed) => {
    expect(() =>
        verificationBoards([parent, changed as ProtocolBoardRecord], scope, verifier())
    ).toThrow()
})

it("refuses an ambiguous parent or malformed public configuration", () => {
    expect(() => verificationBoards([parent, parent, child], scope, verifier())).toThrow()
    expect(() =>
        verificationBoards([{...parent, manager_message: "\\x012"}, child], scope, verifier())
    ).toThrow()
    const wasm = verifier()
    wasm.configurationHash.mockImplementation(() => {
        throw new Error("invalid signature")
    })
    expect(() => verificationBoards([parent, child], scope, wasm)).toThrow()
})

it("never substitutes another tally session when the selected session has no boards", () => {
    expect(
        verificationBoards([parent, {...child, tally_session_id: "other"}], scope, verifier())
    ).toEqual([])
})
