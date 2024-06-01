// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {EditBase, Identifier, RaRecord, useUpdate} from "react-admin"
import {ElectionDataForm} from "./ElectionDataForm"
import {Sequent_Backend_Election_Extended} from "./ElectionDataForm"
import {ICommunicationMethod, IRECEIPTS} from "@/types/communications"
import {Sequent_Backend_Contest} from "@/gql/graphql"
import {ContestsOrder, IContestPresentation, IElectionPresentation} from "@sequentech/ui-essentials"

export const EditElectionData: React.FC = () => {
    const [update] = useUpdate<Sequent_Backend_Contest>()

    function updateContestsOrder(data: Sequent_Backend_Election_Extended) {
        data.contestsOrder?.map((contest: Sequent_Backend_Contest, index: number) => {
            let electionPresentation = data.presentation as IElectionPresentation | undefined
            if (electionPresentation?.contests_order === ContestsOrder.CUSTOM) {
                let contestPresentation = (contest.presentation ?? {}) as IContestPresentation
                contestPresentation.sort_order = index
                return update("sequent_backend_contest", {
                    id: contest.id,
                    data: {
                        presentation: contestPresentation,
                    },
                    previousData: contest,
                })
            }
            return null
        })
    }

    const transform = (data: Sequent_Backend_Election_Extended): RaRecord<Identifier> => {
        // update candidates
        updateContestsOrder(data)

        // save receipts object
        const receipts: IRECEIPTS = {}
        for (const value in Object.values(ICommunicationMethod) as ICommunicationMethod[]) {
            const key = Object.keys(ICommunicationMethod)[value]
            receipts[key] = {}
            receipts[key]["allowed"] = data.allowed?.[key] || false
            receipts[key]["template"] = data.template?.[key] || null
        }

        data.receipts = {...receipts}
        delete data.allowed
        delete data.template
        delete data.scheduledOpening
        delete data.scheduledClosing

        // save presentation object
        // language_conf
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
                ...data.configuration,
                language_conf: {
                    ...language_conf,
                    default_language_code: data?.presentation?.language_conf?.default_language_code,
                },
            },
        }
    }

    return (
        <EditBase redirect={"."} transform={transform}>
            <ElectionDataForm />
        </EditBase>
    )
}
