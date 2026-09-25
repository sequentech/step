// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {mergeConfig} from "vite"
import type {StorybookConfig} from "@storybook/react-vite"
import config from "../../ui-essentials/.storybook/main.ts"

export default {
    ...config,
    staticDirs: ["../public"],
    viteFinal: async (viteConfig, options) =>
        mergeConfig(await config.viteFinal!(viteConfig, options), {
            // Include screen and coverage-only imports before Vitest starts its browser.
            // Late discovery invalidates loaded modules while a story is running.
            optimizeDeps: {
                include: [
                    "keycloak-js",
                    "@apollo/client",
                    "@apollo/client/link/context",
                    "@apollo/client/react",
                    "@apollo/client/errors",
                    "web-vitals",
                    "@mui/icons-material/ChevronLeft",
                    "@mui/material/Tabs",
                    "@mui/material/Tab",
                    "@mui/material/TableSortLabel",
                    "@mui/material/TablePagination",
                    "@emotion/styled",
                    "@mui/icons-material/Visibility",
                    "@mui/icons-material/VideoFile",
                    "@mui/icons-material/AudioFile",
                    "@mui/icons-material/PictureAsPdf",
                    "@mui/icons-material/Image",
                    "@mui/icons-material/Description",
                    "cross-fetch/polyfill",
                    "@testing-library/jest-dom",
                ],
            },
        }),
} satisfies StorybookConfig
