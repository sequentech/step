// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Meta, StoryObj} from "@storybook/react"
import Container from "@mui/material/Container"
import {HomeScreen} from "../../screens/HomeScreen"
import {LanguageSetter} from "@sequentech/ui-essentials"
import {withRouter} from "storybook-addon-react-router-v6"
import {IBallotService, provideBallotService} from "../../services/BallotService"
import {within, userEvent} from "@storybook/testing-library"
import {expect} from "@storybook/jest"
import {IDecodedVoteContest, IAuditableBallot} from "@sequentech/ui-core"

// Remove this line — PlayFunction doesn't exist
// import type { PlayFunction } from "@storybook/testing-library"

/* -------------------------------------------------
   Types
   ------------------------------------------------- */
type HomeScreenProps = React.ComponentProps<typeof HomeScreen>

interface StoryArgs extends HomeScreenProps {
    language: "en" | "es"
}

/* -------------------------------------------------
   Meta
   ------------------------------------------------- */
const meta = {
    title: "screens/HomeScreen",
    component: HomeScreen,
    decorators: [withRouter],
    parameters: {
        backgrounds: {default: "white"},
        reactRouter: {routePath: "/", routeParams: {}},
    },
    argTypes: {
        setConfirmationBallot: {table: {disable: true}},
        ballotService: {table: {disable: true}},
    },
} satisfies Meta<typeof HomeScreen>

export default meta

/* -------------------------------------------------
   Service provider
   ------------------------------------------------- */
const getBallotServiceProvider = (): IBallotService => {
    const service = provideBallotService()

    const decodeAuditableBallot = (
        _auditableBallot: IAuditableBallot
    ): Array<IDecodedVoteContest> | null => null

    return {...service, decodeAuditableBallot}
}

/* -------------------------------------------------
   Template
   ------------------------------------------------- */
const Template: React.FC<StoryArgs> = ({language, ballotService: _ignore, ...componentProps}) => {
    const ballotService = getBallotServiceProvider()

    return (
        <Container style={{backgroundColor: "white"}}>
            <LanguageSetter language={language}>
                <HomeScreen ballotService={ballotService} {...componentProps} />
            </LanguageSetter>
        </Container>
    )
}

/* -------------------------------------------------
   Stories
   ------------------------------------------------- */
type Story = StoryObj<StoryArgs>

export const Primary: Story = {
    render: (args) => <Template {...args} />,
    args: {
        language: "en",
        confirmationBallot: null,
        setConfirmationBallot: () => {},
        ballotId: "",
        setBallotId: () => {},
        // Add other required props here
    },
}

/* -------------------------------------------------
   Error Interaction Test
   ------------------------------------------------- */
export const Error: Story = {
    render: (args) => <Template {...args} />,
    args: {
        language: "en",
        confirmationBallot: null,
        setConfirmationBallot: () => {},
        ballotId: "",
        setBallotId: () => {},
    },

    // Use correct type: StoryContext from @storybook/react
    play: async ({canvasElement}: {canvasElement: HTMLElement}) => {
        const canvas = within(canvasElement)

        const fakeFile = new File(["hello"], "hello.png", {type: "image/png"})

        const inputFile = canvas.getByTestId<HTMLInputElement>("drop-input-file")
        await userEvent.upload(inputFile, fakeFile)

        expect(inputFile.files).toHaveLength(1)
        expect(inputFile.files![0]).toStrictEqual(fakeFile)
        expect(inputFile.files!.item(0)).toStrictEqual(fakeFile)
    },
}
