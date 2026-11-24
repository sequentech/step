// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export const PRODUCT_RETURN_PATH = "/operational-signing/product-return"

export interface ProductCode {
    state: string
    code: string
    issuer?: string
}

/** Remove the secret from the URL before loading the portal or any API client. */
export function captureProductGoldReturn(
    location: Pick<Location, "pathname" | "search" | "hash" | "protocol">,
    clean: (path: string) => void
): ProductCode | undefined {
    if (location.pathname !== PRODUCT_RETURN_PATH) return undefined
    const {hash, search, protocol} = location
    try {
        clean(PRODUCT_RETURN_PATH)
    } catch {
        return undefined
    }
    if (protocol !== "https:" || search || !hash.startsWith("#") || hash.length > 8192)
        return undefined
    const parts = new URLSearchParams(hash.slice(1))
    const keys = Array.from(parts.keys())
    if (
        keys.some((key) => !["state", "code", "session_state", "iss"].includes(key)) ||
        new Set(keys).size !== keys.length
    )
        return undefined
    const state = parts.get("state")
    const code = parts.get("code")
    const issuer = parts.get("iss") ?? undefined
    if (issuer !== undefined && (issuer.length === 0 || issuer.length > 1024)) return undefined
    return state && /^[A-Za-z0-9_-]{43}$/.test(state) && code && /^[!-~]{1,4096}$/.test(code)
        ? {state, code, issuer}
        : undefined
}
