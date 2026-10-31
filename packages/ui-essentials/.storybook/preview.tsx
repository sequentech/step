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
import {
    DEFAULT_STORY_GLOBALS,
    readStoryGlobals,
    storyGlobalTypes,
    withStoryGlobals,
} from "./globals"

// An explicit language keeps the browser language detector out of the stories.
initializeLanguages({}, DEFAULT_STORY_GLOBALS.locale)

const withTheme: Decorator = (Story) => (
    <ThemeProvider theme={theme}>
        <Story />
    </ThemeProvider>
)

const withI18n: Decorator = (Story, {globals}) => {
    const {locale} = readStoryGlobals(globals)

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
    // The router is innermost so that a route layout also renders inside the
    // theme, the translations and the story globals.
    decorators: [withMemoryRouter, withI18n, withTheme, withStoryGlobals],
    // Stories may change the language; each one starts from the toolbar locale.
    beforeEach: async ({globals}) => {
        await i18n.changeLanguage(readStoryGlobals(globals).locale)
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
    globalTypes: {locale: storyGlobalTypes.locale},
    initialGlobals: {...DEFAULT_STORY_GLOBALS},
    tags: ["autodocs"],
}

export default preview
