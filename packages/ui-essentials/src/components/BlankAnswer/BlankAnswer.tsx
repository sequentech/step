// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {useTranslation} from "react-i18next"
import Candidate from "../Candidate/Candidate"

const BlankAnswer: React.FC = () => {
    const {t} = useTranslation()

    return (
        <Candidate
            title={t("candidate.blankVote")}
            isSelectable={false}
            checked={true}
            setChecked={() => undefined}
            hasCategory={false}
            shouldDisable={false}
        />
    )
}

export default BlankAnswer
