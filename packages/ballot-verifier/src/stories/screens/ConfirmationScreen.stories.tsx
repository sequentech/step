// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState, useEffect} from "react"
import {Meta, StoryObj} from "@storybook/react"
import Container from "@mui/material/Container"
import {ConfirmationScreen} from "../../screens/ConfirmationScreen"
import {LanguageSetter} from "@sequentech/ui-essentials"
import {withRouter} from "storybook-addon-react-router-v6"
import ConfirmationBallot from "../../fixtures/confirmation_ballot.json"
import {
    IBallotService,
    IConfirmationBallot,
    provideBallotService,
} from "../../services/BallotService"
import {IDecodedVoteChoice, IContest, IContestLayoutProperties} from "@sequentech/ui-core"

// === Component Props ===
type ConfirmationScreenProps = React.ComponentProps<typeof ConfirmationScreen>

// === Story Args (includes story-only controls) ===
interface StoryArgs extends ConfirmationScreenProps {
    language: "en" | "es"
    ordered: boolean
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
    // Only define argTypes for *actual component props*
    // `language` and `ordered` are NOT props → don't include in argTypes
    argTypes: {
        // Hide internal props
        ballotService: {table: {disable: true}},
        // Allow controlling real props if needed
        // confirmationBallot: { control: false },
    },
} satisfies Meta<typeof ConfirmationScreen>

export default meta

// === Service Provider ===
const getBallotServiceProvider = (ordered: boolean): IBallotService => {
    const service = provideBallotService()

    const getPoints = (question: IContest, answer: IDecodedVoteChoice) => 4

    const getLayoutProperties = (question: IContest): IContestLayoutProperties | null => ({
        state: "state",
        sorted: false,
        ordered,
    })

    return {...service, getPoints, getLayoutProperties}
}

// === Template ===
const Template: React.FC<StoryArgs> = ({
    language,
    ordered,
    ballotService: _,
    ...componentProps
}) => {
    const [service, setService] = useState(getBallotServiceProvider(ordered))

    useEffect(() => {
        setService(getBallotServiceProvider(ordered))
    }, [ordered])

    return (
        <Container style={{backgroundColor: "white"}}>
            <LanguageSetter language={language}>
                <ConfirmationScreen ballotService={service} {...componentProps} />
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
        ordered: false,
        confirmationBallot: ConfirmationBallot as IConfirmationBallot,
        ballotId: "ballot-123", // adjust as needed
        // label: "ConfirmationScreen", // if optional
    },
}

export const Loading: Story = {
    render: (args) => <Template {...args} />,
    args: {
        language: "en",
        ordered: false,
        confirmationBallot: null,
        ballotId: "ballot-123",
    },
}
