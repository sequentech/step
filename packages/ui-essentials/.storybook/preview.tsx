// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {Suspense, useEffect} from "react"
import {theme} from "../src/index"
import {ThemeProvider} from "@mui/material"
import {INITIAL_VIEWPORTS} from "@storybook/addon-viewport"
import {I18nextProvider} from "react-i18next"
import LanguageSetter from "../src/components/LanguageSetter/LanguageSetter"
import i18n, {initializeLanguages} from "../src/services/i18n"
import {withRouter} from "storybook-addon-remix-react-router"

initializeLanguages({})

const MuiDecorator = (Story: React.ComponentType) => (
    <ThemeProvider theme={theme}>
        <Story />
    </ThemeProvider>
)

const withI18next = (Story: React.FC, context: any) => {
    const {locale} = context.globals

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

export const parameters = {
    actions: {argTypesRegex: "^on[A-Z].*"},
    controls: {
        matchers: {
            color: /(background|color)$/i,
            date: /Date$/,
        },
    },
    reactRouter: {
        // Default global route for stories using <Link> or hooks
        location: {
            path: "/",
            pathParams: {},
            searchParams: new URLSearchParams(),
        },
        routing: {
            path: "/",
        },
    },
    viewport: {
        options: INITIAL_VIEWPORTS, // SB 8 uses 'options'
    },
}

// Create a global variable called locale in storybook and add a menu in the toolbar to change your locale
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

export const decorators = [withRouter, MuiDecorator, withI18next]
