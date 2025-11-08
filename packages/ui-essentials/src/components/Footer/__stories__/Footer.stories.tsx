// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {StoryFn, Meta} from "@storybook/react"
import Footer from "../Footer"
import {withRouter} from "storybook-addon-react-router-v6"
import {theme} from "../../../services/theme"

// React 19 compatibility wrapper for I18nextProvider
const I18nextProviderFixed: React.FC<any> = (props) => {
    const Provider = I18nextProvider as any;
    return <Provider {...props} />;
};

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
} as Meta<typeof Footer>

interface TemplateProps {
    backgroundColor?: string
}
type FooterProps = React.ComponentProps<typeof Footer>

const Template: StoryFn<React.FC<FooterProps & TemplateProps>> = ({
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

export const InvalidTranslation: StoryFn<
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
        <I18nextProviderFixed i18n={i18nWithInvalidTranslation}>
            <Footer {...args} />
        </I18nextProviderFixed>
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
