// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

/// <reference types="vite/client" />

interface ImportMetaEnv {
    /** Base URL of the voting portal Storybook, for links to equivalent stories. */
    readonly WORKBENCH_STORYBOOK_URL?: string
}

declare module "virtual:workbench/sequent-core" {
    const info: import("./sequentCore").SequentCoreInfo
    export default info
}
