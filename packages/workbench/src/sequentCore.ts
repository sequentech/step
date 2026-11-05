// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

/** The sequent-core build the workbench loaded, as described by its Vite configuration. */
export interface SequentCoreInfo {
    directory: string
    wasmBytes: number
    wasmSha256: string
    /** ISO 8601 modification time of the WASM binary. */
    modifiedAt: string
}
