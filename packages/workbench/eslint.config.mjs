// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import js from "@eslint/js"
import react from "eslint-plugin-react"
import reactHooks from "eslint-plugin-react-hooks"
import typescriptParser from "@typescript-eslint/parser"
import globals from "globals"

export default [
    {
        ignores: ["node_modules/**", "dist/**", "test-results/**", "playwright-report/**"],
    },
    {
        files: ["**/*.{js,mjs,ts,tsx}"],
        languageOptions: {
            ecmaVersion: "latest",
            sourceType: "module",
            parser: typescriptParser,
            parserOptions: {ecmaFeatures: {jsx: true}},
            globals: {...globals.browser, ...globals.node},
        },
        plugins: {react, "react-hooks": reactHooks},
        rules: {
            ...js.configs.recommended.rules,
            ...reactHooks.configs.recommended.rules,
            "react/jsx-uses-vars": "error",
            // TypeScript reports unused and undefined names with type information.
            "no-unused-vars": "off",
            "no-undef": "off",
            "no-redeclare": "off",
        },
        settings: {react: {version: "detect"}},
    },
    {
        // Playwright fixtures call their `use` argument, which is not React's hook.
        files: ["tests/**"],
        rules: {"react-hooks/rules-of-hooks": "off"},
    },
]
