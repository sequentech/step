// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {defineConfig, mergeConfig} from "vitest/config"
import viteConfig from "./vite.config"

export default mergeConfig(
    viteConfig,
    defineConfig({test: {include: ["src/**/*.test.{ts,tsx}"], environment: "node"}})
)
