// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {SelectInput, required} from "react-admin"
import {useFormContext} from "react-hook-form"
import {useTranslation} from "react-i18next"
import {Typography} from "@mui/material"
import {EReceiptsPolicy, EVoterSigningPolicy, EVotingStatus} from "@sequentech/ui-core"
import type {IElectionEventPresentation, IElectionEventStatus} from "@sequentech/ui-core"

export const RECEIPTS_POLICY_SOURCE = "presentation.receipts.policy"
export const VOTER_SIGNING_POLICY_SOURCE = "presentation.voter_signing_policy"

export const areReceiptsSignedByBallotBox = (
    presentation?: Pick<IElectionEventPresentation, "receipts">
): boolean => presentation?.receipts?.policy === EReceiptsPolicy.SIGNED_BY_BALLOT_BOX

// Ballots already received or cast were made under the published setting.
export const hasVotingStarted = (status?: Partial<IElectionEventStatus> | null): boolean =>
    [
        status?.voting_status,
        status?.kiosk_voting_status,
        status?.early_voting_status,
        status?.telephone_voting_status,
    ].some((channel) => channel !== undefined && channel !== EVotingStatus.NOT_STARTED)

interface BallotReceiptsInputProps {
    canEdit: boolean
    status?: Partial<IElectionEventStatus> | null
}

/**
 * Whether the ballot box receives and signs ballots at review. The ballot box
 * receives only ballots their voter has signed, so turning it on also sets the
 * voter signing policy.
 */
export const BallotReceiptsInput: React.FC<BallotReceiptsInputProps> = ({canEdit, status}) => {
    const {t} = useTranslation()
    const {setValue} = useFormContext()
    const votingStarted = hasVotingStarted(status)

    return (
        <>
            <SelectInput
                source={RECEIPTS_POLICY_SOURCE}
                choices={Object.values(EReceiptsPolicy).map((value) => ({
                    id: value,
                    name: t(`electionEventScreen.field.receiptsPolicy.${value}`),
                }))}
                label={String(t("electionEventScreen.field.receiptsPolicy.policyLabel"))}
                defaultValue={EReceiptsPolicy.DISABLED}
                emptyText={undefined}
                validate={required()}
                disabled={!canEdit || votingStarted}
                onChange={(event) => {
                    if (event.target.value === EReceiptsPolicy.SIGNED_BY_BALLOT_BOX) {
                        setValue(VOTER_SIGNING_POLICY_SOURCE, EVoterSigningPolicy.WITH_SIGNATURE, {
                            shouldDirty: true,
                        })
                    }
                }}
            />
            <Typography variant="caption" color="text.secondary" sx={{display: "block"}}>
                {t(
                    votingStarted
                        ? "electionEventScreen.field.receiptsPolicy.lockedHelperText"
                        : "electionEventScreen.field.receiptsPolicy.helperText"
                )}
            </Typography>
        </>
    )
}
