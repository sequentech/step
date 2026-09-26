// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Vite serves a file imported with `?url` and returns its address.
declare module "*?url" {
    const url: string
    export default url
}
