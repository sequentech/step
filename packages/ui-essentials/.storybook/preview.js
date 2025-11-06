// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {theme} from "../src/index"
import {ThemeProvider} from "@mui/material"
import {INITIAL_VIEWPORTS} from "@storybook/addon-viewport"
import React, {Suspense, useEffect} from "react"
import {I18nextProvider} from "react-i18next"
import LanguageSetter from "../src/components/LanguageSetter/LanguageSetter"
import i18n, {initializeLanguages} from "../src/services/i18n"

initializeLanguages({})

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

// Create a global variable called locale in storybook
// and add a menu in the toolbar to change your locale
export const globalTypes = {
    locale: {
        name: "Locale",
        description: "Internationalization locale",
        toolbar: {
            icon: "globe",
            items: [
                {value: "en", title: "English"},
                {value: "es", title: "Spanish"},
            ],
            showName: true,
        },
    },
}

const MuiDecorator = (Story) => (
    <ThemeProvider theme={theme}>
        <Story />
    </ThemeProvider>
)

// https://storybook.js.org/recipes/react-i18next

// Wrap your stories in the I18nextProvider component
const withI18next = (Story, context) => {
    const {locale} = context.globals

    // When the locale global changes
    // Set the new locale in i18n
    useEffect(() => {
        console.log(`new locale ${locale}`)
        i18n.changeLanguage(locale)
    }, [locale])

    return (
        <Suspense fallback={<div>loading translations...</div>}>
            <I18nextProvider i18n={i18n}>
                <LanguageSetter language={locale}>
                    <Story />
                </LanguageSetter>
            </I18nextProvider>
        </Suspense>
    )
}

export const decorators = [MuiDecorator, withI18next]
