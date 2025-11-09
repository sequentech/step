// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Meta, StoryObj} from "@storybook/react"
import Candidate, {CandidateProps} from "../Candidate"
import {INITIAL_VIEWPORTS} from "@storybook/addon-viewport"
import {Box} from "@mui/material"
import CandidateImg from "../../../../public/example_candidate.jpg"

const Image: React.FC<{src: string | undefined}> = ({src}) => (
    <Box sx={{width: "100%", height: "100%", overflow: "hidden"}}>
        <img
            src={src}
            alt=""
            style={{
                width: "100%",
                height: "100%",
                objectFit: "cover",
                transition: "opacity 100ms ease",
            }}
        />
    </Box>
)

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
        children: <Image src={CandidateImg} />,
        title: "Micky Mouse",
        description: "Candidate Description",
        isActive: true,
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
        children: <Image src={CandidateImg} />,
        title: "Micky Mouse",
        description: "Candidate Description",
        isActive: false,
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
        isActive: true,
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
        children: <Image src={CandidateImg} />,
        title: "Micky Mouse",
        isActive: true,
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
        isActive: true,
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
        children: <Image src={CandidateImg} />,
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
        children: <Image src={CandidateImg} />,
        title:
            "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.",
        description: "Candidate Description",
        isActive: true,
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
        children: <Image src={CandidateImg} />,
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
        isActive: true,
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
        children: <Image src={CandidateImg} />,
        title: "Micky Mouse",
        description: "Candidate Description",
        className: "hover",
        isActive: true,
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
        children: <Image src={CandidateImg} />,
        title: "Micky Mouse",
        description: "Candidate Description",
        className: "hover active",
        isActive: true,
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
        isActive: true,
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
        isActive: true,
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
        isActive: true,
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
        isActive: true,
        isInvalidVote: true,
        checked: false,
    },
    parameters: {
        viewport: {
            disable: true,
        },
    },
}
