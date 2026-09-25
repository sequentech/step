// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect} from "react"
import type {Decorator, Preview} from "@storybook/react-vite"
import {INITIAL_VIEWPORTS} from "storybook/viewport"
import {ThemeProvider} from "@mui/material"
import {I18nextProvider} from "react-i18next"
import {i18n, initializeLanguages} from "@sequentech/ui-core"
import theme from "../src/services/theme"
import {withMemoryRouter} from "./withMemoryRouter"

// An explicit language keeps the browser language detector out of the stories.
initializeLanguages({}, "en")

const withTheme: Decorator = (Story) => (
    <ThemeProvider theme={theme}>
        <Story />
    </ThemeProvider>
)

const withI18n: Decorator = (Story, {globals}) => {
    const locale: string = globals.locale ?? "en"

    useEffect(() => {
        void i18n.changeLanguage(locale)
    }, [locale])

    return (
        <I18nextProvider i18n={i18n}>
            <Story />
        </I18nextProvider>
    )
}

const preview: Preview = {
    decorators: [withI18n, withTheme, withMemoryRouter],
    // Stories may change the language; each one starts from the toolbar locale.
    beforeEach: async ({globals}) => {
        await i18n.changeLanguage(globals.locale ?? "en")
    },
    parameters: {
        a11y: {test: "error"},
        controls: {
            matchers: {
                color: /(background|color)$/i,
                date: /Date$/,
            },
        },
        viewport: {options: INITIAL_VIEWPORTS},
    },
    globalTypes: {
        locale: {
            description: "Internationalization locale",
            toolbar: {
                icon: "globe",
                items: [
                    {value: "en", title: "English"},
                    {value: "es", title: "Spanish"},
                ],
            },
        },
    },
    initialGlobals: {locale: "en"},
    tags: ["autodocs"],
}

export default preview
