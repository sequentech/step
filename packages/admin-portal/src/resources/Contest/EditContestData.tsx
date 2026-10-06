// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {CandidatesOrder, ICandidatePresentation, IContestPresentation} from "@sequentech/ui-core"
import React from "react"
import {EditBase, Identifier, RaRecord, useNotify, useUpdate} from "react-admin"
import {ContestDataForm, Sequent_Backend_Contest_Extended} from "./EditContestDataForm"
import {serializeIvrEntityAnnotations} from "@/utils/ivr"

export const EditContestData: React.FC = () => {
    const [update] = useUpdate()
    const notify = useNotify()

    async function updateCandidatesOrder(data: Sequent_Backend_Contest_Extended) {
        const contestPresentation = data.presentation as IContestPresentation | undefined
        if (contestPresentation?.candidates_order === CandidatesOrder.CUSTOM) {
            const candidates = data.candidatesOrder ?? []
            for (let index = 0; index < candidates.length; index++) {
                const candidate = candidates[index]
                const candidatePresentation = candidate.presentation as
                    | ICandidatePresentation
                    | undefined
                if (candidatePresentation?.sort_order !== index) {
                    await update(
                        "sequent_backend_candidate",
                        {
                            id: candidate.id,
                            data: {
                                presentation: {
                                    ...candidatePresentation,
                                    sort_order: index,
                                },
                            },
                            previousData: candidate,
                        },
                        {returnPromise: true}
                    )
                }
            }
        }
    }

    const transform = async (
        data: Sequent_Backend_Contest_Extended
    ): Promise<RaRecord<Identifier>> => {
        // update candidates
        try {
            await updateCandidatesOrder(data)
        } catch (error) {
            notify(error instanceof Error ? error.message : String(error), {type: "error"})
            throw error
        }

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
        data.annotations = serializeIvrEntityAnnotations(data.annotations)

        return data
    }

    return (
        <EditBase redirect={"."} transform={transform}>
            <ContestDataForm />
        </EditBase>
    )
}
