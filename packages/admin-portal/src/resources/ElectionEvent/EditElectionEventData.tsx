// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useRef} from "react"
import {useMutation} from "@apollo/client"
import {EditBase, Identifier, RaRecord, useUpdate, useRefresh, useNotify} from "react-admin"
import {
    EditElectionEventDataForm,
    Sequent_Backend_Election_Event_Extended,
} from "./EditElectionEventDataForm"
import {Sequent_Backend_Election} from "@/gql/graphql"
import {
    ElectionsOrder,
    IElectionEventPresentation,
    IElectionPresentation,
    IResultsWebsitePolicy,
} from "@sequentech/ui-core"
import {
    CONFIGURE_RESULTS_WEBSITE_POLICY,
    ConfigureResultsWebsitePolicyData,
    ConfigureResultsWebsitePolicyVariables,
} from "@/queries/ResultsWebsitePublication"
import {IPermissions} from "@/types/keycloak"

export const EditElectionEventData: React.FC = () => {
    const [update] = useUpdate()
    const refresh = useRefresh()
    const notify = useNotify()
    const resultsWebsitePolicy = useRef<IResultsWebsitePolicy | undefined>(undefined)
    const [configureResultsWebsitePolicy] = useMutation<
        ConfigureResultsWebsitePolicyData,
        ConfigureResultsWebsitePolicyVariables
    >(CONFIGURE_RESULTS_WEBSITE_POLICY, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.PUBLISH_RESULTS_WRITE,
            },
        },
    })

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
        resultsWebsitePolicy.current = data.resultsWebsitePolicy
        delete data.resultsWebsitePolicy

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
                    language_detection_policy:
                        data?.presentation?.language_conf?.language_detection_policy,
                },
            },
        }
    }

    const onSuccess = async (data: RaRecord<Identifier>) => {
        const electionEventId = data?.id?.toString()

        if (electionEventId) {
            try {
                const policy = resultsWebsitePolicy.current
                if (!policy) {
                    throw new Error("Results website policy is missing")
                }
                await configureResultsWebsitePolicy({
                    variables: {
                        election_event_id: electionEventId,
                        status: policy.status,
                        access: policy.access,
                        visibility_scope: policy.visibility_scope,
                    },
                })
            } catch (error) {
                console.error(error)
                notify("Failed to update the results website policy", {type: "error"})
            }
        }

        refresh()
    }

    return (
        <EditBase
            redirect={"."}
            transform={transform}
            mutationMode="pessimistic"
            mutationOptions={{onSuccess}}
        >
            <EditElectionEventDataForm />
        </EditBase>
    )
}
