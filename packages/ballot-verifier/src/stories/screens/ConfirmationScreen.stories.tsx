// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState, useEffect} from "react"
import {ComponentStory, ComponentMeta} from "@storybook/react"
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

export default {
    title: "screens/ConfirmationScreen",
    component: ConfirmationScreen,
    decorators: [withRouter],
    parameters: {
        backgrounds: {
            default: "white",
        },
        reactRouter: {
            routePath: "/confirmation",
            routeParams: {},
        },
    },
    argTypes: {
        language: {
            options: ["en", "es"],
            control: {type: "radio"},
        },
        ordered: {
            options: [true, false],
        },
        ballotService: {
            table: {
                disable: true,
            },
        },
    },
} as ComponentMeta<typeof ConfirmationScreen>

const getBallotServiceProvider = (ordered: boolean): IBallotService => {
    const service = provideBallotService()

    const getPoints = (question: IContest, answer: IDecodedVoteChoice) => 4

    const getLayoutProperties = (question: IContest): IContestLayoutProperties | null => ({
        state: "state",
        sorted: false,
        ordered: ordered,
    })

    return {
        ...service,
        getPoints,
        getLayoutProperties,
    }
}

interface TemplateProps {
    language: "en" | "es"
    ordered: boolean
}

type ConfirmationScreenProps = React.ComponentProps<typeof ConfirmationScreen>

const Template: ComponentStory<React.FC<TemplateProps & ConfirmationScreenProps>> = ({
    language,
    ordered,
    ...args
}) => {
    const [service, setService] = useState(getBallotServiceProvider(ordered))
    useEffect(() => {
        setService(getBallotServiceProvider(ordered))
    }, [ordered])

    return (
        <Container style={{backgroundColor: "white"}}>
            <LanguageSetter language={language}>
                <ConfirmationScreen {...args} />
            </LanguageSetter>
        </Container>
    )
}

export const Primary = Template.bind({})

Primary.args = {
    label: "ConfirmationScreen",
    language: "en",
    ordered: false,
    confirmationBallot: ConfirmationBallot as IConfirmationBallot,
}

export const Loading = Template.bind({})

Loading.args = {
    label: "ConfirmationScreen",
    language: "en",
    ordered: false,
    confirmationBallot: null,
}
