// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import config from "../ui-test-kit/eslint.config.mjs"
export default config.map((entry) =>
    entry.files ? {...entry, files: ["**/*.{js,mjs,ts,mts}"]} : entry
)
