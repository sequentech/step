// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Meta, StoryObj} from "@storybook/react"
import Candidate, {CandidateProps} from "../Candidate"
import {INITIAL_VIEWPORTS} from "@storybook/addon-viewport"
import {Box} from "@mui/material"
import Image from "mui-image"

import CandidateImg from "../../../../public/example_candidate.jpg"

const CandidateWrapper: React.FC<CandidateProps & {className?: string}> = ({
    className,
    ...props
}) => (
    <Box className={className}>
        <Candidate {...props} />
    </Box>
)

const meta: Meta<typeof CandidateWrapper> = {
    title: "components/Candidate",
    component: CandidateWrapper,
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

type Story = StoryObj<typeof CandidateWrapper>

export const Primary: Story = {
    args: {
        children: <Image src={CandidateImg} duration={100} />,
        title: "Micky Mouse",
        description: "Candidate Description",
        isSelectable: true,
        checked: true,
        url: "https://google.com",
    },
    parameters: {
        viewport: {
            disable: true,
        },
    },
}

export const ReadOnly: Story = {
    args: {
        children: <Image src={CandidateImg} duration={100} />,
        title: "Micky Mouse",
        description: "Candidate Description",
        isSelectable: false,
        checked: true,
        url: "https://google.com",
    },
    parameters: {
        viewport: {
            disable: true,
        },
    },
}

export const NoImage: Story = {
    args: {
        title: "Micky Mouse",
        description: "Candidate Description",
        isSelectable: true,
        url: "https://google.com",
    },
    parameters: {
        viewport: {
            disable: true,
        },
    },
}

export const NoDescription: Story = {
    args: {
        children: <Image src={CandidateImg} duration={100} />,
        title: "Micky Mouse",
        isSelectable: true,
        url: "https://google.com",
        checked: false,
    },
    parameters: {
        viewport: {
            disable: true,
        },
    },
}

export const OnlyTitle: Story = {
    args: {
        title: "Micky Mouse",
        isSelectable: true,
        checked: false,
    },
    parameters: {
        viewport: {
            disable: true,
        },
    },
}

export const LongDescription: Story = {
    args: {
        children: <Image src={CandidateImg} duration={100} />,
        title: "Micky Mouse",
        description:
            "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.",
    },
    parameters: {
        viewport: {
            disable: true,
        },
    },
}

export const LongTitle: Story = {
    args: {
        children: <Image src={CandidateImg} duration={100} />,
        title: "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.",
        description: "Candidate Description",
        isSelectable: true,
        url: "https://google.com",
    },
    parameters: {
        viewport: {
            disable: true,
        },
    },
}

export const WithHtml: Story = {
    args: {
        children: <Image src={CandidateImg} duration={100} />,
        title: (
            <>
                Micky <b>Mouse</b>
            </>
        ),
        description: (
            <>
                Candidate <b>description</b>
            </>
        ),
        isSelectable: true,
        url: "https://google.com",
    },
    parameters: {
        viewport: {
            disable: true,
        },
    },
}

export const Hover: Story = {
    args: {
        children: <Image src={CandidateImg} duration={100} />,
        title: "Micky Mouse",
        description: "Candidate Description",
        className: "hover",
        isSelectable: true,
        url: "https://google.com",
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

export const Active: Story = {
    args: {
        children: <Image src={CandidateImg} duration={100} />,
        title: "Micky Mouse",
        description: "Candidate Description",
        className: "hover active",
        isSelectable: true,
        url: "https://google.com",
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

export const WriteInSimple: Story = {
    args: {
        title: "",
        description: "",
        isSelectable: true,
        isWriteIn: true,
    },
    parameters: {
        viewport: {
            disable: true,
        },
    },
}

export const WriteInInvalid: Story = {
    args: {
        title: "",
        description: "",
        isSelectable: true,
        isWriteIn: true,
        writeInValue: "John Connor",
        isInvalidWriteIn: true,
    },
    parameters: {
        viewport: {
            disable: true,
        },
    },
}

export const WriteInFields: Story = {
    args: {
        title: "",
        description: "",
        isSelectable: true,
        isWriteIn: true,
    },
    parameters: {
        viewport: {
            disable: true,
        },
    },
}

export const InvalidVote: Story = {
    args: {
        title: "Micky Mouse",
        isSelectable: true,
        isInvalidVote: true,
        checked: false,
    },
    parameters: {
        viewport: {
            disable: true,
        },
    },
}
