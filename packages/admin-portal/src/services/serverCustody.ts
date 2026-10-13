// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

export interface IServerCustodyScope {
    election_event_id: string
    keys_ceremony_id?: string
    tally_session_id?: string
}
export enum ServerCustodyRetryPolicy {
    Keep = "KEEP",
    RetryFailed = "RETRY_FAILED",
}
export interface IServerCustodyInput extends IServerCustodyScope {
    custody_request_json: string
    retry_policy: ServerCustodyRetryPolicy
}
export interface IServerCustodyReply {
    status: "PENDING" | "READY" | "FAILED"
    custody_proof_json?: string
    detail?: string
}
export class ServerCustodyError extends Error {
    constructor(
        public readonly reason: "unavailable" | "failed" | "invalidReply" | "cancelled",
        public readonly detail?: string
    ) {
        super(detail || `The trustee server ${reason}; upload the ceremony file again to retry.`)
    }
}

/** An offline proof is submitted as before. A saved public ceremony file
 * requests a fresh proof from the server without supplying its tally scope. */
export async function resolveCustodyFile(
    text: string,
    scope: IServerCustodyScope,
    request: (input: IServerCustodyInput) => Promise<IServerCustodyReply>,
    pause: () => Promise<void> = () => new Promise((resolve) => setTimeout(resolve, 2000)),
    maxAttempts = 60,
    signal?: AbortSignal
): Promise<string> {
    if (signal?.aborted) throw new ServerCustodyError("cancelled")
    let file: unknown
    try {
        file = JSON.parse(text)
    } catch {
        return text
    }
    if (!file || typeof file !== "object" || !("keys_ceremony_id" in file)) return text
    for (let attempt = 0; attempt < maxAttempts; attempt++) {
        if (signal?.aborted) throw new ServerCustodyError("cancelled")
        const reply = await request({
            ...scope,
            custody_request_json: text,
            retry_policy:
                attempt === 0
                    ? ServerCustodyRetryPolicy.RetryFailed
                    : ServerCustodyRetryPolicy.Keep,
        })
        if (signal?.aborted) throw new ServerCustodyError("cancelled")
        switch (reply.status) {
            case "READY":
                if (!reply.custody_proof_json) throw new ServerCustodyError("invalidReply")
                return reply.custody_proof_json
            case "FAILED":
                throw new ServerCustodyError("failed", reply.detail)
            case "PENDING":
                if (attempt + 1 < maxAttempts) await pause()
                break
            default:
                throw new ServerCustodyError("invalidReply")
        }
    }
    throw new ServerCustodyError("unavailable")
}
