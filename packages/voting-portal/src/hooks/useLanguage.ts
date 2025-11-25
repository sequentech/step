// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect} from "react"
import {useTranslation} from "react-i18next"
import {IBallotStyle} from "../store/ballotStyles/ballotStylesSlice"

type props = {
    ballotStyle: IBallotStyle | undefined
}
const useLanguage = ({ballotStyle}: props) => {
    const {i18n} = useTranslation()

    useEffect(() => {
        const currLanguage = i18n.language
        const electionLanguages =
            ballotStyle?.ballot_eml?.election_presentation?.language_conf?.enabled_language_codes
        const defaultLang =
            ballotStyle?.ballot_eml?.election_presentation?.language_conf?.default_language_code
        if (
            !electionLanguages ||
            !currLanguage ||
            electionLanguages.includes(currLanguage) ||
            !defaultLang
        )
            return
        i18n.changeLanguage(defaultLang)
    }, [])
}

export default useLanguage
