// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Meta, StoryObj} from "@storybook/react"
import Tree from "../Tree"
import {INITIAL_VIEWPORTS} from "@storybook/addon-viewport"
import {faBank} from "@fortawesome/free-solid-svg-icons"
import Icon from "../../Icon/Icon"
import {styled} from "@mui/material/styles"

const StyledIcon = styled(Icon)`
    margin-right: 8px;
`

const meta: Meta<typeof Tree> = {
    title: "components/Tree",
    component: Tree,
    parameters: {
        backgrounds: {
            default: "white",
        },
        viewport: {
            viewports: INITIAL_VIEWPORTS,
            defaultViewport: "iphone6",
        },
    },
}

export default meta

type Story = StoryObj<typeof Tree>

export const SimpleTree: Story = {
    args: {
        leaves: [
            {
                label: "Parent",
                leaves: [
                    {
                        label: "Child 1",
                        leaves: [
                            {
                                label: "SubChild A",
                            },
                            {
                                label: "SubChild B",
                            },
                        ],
                    },
                    {
                        label: "Child 2",
                    },
                ],
            },
            {
                label: "Parent 2",
            },
        ],
    },
    parameters: {
        backgrounds: {
            default: "white",
        },
        viewport: {
            disable: true,
        },
    },
}

export const TreeComponents: Story = {
    args: {
        leaves: [
            {
                label: (
                    <>
                        <StyledIcon icon={faBank} />
                        <i>Parent</i>
                    </>
                ),
                leaves: [
                    {
                        label: "Child 1",
                        leaves: [
                            {
                                label: "SubChild A",
                            },
                            {
                                label: "SubChild B",
                            },
                        ],
                    },
                    {
                        label: "Child 2",
                    },
                ],
            },
        ],
    },
    parameters: {
        backgrounds: {
            default: "white",
        },
        viewport: {
            disable: true,
        },
    },
}
