// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Vite inlines a certificate fixture imported with ?url&inline as a data URL, so
// the stories open real .p12 files without a network request.
declare module "*.p12?url&inline" {
    const dataUrl: string
    export default dataUrl
}
