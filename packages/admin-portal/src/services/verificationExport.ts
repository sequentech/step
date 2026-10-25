// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

/** Public rows obtained from the authenticated platform, never from the export being verified. */
export interface ProtocolBoardRecord {
    id: string
    tenant_id: string
    election_event_id: string
    parent_id: string | null
    keys_ceremony_id: string
    tally_session_id: string | null
    batch: number | null
    name: string
    manager_message: string
}
export interface TranscriptVerifier {
    configurationHash(message: string): string
}
export interface VerificationBoard {
    id: string
    name: string
    dkgName: string
    configurationHash: string
    tallyId: string
}
const uuid = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/

function publicMessageBase64(value: string): string {
    // Hasura serializes PostgreSQL bytea as its lossless hexadecimal form.
    if (!/^\\x(?:[0-9a-fA-F]{2})+$/.test(value)) throw new Error("Invalid public configuration")
    let bytes = ""
    for (let index = 2; index < value.length; index += 2) {
        bytes += String.fromCharCode(parseInt(value.slice(index, index + 2), 16))
    }
    return btoa(bytes)
}

export function verificationBoards(
    rows: ReadonlyArray<ProtocolBoardRecord>,
    scope: {tenantId: string; eventId: string; tallySessionId: string},
    verifier: Pick<TranscriptVerifier, "configurationHash">
): VerificationBoard[] {
    const ids = new Set<string>()
    for (const row of rows) {
        if (
            row.tenant_id !== scope.tenantId ||
            row.election_event_id !== scope.eventId ||
            !uuid.test(row.id) ||
            ids.has(row.id)
        )
            throw new Error("Invalid protocol board scope")
        ids.add(row.id)
    }
    return rows
        .filter((row) => row.tally_session_id === scope.tallySessionId)
        .sort((left, right) => (left.batch ?? -1) - (right.batch ?? -1))
        .map((row) => {
            const parent = rows.find((candidate) => candidate.id === row.parent_id)
            if (
                !parent ||
                parent.parent_id !== null ||
                parent.tally_session_id !== null ||
                !uuid.test(row.keys_ceremony_id) ||
                !uuid.test(scope.tallySessionId) ||
                parent.keys_ceremony_id !== row.keys_ceremony_id ||
                parent.name !== `dkg_${row.keys_ceremony_id.replace(/-/g, "")}` ||
                row.batch === null ||
                !Number.isSafeInteger(row.batch) ||
                row.batch < 0 ||
                row.name !== `tally_${scope.tallySessionId.replace(/-/g, "")}_${row.batch}`
            ) {
                throw new Error("Invalid tally board relationship")
            }
            const configurationHash = verifier.configurationHash(
                publicMessageBase64(parent.manager_message)
            )
            if (!/^[0-9a-f]{128}$/.test(configurationHash))
                throw new Error("Invalid configuration hash")
            return {
                id: row.id,
                name: row.name,
                dkgName: parent.name,
                configurationHash,
                // Rust Uuid::as_u128(): use BigInt, never a lossy JavaScript Number.
                tallyId: BigInt("0x" + row.id.replace(/-/g, "")).toString(10),
            }
        })
}
