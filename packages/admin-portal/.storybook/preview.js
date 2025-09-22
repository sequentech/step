// SPDX-FileCopyrightText: 2022-2024 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {theme, initializeLanguages} from "@sequentech/ui-core"
import {ThemeProvider} from "@mui/material"
import {INITIAL_VIEWPORTS} from "@storybook/addon-viewport"
import englishTranslation from "../src/translations/en"
import spanishTranslation from "../src/translations/es"

initializeLanguages({
    en: englishTranslation,
    es: spanishTranslation,
})

export const parameters = {
    actions: {argTypesRegex: "^on[A-Z].*"},
    controls: {
        matchers: {
            color: /(background|color)$/i,
            date: /Date$/,
        },
    },
    viewport: {
        viewports: INITIAL_VIEWPORTS,
    },
}

const MuiDecorator = (Story) => (
    <ThemeProvider theme={theme}>
        <Story />
    </ThemeProvider>
)

export const decorators = [MuiDecorator]
