// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {CreateBase, EditBase, Identifier, RaRecord} from "react-admin"
import {CandidateDataForm, Sequent_Backend_Candidate_Extended} from "./CandidateDataForm"
import {Sequent_Backend_Candidate} from "@/gql/graphql"

export const CreateCandidateData: React.FC<{record: Sequent_Backend_Candidate}> = ({record}) => {
    const transform = (data: Sequent_Backend_Candidate_Extended): RaRecord<Identifier> => {
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
            <CandidateDataForm record={record} />
        </CreateBase>
    )
}
