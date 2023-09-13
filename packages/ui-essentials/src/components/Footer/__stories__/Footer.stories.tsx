// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {ComponentStory, ComponentMeta} from "@storybook/react"
import Footer from "../Footer"
import {withRouter} from "storybook-addon-react-router-v6"
import {theme} from "../../../services/theme"

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
