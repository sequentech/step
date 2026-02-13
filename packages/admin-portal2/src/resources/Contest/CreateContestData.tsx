// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {CreateBase} from "react-admin"
import {ContestDataForm} from "./EditContestDataForm"

export const CreateContestData: React.FC = () => {
    const transform = (data: any) => {
        console.log("TRANSFORM ELECTION :: ", data)

        // save presentation object
        // language_conf
        console.log("data before :: ", data)
        const enabled_language_codes = []
        for (const key in data.enabled_languages) {
            if (typeof data.enabled_languages[key] === "boolean" && data.enabled_languages[key]) {
                enabled_language_codes.push(key)
            }
        }
        const language_conf = {
            enabled_language_codes: enabled_language_codes,
        }
        // i18n
        // is alll object, no change needed
        delete data.enabled_languages

        return {
            ...data,
            presentation: {
                ...data.presentation,
                language_conf: {
                    ...language_conf,
                    default_language_code: data.defaultLanguage,
                },
            },
        }
    }

    return (
        <CreateBase redirect={"show"} transform={transform}>
            <ContestDataForm />
        </CreateBase>
    )
}
