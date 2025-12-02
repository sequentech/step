// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Meta, StoryObj} from "@storybook/react"
import SelectElection, {SelectElectionProps} from "../SelectElection"
import {INITIAL_VIEWPORTS} from "@storybook/addon-viewport"
import {Box} from "@mui/material"

const SelectElectionWrapper: React.FC<SelectElectionProps & {className?: string}> = ({
    className,
    ...props
}) => (
    <Box className={className}>
        <SelectElection {...props} />
    </Box>
)

const meta: Meta<typeof SelectElectionWrapper> = {
    title: "components/SelectElection",
    component: SelectElectionWrapper,
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

type Story = StoryObj<typeof SelectElectionWrapper>

export const OpenVoted: Story = {
    args: {
        isActive: true,
        isOpen: true,
        title: "Executive Board",
        electionHomeUrl: "/election/34570007/public/home",
        hasVoted: true,
        electionDates: {
            first_started_at: "2025-10-29T14:00:00.000Z",
        },
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

export const OnHover: Story = {
    args: {
        isActive: true,
        isOpen: true,
        title: "Executive Board",
        electionHomeUrl: "/election/34570007/public/home",
        hasVoted: true,
        className: "hover",
        electionDates: {
            first_started_at: "2025-10-29T14:00:00.000Z",
        },
    },
    parameters: {
        pseudo: {
            hover: [".hover"],
            active: [".active"],
            focus: [".focus"],
        },
        viewport: {
            disable: true,
        },
    },
}

export const OnActive: Story = {
    args: {
        isActive: true,
        isOpen: true,
        title: "Executive Board",
        electionHomeUrl: "/election/34570007/public/home",
        hasVoted: true,
        className: "active",
        electionDates: {
            first_started_at: "2025-10-29T14:00:00.000Z",
        },
    },
    parameters: {
        pseudo: {
            hover: [".hover"],
            active: [".active"],
            focus: [".focus"],
        },
        viewport: {
            disable: true,
        },
    },
}

export const OnFocus: Story = {
    args: {
        isActive: true,
        isOpen: true,
        title: "Executive Board",
        electionHomeUrl: "/election/34570007/public/home",
        hasVoted: true,
        className: "focus",
        electionDates: {
            first_started_at: "2025-10-29T14:00:00.000Z",
        },
    },
    parameters: {
        pseudo: {
            hover: [".hover"],
            active: [".active"],
            focus: [".focus"],
        },
        viewport: {
            disable: true,
        },
    },
}

export const ClosedNotVoted: Story = {
    args: {
        isActive: false,
        isOpen: false,
        title: "Executive Board",
        electionHomeUrl: "/election/34570007/public/home",
        hasVoted: false,
        electionDates: {
            first_started_at: "2025-10-29T14:00:00.000Z",
        },
    },
    parameters: {
        viewport: {
            disable: true,
        },
    },
}

export const DisplayBallotLocator: Story = {
    args: {
        isActive: true,
        isOpen: true,
        title: "Executive Board",
        electionHomeUrl: "/election/34570007/public/home",
        hasVoted: false,
        onClickBallotLocator() {
            console.log("Clicked to locate the ballot")
        },
        electionDates: {
            first_started_at: "2025-10-29T14:00:00.000Z",
        },
    },
    parameters: {
        viewport: {
            disable: true,
        },
    },
}
