// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import Box from "@mui/material/Box"
import {useNavigate} from "react-router-dom"
import {IBallotService, IConfirmationBallot} from "../services/BallotService"
import {BreadCrumbSteps, PageLimit} from "@sequentech/ui-essentials"
import {ActionButtons} from "../components/ActionButtons"
import {BallotIdSection, isMatchingBallotIds} from "../components/BallotIdSection"
import {VerifySelectionsSection} from "../components/VerifySelectionsSection"
import {IBallotStyle} from "@sequentech/ui-core"

interface IProps {
    ballotStyle: IBallotStyle | undefined
    confirmationBallot: IConfirmationBallot | null
    ballotService: IBallotService
    ballotId: string
    label?: string
}

export const ConfirmationScreen: React.FC<IProps> = ({
    ballotStyle,
    confirmationBallot,
    ballotService,
    ballotId,
}) => {
    const navigate = useNavigate()
    const [isLoading, setIsLoading] = useState(confirmationBallot === null)

    console.log("aa confirmationBallot", confirmationBallot)

    useEffect(() => {
        setIsLoading(confirmationBallot === null)
        if (confirmationBallot == null) {
            navigate("/")
        }
    }, [confirmationBallot])

    return (
        <PageLimit maxWidth="lg">
            <Box marginTop="48px" marginBottom="24px">
                <BreadCrumbSteps
                    labels={[
                        "breadcrumbSteps.import",
                        "breadcrumbSteps.verify",
                        //"breadcrumbSteps.finish",
                    ]}
                    selected={1}
                />
            </Box>

            <BallotIdSection confirmationBallot={confirmationBallot} ballotId={ballotId} />

            {isMatchingBallotIds(confirmationBallot?.ballot_hash, ballotId) ? (
                <VerifySelectionsSection
                    ballotStyle={ballotStyle}
                    confirmationBallot={confirmationBallot}
                    isLoading={isLoading}
                    ballotService={ballotService}
                />
            ) : null}

            <ActionButtons />
        </PageLimit>
    )
}
