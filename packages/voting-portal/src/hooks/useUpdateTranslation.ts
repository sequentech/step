// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect} from "react"
import {IElectionEventPresentation, overwriteTranslations} from "@sequentech/ui-core"

type props = {
    presentation: IElectionEventPresentation | undefined
}
const useUpdateTranslation = (
    {presentation}: props,
    defaultLanguageTouched: boolean,
    setDefaultLanguageTouched: (value: boolean) => void
) => {
    // Overwrites translations based on the election event presentation
    // Update Language based on presentation only if default language has not been touched,
    // So search param "lang" > user selected locale (saved in cookie) >
    // language detection policy (force default) > browser settings
    useEffect(() => {
        if (!presentation) {
            return
        }
        let hasSetDefaultLanguage = overwriteTranslations(presentation, !defaultLanguageTouched)
        if (hasSetDefaultLanguage) {
            setDefaultLanguageTouched(true)
        }
    }, [presentation])

    return {}
}

export default useUpdateTranslation
