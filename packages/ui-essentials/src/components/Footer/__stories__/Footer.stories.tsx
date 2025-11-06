// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {ComponentStory, ComponentMeta} from "@storybook/react"
import Footer from "../Footer"
import {withRouter} from "storybook-addon-react-router-v6"
import {theme} from "../../../services/theme"

import i18n from "i18next"
import {I18nextProvider, initReactI18next} from "react-i18next"

export default {
    title: "components/Footer",
    component: Footer,
    decorators: [withRouter],
    parameters: {
        backgrounds: {
            default: "white",
        },
        reactRouter: {
            routePath: "/footer",
            routeParams: {},
        },
    },
    argTypes: {
        children: {
            table: {
                disable: true,
            },
        },
        ref: {
            table: {
                disable: true,
            },
        },
        sx: {
            table: {
                disable: true,
            },
        },
    },
} as ComponentMeta<typeof Footer>

interface TemplateProps {
    backgroundColor?: string
}
type FooterProps = React.ComponentProps<typeof Footer>

const Template: ComponentStory<React.FC<FooterProps & TemplateProps>> = ({
    backgroundColor,
    ...args
}) => <Footer {...args} style={{backgroundColor: backgroundColor}} />

export const Primary = Template.bind({})
// More on args: https://storybook.js.org/docs/react/writing-stories/args
Primary.args = {
    label: "Footer",
} as any

const i18nWithInvalidTranslation = i18n.createInstance()
void i18nWithInvalidTranslation.use(initReactI18next).init({
    lng: "en",
    fallbackLng: "en",
    ns: ["common"],
    defaultNS: "common",
    resources: {
        en: {
            common: {
                footer: {
                    poweredBy: "Powered by Sequent",
                },
            },
        },
    },
})

export const InvalidTranslation: ComponentStory<
    React.FC<FooterProps & TemplateProps & {poweredBy: string}>
> = ({poweredBy, ...args}) => {
    const i18nWithInvalidTranslation = i18n.createInstance()
    void i18nWithInvalidTranslation.use(initReactI18next).init({
        lng: "en",
        fallbackLng: "en",
        ns: ["common"],
        defaultNS: "common",
        resources: {
            en: {
                common: {
                    footer: {
                        poweredBy,
                    },
                },
            },
        },
    })
    return (
        <I18nextProvider i18n={i18nWithInvalidTranslation}>
            <Footer {...args} />
        </I18nextProvider>
    )
}
InvalidTranslation.args = {
    label: "Footer",
    poweredBy: "Powered by Sequent",
} as any
InvalidTranslation.argTypes = {
    poweredBy: {
        control: "text",
    },
}
