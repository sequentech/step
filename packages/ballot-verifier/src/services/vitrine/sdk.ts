// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type * as VitrineVerify from "@vitrine/verify"
import type * as VitrinePure from "@vitrine/verify/pure"
import type * as VitrineUi from "@vitrine/ui"

export type TVitrineVerify = typeof VitrineVerify
export type TVitrinePure = typeof VitrinePure
export type TVitrineUi = typeof VitrineUi

export interface IVitrineSdk {
    verify: TVitrineVerify
    pure: TVitrinePure
}

let sdk: Promise<IVitrineSdk> | undefined
let ui: Promise<TVitrineUi> | undefined

// Both are loaded on first use, so that the Benaloh audit never downloads them.

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

/** The UI kit, with its elements (`<vitrine-receipt>`, `<vitrine-proof-steps>`
 * and the others) defined. */
export const loadVitrineUi = (): Promise<TVitrineUi> => {
    if (!ui) {
        const loading = import("@vitrine/ui").then((kit) => {
            kit.defineAll()
            return kit
        })
        loading.catch(() => {
            if (ui === loading) ui = undefined
        })
        ui = loading
    }
    return ui
}
