// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {EditBase, Identifier, RaRecord, useUpdate} from "react-admin"
import {
    EditElectionEventDataForm,
    Sequent_Backend_Election_Event_Extended,
} from "./EditElectionEventDataForm"
import {Sequent_Backend_Election} from "@/gql/graphql"
import {
    ElectionsOrder,
    IElectionEventPresentation,
    IElectionPresentation,
} from "@sequentech/ui-core"

export const EditElectionEventData: React.FC = () => {
    const [update] = useUpdate()

    function updateElectionsOrder(data: Sequent_Backend_Election_Event_Extended) {
        data.electionsOrder?.map((election: Sequent_Backend_Election, index: number) => {
            let electionEventPresentation = data.presentation as
                | IElectionEventPresentation
                | undefined
            if (electionEventPresentation?.elections_order === ElectionsOrder.CUSTOM) {
                let electionPresentation = (election.presentation ?? {}) as IElectionPresentation
                return update("sequent_backend_election", {
                    id: election.id,
                    data: {
                        presentation: {
                            ...electionPresentation,
                            sort_order: index,
                        },
                    },
                    previousData: election,
                })
            }
            return null
        })
    }

    const transform = (data: Sequent_Backend_Election_Event_Extended): RaRecord<Identifier> => {
        //update elections
        updateElectionsOrder(data)

        delete data.electionsOrder

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
                    default_language_code: data?.presentation?.language_conf?.default_language_code,
                },
            },
        }
    }

    return (
        <EditBase redirect={"."} transform={transform}>
            <EditElectionEventDataForm />
        </EditBase>
    )
}
