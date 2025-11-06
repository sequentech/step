// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {EditBase, useGetOne} from "react-admin"
import {CandidateDataForm} from "./CandidateDataForm"
import {Sequent_Backend_Candidate, Sequent_Backend_Document} from "@/gql/graphql"

export const EditCandidateData: React.FC<{record: Sequent_Backend_Candidate}> = ({record}) => {
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

        // name, alias and description fields
        const fromPresentationName =
            data?.presentation?.i18n?.en?.name ||
            data?.presentation?.i18n[Object.keys(data.presentation.i18n)[0]].name ||
            ""
        data.name = fromPresentationName
        const fromPresentationAlias =
            data?.presentation?.i18n?.en?.alias ||
            data?.presentation?.i18n[Object.keys(data.presentation.i18n)[0]].alias ||
            ""
        data.alias = fromPresentationAlias
        const fromPresentationDescription =
            data?.presentation?.i18n?.en?.description ||
            data?.presentation?.i18n[Object.keys(data.presentation.i18n)[0]].description ||
            ""
        data.description = fromPresentationDescription
        // END name, alias and description fields

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
        <EditBase redirect={"."} transform={transform}>
            <CandidateDataForm record={record} />
        </EditBase>
    )
}
