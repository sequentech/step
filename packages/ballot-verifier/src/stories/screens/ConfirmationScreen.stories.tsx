// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Meta, StoryObj} from "@storybook/react"
import Container from "@mui/material/Container"
import {ConfirmationScreen} from "../../screens/ConfirmationScreen"
import {LanguageSetter} from "@sequentech/ui-essentials"
import {withRouter} from "storybook-addon-react-router-v6"
import ConfirmationBallot from "../../fixtures/confirmation_ballot.json"
import {IConfirmationBallot} from "../../services/BallotService"

// === Component Props ===
type ConfirmationScreenProps = React.ComponentProps<typeof ConfirmationScreen>

// === Story Args (includes story-only controls) ===
interface StoryArgs extends ConfirmationScreenProps {
    language: "en" | "es"
}

// === Meta ===
const meta = {
    title: "screens/ConfirmationScreen",
    component: ConfirmationScreen,
    decorators: [withRouter],
    parameters: {
        backgrounds: {default: "white"},
        reactRouter: {routePath: "/confirmation", routeParams: {}},
    },
} satisfies Meta<typeof ConfirmationScreen>

export default meta

// === Template ===
const Template: React.FC<StoryArgs> = ({language, ...componentProps}) => {
    return (
        <Container style={{backgroundColor: "white"}}>
            <LanguageSetter language={language}>
                <ConfirmationScreen {...componentProps} />
            </LanguageSetter>
        </Container>
    )
}

// === Stories ===
type Story = StoryObj<StoryArgs>

export const Primary: Story = {
    render: (args) => <Template {...args} />,
    args: {
        language: "en",
        confirmationBallot: ConfirmationBallot as IConfirmationBallot,
        ballotId: "ballot-123",
    },
}

export const Loading: Story = {
    render: (args) => <Template {...args} />,
    args: {
        language: "en",
        confirmationBallot: null,
        ballotId: "ballot-123",
    },
}
