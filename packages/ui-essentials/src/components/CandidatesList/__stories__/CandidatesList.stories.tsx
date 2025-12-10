// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Meta, StoryObj} from "@storybook/react"
import CandidatesList, {CandidatesListProps} from "../CandidatesList"
import {INITIAL_VIEWPORTS} from "@storybook/addon-viewport"
import {Box} from "@mui/material"
import Image from "mui-image"
import CandidateImg from "../../../../public/example_candidate.jpg"
import Candidate from "../../Candidate/Candidate"

interface SimpleCandidateProps {
    isActive?: boolean
}

const SimpleCandidate: React.FC<SimpleCandidateProps> = ({isActive}) => (
    <Candidate
        title="Micky Mouse"
        description="Candidate Description"
        isSelectable={isActive}
        hasCategory={true}
        url="https://google.com"
        shouldDisable={false}
    >
        <Image src={CandidateImg} duration={100} />
    </Candidate>
)

const CandidatesListWrapper: React.FC<CandidatesListProps & {className?: string}> = ({
    className,
    isActive,
    ...props
}) => (
    <Box className={className}>
        <CandidatesList isActive={isActive} {...props}>
            <SimpleCandidate isActive={isActive} />
            <SimpleCandidate isActive={isActive} />
        </CandidatesList>
    </Box>
)

const meta: Meta<typeof CandidatesListWrapper> = {
    title: "components/CandidatesList",
    component: CandidatesListWrapper,
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

type Story = StoryObj<typeof CandidatesListWrapper>

export const Primary: Story = {
    args: {
        title: "Category A",
        isActive: true,
        isCheckable: true,
    },
    parameters: {
        viewport: {
            disable: true,
        },
    },
}

export const NotCheckable: Story = {
    args: {
        title: "Category A",
        isActive: true,
    },
    parameters: {
        viewport: {
            disable: true,
        },
    },
}

export const NotActive: Story = {
    args: {
        title: "Category A",
        isActive: false,
    },
    parameters: {
        viewport: {
            disable: true,
        },
    },
}
