import {Sequent_Backend_Candidate} from "@/gql/graphql"
import {CandidatesOrder} from "@sequentech/ui-essentials"
import React from "react"
import {EditBase, Identifier, RaRecord, useUpdate} from "react-admin"
import {ContestDataForm, Sequent_Backend_Contest_Extended} from "./EditContestDataForm"

export const EditContestData: React.FC = () => {
    const [update] = useUpdate()

    function updateCandidatesOrder(data: Sequent_Backend_Contest_Extended) {
        data.candidatesOrder?.map((c: Sequent_Backend_Candidate, index: number) => {
            if (data.contest_candidates_order === CandidatesOrder.CUSTOM) {
                return update("sequent_backend_candidate", {
                    id: c.id,
                    data: {
                        presentation: {
                            ...c.presentation,
                            sort_order: index,
                        },
                    },
                    previousData: c,
                })
            }
            return null
        })
    }

    const transform = (data: Sequent_Backend_Contest_Extended): RaRecord<Identifier> => {
        // update candidates
        updateCandidatesOrder(data)

        delete data.candidatesOrder

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

        return data
    }

    return (
        <EditBase redirect={"."} transform={transform}>
            <ContestDataForm />
        </EditBase>
    )
}
