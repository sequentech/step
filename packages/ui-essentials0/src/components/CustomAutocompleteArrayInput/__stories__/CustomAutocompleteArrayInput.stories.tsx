// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ComponentProps} from "react"
import {StoryFn, Meta} from "@storybook/react"
import CustomAutocompleteArrayInput from "../CustomAutocompleteArrayInput"
import {withRouter} from "storybook-addon-react-router-v6"

type StoryProps = ComponentProps<typeof CustomAutocompleteArrayInput>

const meta: Meta<StoryProps> = {
    title: "components/CustomAutocompleteArrayInput",
    component: CustomAutocompleteArrayInput,
    decorators: [withRouter],
}

export default meta

const Template: StoryFn<React.FC<StoryProps>> = ({...args}) => (
    <CustomAutocompleteArrayInput {...args} />
)

// More on args: https://storybook.js.org/docs/react/writing-stories/args
export const Default = Template.bind({})
Default.args = {
    label: "Name of the field",
    choices: [],
} as any

export const Disabled = Template.bind({})
Disabled.args = {
    label: "Name of the field",
    choices: [],
    disabled: true,
} as any

export const Choices = Template.bind({})
Choices.args = {
    label: "Name of the field",
    choices: [
        {id: "uno", name: "uno"},
        {id: "dos", name: "dos"},
        {id: "tres", name: "tres"},
    ],
    defaultValue: ["uno", "dos", "tres"],
} as any
