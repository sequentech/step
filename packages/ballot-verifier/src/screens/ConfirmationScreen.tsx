// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import Box from "@mui/material/Box"
import {useNavigate} from "react-router-dom"
import {IConfirmationBallot} from "../services/BallotService"
import {BreadCrumbSteps, PageLimit} from "@sequentech/ui-essentials"
import {ActionButtons} from "../components/ActionButtons"
import {BallotIdSection, isMatchingBallotIds} from "../components/BallotIdSection"
import {VerifySelectionsSection} from "../components/VerifySelectionsSection"

import {IBallotStyle as IElectionDTO} from "@sequentech/ui-core"

export interface IBallotStyle {
    id: string
    election_id: string
    election_event_id: string
    tenant_id: string
    ballot_eml: IElectionDTO
    ballot_signature?: string | null
    created_at: string
    area_id?: string | null
    annotations?: string | null
    labels?: string | null
    last_updated_at: string
}

interface IProps {
    ballotStyle: IBallotStyle | undefined
    confirmationBallot: IConfirmationBallot | null
    ballotId: string
    label?: string
}

export const ConfirmationScreen: React.FC<IProps> = ({
    ballotStyle,
    confirmationBallot,
    ballotId,
}) => {
    const navigate = useNavigate()
    const [isLoading, setIsLoading] = useState(confirmationBallot === null)

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

            {ballotStyle && isMatchingBallotIds(confirmationBallot?.ballot_hash, ballotId) ? (
                <VerifySelectionsSection
                    ballotStyle={ballotStyle}
                    confirmationBallot={confirmationBallot}
                    isLoading={isLoading}
                />
            ) : null}

            <ActionButtons />
        </PageLimit>
    )
}
