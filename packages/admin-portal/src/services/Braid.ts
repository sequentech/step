// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

type BraidModule = typeof import("braid-wasm")
let loading: Promise<BraidModule> | undefined

export function loadBraid(): Promise<BraidModule> {
    if (!loading) {
        loading = (async () => {
            // The vendored rayon package needs its original browser module URLs
            // to start workers; webpack copies the package without bundling it.
            const url = new URL("/braid-wasm/braid.js", window.location.origin).href
            const braid: BraidModule = await import(/* webpackIgnore: true */ url)
            await braid.default({})
            await braid.initThreadPool(navigator.hardwareConcurrency || 4)
            return braid
        })().catch((error) => {
            loading = undefined
            throw error
        })
    }
    return loading
}
