// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The tag next to a Tracking ID that says whether the ballot is cast, not
// cast, or audited (design board BallotId).

import React from "react"
import type {SxProps, Theme} from "@mui/material/styles"
import {useTranslation} from "react-i18next"
import {EBallotTag} from "../../services/vitrine/ballotStatus"
import {EStatusTone, StatusTag} from "./StatusTag"

const TONES: Record<EBallotTag, EStatusTone> = {
    [EBallotTag.NOT_CAST]: EStatusTone.NEUTRAL,
    [EBallotTag.CAST]: EStatusTone.SUCCESS,
    [EBallotTag.AUDITABLE]: EStatusTone.SUCCESS,
}

export interface IBallotTagProps {
    tag: EBallotTag
    sx?: SxProps<Theme>
}

export const BallotTag: React.FC<IBallotTagProps> = ({tag, sx}) => {
    const {t} = useTranslation()
    return <StatusTag label={t(`vitrine.status.tag.${tag}`)} tone={TONES[tag]} sx={sx} />
}
