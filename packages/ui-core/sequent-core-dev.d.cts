// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export type SequentCoreDevState = "ok" | "failed"

export interface SequentCoreDevStatus {
    schema: number
    state: SequentCoreDevState
    published?: {
        build: string
        fingerprint: string
        source_fingerprint: string
        built_at: string
    }
    last_attempt?: {
        fingerprint: string | null
        at: string
        log?: string
        error?: string
    }
}

export interface SequentCoreViteAlias {
    find: RegExp
    replacement: string
}

export declare function sequentCoreDevStatus(packagesDir?: string): SequentCoreDevStatus | undefined

export declare function sequentCoreWebpackAlias(
    mode: string | undefined,
    packagesDir?: string
): Record<string, string>

export declare function sequentCoreViteAlias(packagesDir?: string): Array<SequentCoreViteAlias>
