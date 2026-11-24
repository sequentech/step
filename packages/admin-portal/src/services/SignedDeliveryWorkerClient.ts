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
