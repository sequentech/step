// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
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

        if (!defaultLang) return

        // If current language differs from election default, switch to default.
        // Only proceed if default is among the enabled languages (when provided).
        const defaultIsEnabled = !electionLanguages || electionLanguages.includes(defaultLang)
        if (defaultIsEnabled && currLanguage && currLanguage !== defaultLang) {
            i18n.changeLanguage(defaultLang)
        }
    }, [])
}

export default useLanguage
