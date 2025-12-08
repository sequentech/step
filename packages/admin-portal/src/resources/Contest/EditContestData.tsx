// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Sequent_Backend_Candidate} from "@/gql/graphql"
import {CandidatesOrder, ICandidatePresentation, IContestPresentation} from "@sequentech/ui-core"
import React from "react"
import {EditBase, Identifier, RaRecord, useUpdate} from "react-admin"
import {ContestDataForm, Sequent_Backend_Contest_Extended} from "./EditContestDataForm"

export const EditContestData: React.FC = () => {
    const [update] = useUpdate()

    function updateCandidatesOrder(data: Sequent_Backend_Contest_Extended) {
        data.candidatesOrder?.map((candidate: Sequent_Backend_Candidate, index: number) => {
            let contestPresentation = data.presentation as IContestPresentation | undefined
            if (contestPresentation?.candidates_order === CandidatesOrder.CUSTOM) {
                let candidatePresentation = candidate.presentation as
                    | ICandidatePresentation
                    | undefined
                return update("sequent_backend_candidate", {
                    id: candidate.id,
                    data: {
                        presentation: {
                            ...candidatePresentation,
                            sort_order: index,
                        },
                    },
                    previousData: candidate,
                })
            }
            return null
        })
    }

    const transform = (data: Sequent_Backend_Contest_Extended): RaRecord<Identifier> => {
        // update candidates
        updateCandidatesOrder(data)

        delete data.candidatesOrder

        const presentation = data?.presentation as IContestPresentation | undefined
        const i18n = presentation?.i18n ?? {}

        // name, alias and description fields
        const fromPresentationName = i18n?.en?.name || i18n[Object.keys(i18n)[0]]?.name || ""
        data.name = fromPresentationName
        const fromPresentationAlias = i18n?.en?.alias || i18n[Object.keys(i18n)[0]]?.alias || ""
        data.alias = fromPresentationAlias
        const fromPresentationDescription =
            i18n?.en?.description || i18n[Object.keys(i18n)[0]]?.description || ""
        data.description = fromPresentationDescription
        // END name, alias and description fields

        return data
    }

    return (
        <EditBase redirect={"."} transform={transform}>
            <ContestDataForm />
        </EditBase>
    )
}
