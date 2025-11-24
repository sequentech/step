// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import js from "@eslint/js"
import react from "eslint-plugin-react"
import reactHooks from "eslint-plugin-react-hooks"
import typescriptEslint from "@typescript-eslint/eslint-plugin"
import typescriptParser from "@typescript-eslint/parser"
import globals from "globals"

export default [
    {
        ignores: [
            "node_modules/**",
            "dist/**",
            "target/**",
            "pkg/**",
            "build/**",
            "src/gql/**",
            "public/tinymce/**",
            "public/intl-tel-input/**",
            "public/sql-wasm.js",
            "*.config.js",
            "*.config.ts",
            "**/*.mdx",
            "**/*.license",
        ],
    },
    {
        files: ["**/*.{js,jsx,ts,tsx}"],
        languageOptions: {
            ecmaVersion: "latest",
            sourceType: "module",
            parser: typescriptParser,
            parserOptions: {
                ecmaFeatures: {
                    jsx: true,
                },
            },
            globals: {
                ...globals.browser,
                ...globals.es2021,
                ...globals.jquery,
                ...globals.webextensions,
                ...globals.node,
                JSX: "readonly",
            },
        },
        plugins: {
            react,
            "react-hooks": reactHooks,
            "@typescript-eslint": typescriptEslint,
        },
        rules: {
            ...js.configs.recommended.rules,
            "react/prop-types": "off",
            "react/react-in-jsx-scope": "off",
            // Disable problematic rules that aren't compatible with ESLint 9
            "react/no-string-refs": "off",
            // Allow unused variables - turn off the rule entirely
            "no-unused-vars": "off",
            "@typescript-eslint/no-unused-vars": "off",
            // Allow redeclare for function overloads
            "no-redeclare": "off",
        },
        settings: {
            react: {
                version: "detect",
            },
        },
    },
    {
        files: ["**/*.stories.*"],
        rules: {
            "import/no-anonymous-default-export": "off",
        },
    },
    {
        files: ["**/*.test.*", "**/test/**/*", "**/nightwatch/**/*"],
        languageOptions: {
            globals: {
                ...globals.jest,
                ...globals.mocha,
                describe: "readonly",
                it: "readonly",
                before: "readonly",
                beforeEach: "readonly",
                after: "readonly",
                afterEach: "readonly",
                expect: "readonly",
            },
        },
        rules: {
            // Disable Jest-specific rules for Nightwatch tests
            "jest/valid-describe-callback": "off",
        },
    },
    {
        files: ["**/*.config.js", "**/setupProxy.js", "**/nightwatch.conf.js"],
        languageOptions: {
            sourceType: "commonjs",
            globals: {
                ...globals.node,
                module: "writable",
                exports: "writable",
                require: "readonly",
                __dirname: "readonly",
            },
        },
    },
]
