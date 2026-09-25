// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import js from "@eslint/js"
import typescriptParser from "@typescript-eslint/parser"
import globals from "globals"

export default [
    {
        ignores: ["node_modules/**", "test-results/**", "playwright-report/**", "**/*.license"],
    },
    {
        files: ["**/*.{js,mjs,ts}"],
        languageOptions: {
            ecmaVersion: "latest",
            sourceType: "module",
            parser: typescriptParser,
            globals: {
                ...globals.node,
                ...globals.browser,
            },
        },
        rules: {
            ...js.configs.recommended.rules,
            // TypeScript reports unused and undefined names with type information.
            "no-unused-vars": "off",
            "no-undef": "off",
            "no-redeclare": "off",
        },
    },
]
