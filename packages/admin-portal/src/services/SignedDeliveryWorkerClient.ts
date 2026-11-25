// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

const MAGIC = "SEQUENT-EZIP"
export type SignedDeliveryReply =
    | {ok: true; bytes: Uint8Array<ArrayBuffer>; sha256: string}
    | {ok: false; reason: "size" | "decrypt" | "unavailable"}

export const isEncryptedSignedDeliveryFile = async (file: File): Promise<boolean> => {
    // Named encrypted input never falls through to the ordinary ZIP path, even
    // if its magic is damaged or it was produced by the legacy election importer.
    if (/\.ezip$/i.test(file.name)) return true
    const prefix = await new Promise<ArrayBuffer>((resolve, reject) => {
        const reader = new FileReader()
        reader.onload = () => resolve(reader.result as ArrayBuffer)
        reader.onerror = () => reject(new Error("Could not read configuration package"))
        reader.readAsArrayBuffer(file.slice(0, MAGIC.length))
    })
    const bytes = new Uint8Array(prefix)
    return bytes.length === MAGIC.length && bytes.every((byte, i) => byte === MAGIC.charCodeAt(i))
}

/** Version byte of a V2 envelope: messaging credentials travel beside the
 * signed ZIP and are unwrapped only by the server, never in this browser. */
export const CREDENTIALS_DELIVERY_VERSION = 2

/** The envelope version byte, or undefined when this is not an envelope. */
export const signedDeliveryVersion = async (file: File): Promise<number | undefined> => {
    const prefix = await new Promise<ArrayBuffer>((resolve, reject) => {
        const reader = new FileReader()
        reader.onload = () => resolve(reader.result as ArrayBuffer)
        reader.onerror = () => reject(new Error("Could not read configuration package"))
        reader.readAsArrayBuffer(file.slice(0, MAGIC.length + 1))
    })
    const bytes = new Uint8Array(prefix)
    if (bytes.length !== MAGIC.length + 1) return undefined
    if (!Array.from(MAGIC).every((char, i) => bytes[i] === char.charCodeAt(0))) return undefined
    return bytes[MAGIC.length]
}

export const SERVER_UNWRAP_PATH = "/configuration-package-intake/unwrap"

const LOOPBACK_HOSTS = new Set(["localhost", "127.0.0.1", "[::1]"])

/** Where the transfer password may be sent: HTTPS, or plain HTTP to this
 * machine only. Never plain HTTP to any other host. */
export const passwordTransportAllowed = (url: URL): boolean =>
    url.protocol === "https:" || (url.protocol === "http:" && LOOPBACK_HOSTS.has(url.hostname))

export type ServerUnwrapReply =
    | {ok: true; sha256: string}
    | {ok: false; reason: "insecure" | "decrypt" | "mismatch" | "refused" | "unavailable"}

/**
 * Ask Harvest to unwrap an encrypted package already in protected staging.
 * Same origin and direct: never a GraphQL request, so the transfer password
 * is never a logged variable, and the decrypted credentials never come back.
 */
export const unwrapOnServer = async (
    documentId: string,
    password: string,
    token: string | undefined,
    signal: AbortSignal,
    fetchImpl: typeof fetch = (...args) => fetch(...args),
    secure: boolean = globalThis.isSecureContext === true,
    origin: string = globalThis.location?.origin ?? "",
    // The operator's optional SHA-256 of the inner signed ZIP: the server
    // refuses a mismatch before anything is staged.
    expectedSha256?: string
): Promise<ServerUnwrapReply> => {
    let target: URL
    try {
        target = new URL(SERVER_UNWRAP_PATH, origin)
    } catch {
        return {ok: false, reason: "insecure"}
    }
    if (!secure || !passwordTransportAllowed(target)) return {ok: false, reason: "insecure"}
    if (!token) return {ok: false, reason: "refused"}
    let response: Response
    try {
        response = await fetchImpl(target.toString(), {
            method: "POST",
            headers: {
                "Authorization": `Bearer ${token}`,
                "Content-Type": "application/json",
            },
            body: JSON.stringify(
                expectedSha256
                    ? {document_id: documentId, password, expected_sha256: expectedSha256}
                    : {document_id: documentId, password}
            ),
            // Bearer only: no cookies, and no cross-origin target.
            credentials: "omit",
            mode: "same-origin",
            cache: "no-store",
            // Never follow a redirect: the password goes to this exact URL or nowhere.
            redirect: "error",
            signal,
        })
    } catch {
        return {ok: false, reason: "unavailable"}
    }
    if (response.status === 409) return {ok: false, reason: "mismatch"}
    if (response.status === 400) return {ok: false, reason: "decrypt"}
    if (!response.ok)
        return {ok: false, reason: response.status === 403 ? "refused" : "unavailable"}
    try {
        const body: {sha256?: unknown} = await response.json()
        return typeof body.sha256 === "string" && /^[a-f0-9]{64}$/.test(body.sha256)
            ? {ok: true, sha256: body.sha256}
            : {ok: false, reason: "unavailable"}
    } catch {
        return {ok: false, reason: "unavailable"}
    }
}

export const unlockSignedDelivery = (
    file: File,
    password: string,
    signal: AbortSignal
): Promise<SignedDeliveryReply> =>
    new Promise((resolve) => {
        if (signal.aborted) return resolve({ok: false, reason: "unavailable"})
        const worker = new Worker(new URL("/signed-delivery-worker.js", document.baseURI), {
            type: "module",
        })
        const finish = (outcome: SignedDeliveryReply) => {
            worker.terminate()
            signal.removeEventListener("abort", abort)
            resolve(outcome)
        }
        const abort = () => finish({ok: false, reason: "unavailable"})
        signal.addEventListener("abort", abort, {once: true})
        worker.onmessage = (event: MessageEvent<SignedDeliveryReply>) => finish(event.data)
        worker.onerror = abort
        worker.onmessageerror = abort
        // The immutable input file is cloned, not buffered on the UI thread.
        // The transient password never goes to the upload or GraphQL request.
        try {
            worker.postMessage({file, password})
        } catch {
            abort()
        }
    })
