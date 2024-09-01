// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
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
        delete data.enabled_languages

        const presentationI18n = data.presentation?.i18n

        let fromPresentationName = ""
        let fromPresentationAlias = ""
        let fromPresentationDescription = ""

        if (presentationI18n && Object.keys(presentationI18n).length > 0) {
            fromPresentationName =
                presentationI18n.en?.name ||
                presentationI18n[Object.keys(presentationI18n)[0]]?.name ||
                ""

            fromPresentationAlias =
                presentationI18n.en?.alias ||
                presentationI18n[Object.keys(presentationI18n)[0]]?.alias ||
                ""

            fromPresentationDescription =
                presentationI18n.en?.description ||
                presentationI18n[Object.keys(presentationI18n)[0]]?.description ||
                ""
        }

        data.name = fromPresentationName
        data.alias = fromPresentationAlias
        data.description = fromPresentationDescription

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
