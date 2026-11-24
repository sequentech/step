// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import init, {decryptSignedDelivery, encryptedSignedDeliveryFileLimit} from "sequent-core"
import type {SignedDeliveryReply} from "./SignedDeliveryWorkerClient"

const scope = globalThis as unknown as {
    onmessage: (event: MessageEvent<{file: File; password: string}>) => void
    postMessage: (message: SignedDeliveryReply, transfer?: ArrayBuffer[]) => void
}

scope.onmessage = async ({data: {file, password}}) => {
    try {
        // Keep codec JS in the worker entry: Webpack async chunk loaders for
        // web pages often need document, which a dedicated worker has not.
        await init()
        if (!crypto.subtle) throw new Error("Secure context required")
    } catch {
        scope.postMessage({ok: false, reason: "unavailable"})
        return
    }
    if (file.size > encryptedSignedDeliveryFileLimit()) {
        scope.postMessage({ok: false, reason: "size"})
        return
    }
    try {
        const sealed = new Uint8Array(await file.arrayBuffer())
        const bytes = new Uint8Array(decryptSignedDelivery(sealed, password))
        password = ""
        const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", bytes))
        const sha256 = Array.from(digest, (b) => b.toString(16).padStart(2, "0")).join("")
        scope.postMessage({ok: true, bytes, sha256}, [bytes.buffer])
    } catch {
        // Do not reflect WASM or file errors, which could carry input content.
        scope.postMessage({ok: false, reason: "decrypt"})
    } finally {
        password = ""
    }
}
