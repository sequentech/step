// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {ComponentStory, ComponentMeta} from "@storybook/react"
import Container from "@mui/material/Container"
import {HomeScreen} from "../../screens/HomeScreen"
import {LanguageSetter} from "@sequentech/ui-essentials"
import {withRouter} from "storybook-addon-react-router-v6"
import {IBallotService, provideBallotService} from "../../services/BallotService"
import {within, userEvent} from "@storybook/testing-library"
import {expect} from "@storybook/jest"
import {IDecodedVoteContest, IAuditableBallot} from "@sequentech/ui-core"

export default {
    title: "screens/HomeScreen",
    component: HomeScreen,
    decorators: [withRouter],
    parameters: {
        backgrounds: {
            default: "white",
        },
        reactRouter: {
            routePath: "/",
            routeParams: {},
        },
    },
    argTypes: {
        language: {
            options: ["en", "es"],
            control: {type: "radio"},
        },
        setConfirmationBallot: {
            table: {
                disable: true,
            },
        },
        ballotService: {
            table: {
                disable: true,
            },
        },
    },
} as ComponentMeta<typeof HomeScreen>

const getBallotServiceProvider = (): IBallotService => {
    const service = provideBallotService()

    const decodeAuditableBallot = (
        auditableBallot: IAuditableBallot
    ): Array<IDecodedVoteContest> | null => null

    return {
        ...service,
        decodeAuditableBallot,
    }
}

interface TemplateProps {
    language: "en" | "es"
}
type HomeScreenProps = React.ComponentProps<typeof HomeScreen>

const Template: ComponentStory<React.FC<TemplateProps & HomeScreenProps>> = ({
    language,
    ballotService: _ballotService,
    ...args
}) => {
    const ballotService = getBallotServiceProvider()

    return (
        <Container style={{backgroundColor: "white"}}>
            <LanguageSetter language={language}>
                <HomeScreen ballotService={ballotService} {...args} />
            </LanguageSetter>
        </Container>
    )
}

export const Primary = Template.bind({})
// More on args: https://storybook.js.org/docs/react/writing-stories/args
Primary.args = {
    label: "HomeScreen",
    language: "en",
    setConfirmationBallot: () => {},
}

export const Error = Template.bind({})
// More on args: https://storybook.js.org/docs/react/writing-stories/args
Error.args = {
    label: "HomeScreen",
    language: "en",
    setConfirmationBallot: () => {},
}

Error.play = async ({canvasElement}) => {
    const canvas = within(canvasElement)

    const fakeFile = new File(["hello"], "hello.png", {type: "image/png"})

    const inputFile = canvas.getByTestId<HTMLInputElement>("drop-input-file")
    userEvent.upload(inputFile, fakeFile)

    expect(inputFile.files).toHaveLength(1)
    expect(inputFile.files![0]).toStrictEqual(fakeFile)
    expect(inputFile.files!.item(0)).toStrictEqual(fakeFile)
}
