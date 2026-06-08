// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {useTranslation} from "react-i18next"
import {BorderBox, UnselectableTypography} from "../Candidate/Candidate"
import {theme} from "../../services/theme"

interface BlankAnswerProps {
    title?: string
}
const BlankAnswer: React.FC<BlankAnswerProps> = ({title}) => {
    const {t} = useTranslation()

    return (
        <BorderBox
            isSelectable={false}
            hasCategory={false}
            isInvalidVote={false}
            isDisabled={false}
            className="candidate-item blank-answer"
        >
            <UnselectableTypography
                className="candidate-title"
                fontWeight="bold"
                fontSize="16px"
                lineHeight="22px"
                marginTop="4px"
                marginBottom="4px"
                marginInlineStart="8px"
                color={theme.palette.customGrey.contrastText}
            >
                {title || t("candidate.blankVote")}
            </UnselectableTypography>
        </BorderBox>
    )
}

export default BlankAnswer
