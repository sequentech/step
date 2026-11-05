// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type * as VitrineVerify from "@vitrine/verify"
import type * as VitrinePure from "@vitrine/verify/pure"

export type TVitrineVerify = typeof VitrineVerify
export type TVitrinePure = typeof VitrinePure

export interface IVitrineSdk {
    verify: TVitrineVerify
    pure: TVitrinePure
}

let sdk: Promise<IVitrineSdk> | undefined
let elements: Promise<void> | undefined

// Both are loaded on first use, so that voters of events without
// transparency never download them.

/** The Vitrine verifier, initialised once. */
export const loadVitrineSdk = (): Promise<IVitrineSdk> => {
    if (!sdk) {
        const loading = Promise.all([
            import("@vitrine/verify"),
            import("@vitrine/verify/pure"),
        ]).then(async ([verify, pure]) => {
            await verify.init()
            return {verify, pure}
        })
        loading.catch(() => {
            if (sdk === loading) sdk = undefined
        })
        sdk = loading
    }
    return sdk
}

/** Defines the UI kit's elements (`<vitrine-receipt>`, `<vitrine-status>`,
 * `<vitrine-proof-steps>` and the others). */
export const loadVitrineElements = (): Promise<void> => {
    if (!elements) {
        const loading = import("@vitrine/ui").then((ui) => ui.defineAll())
        loading.catch(() => {
            if (elements === loading) elements = undefined
        })
        elements = loading
    }
    return elements
}
