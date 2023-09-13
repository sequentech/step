// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Meta, StoryObj} from "@storybook/react"
import Button, {ButtonProps} from "@mui/material/Button"
import {FontAwesomeIcon} from "@fortawesome/react-fontawesome"
import {faPrint} from "@fortawesome/free-solid-svg-icons"
import VerticalBox from "../../../components/VerticalBox/VerticalBox"

const Template: React.FC<ButtonProps> = (args) => (
    <VerticalBox maxWidth="210px">
        <Button className="normal" {...args}>
            <FontAwesomeIcon icon={faPrint} size="sm" />
            <span>Label</span>
        </Button>
        <Button disabled {...args}>
            <FontAwesomeIcon icon={faPrint} size="sm" />
            <span>Label</span>
        </Button>
        <Button className="hover" {...args}>
            <FontAwesomeIcon icon={faPrint} size="sm" />
            <span>Label</span>
        </Button>
        <Button className="active" {...args}>
            <FontAwesomeIcon icon={faPrint} size="sm" />
            <span>Label</span>
        </Button>
        <Button className="focus" {...args}>
            <FontAwesomeIcon icon={faPrint} size="sm" />
            <span>Label</span>
        </Button>
    </VerticalBox>
)

const meta: Meta<typeof Button> = {
    title: "material/Button",
    component: Template,
    parameters: {
        backgrounds: {
            default: "white",
        },
    },
    argTypes: {},
}

export default meta

type Story = StoryObj<typeof Button>

export const SecondaryButton: Story = {
    parameters: {
        pseudo: {
            hover: [".hover"],
            active: [".active"],
            focus: [".focus"],
        },
    },
    args: {
        variant: "secondary",
    },
}

export const DefaultButton: Story = {
    parameters: {
        pseudo: {
            hover: [".hover"],
            active: [".active"],
            focus: [".focus"],
        },
    },
    args: {},
}

export const ActionButton: Story = {
    parameters: {
        pseudo: {
            hover: [".hover"],
            active: [".active"],
            focus: [".focus"],
        },
    },
    args: {
        variant: "action",
    },
}

export const CancelButton: Story = {
    parameters: {
        pseudo: {
            hover: [".hover"],
            active: [".active"],
            focus: [".focus"],
        },
    },
    args: {
        variant: "cancel",
    },
}

export const WarningButton: Story = {
    parameters: {
        pseudo: {
            hover: [".hover"],
            active: [".active"],
            focus: [".focus"],
        },
    },
    args: {
        variant: "warning",
    },
}

export const SolidWarningButton: Story = {
    parameters: {
        pseudo: {
            hover: [".hover"],
            active: [".active"],
            focus: [".focus"],
        },
    },
    args: {
        variant: "solidWarning",
    },
}

export const ActionbarButton: Story = {
    parameters: {
        pseudo: {
            hover: [".hover"],
            active: [".active"],
            focus: [".focus"],
        },
    },
    args: {
        variant: "actionbar",
    },
}
