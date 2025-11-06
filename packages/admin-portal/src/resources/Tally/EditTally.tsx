// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {
    CheckboxGroupInput,
    EditBase,
    Identifier,
    RecordContext,
    SaveButton,
    SimpleForm,
    useGetList,
    useNotify,
    useRefresh,
} from "react-admin"
import {PageHeaderStyles} from "../../components/styles/PageHeaderStyles"
import {useTranslation} from "react-i18next"
import {useTenantStore} from "@/providers/TenantContextProvider"

interface EditTallyProps {
    id?: Identifier | undefined
    electionEventId: Identifier | undefined
    close?: () => void
}

export const EditTally: React.FC<EditTallyProps> = (props) => {
    const {id, close, electionEventId} = props

    const refresh = useRefresh()
    const notify = useNotify()
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()

    const [renderUI, setRenderUI] = useState(false)

    const {data: elections} = useGetList("sequent_backend_election", {
        pagination: {page: 1, perPage: 9999},
        filter: {election_event_id: electionEventId, tenant_id: tenantId},
    })

    const {data: trustees} = useGetList("sequent_backend_trustee", {
        pagination: {page: 1, perPage: 9999},
        filter: {tenant_id: tenantId},
    })

    useEffect(() => {
        if (elections && trustees) {
            setRenderUI(true)
        }
    }, [elections, trustees])

    const parseValues = (incoming: any) => {
        const temp = {...incoming}

        // temp.election_ids = elections?.map(
        //     (election: any) => election.id
        // )

        return temp
    }
    function shallowEqual(object1: any, object2: any) {
        const keys1 = Object.keys(object1)
        const keys2 = Object.keys(object2)

        if (keys1.length !== keys2.length) {
            return false
        }

        for (let key of keys1) {
            if (object1[key] !== object2[key]) {
                return false
            }
        }

        return true
    }

    const transform = async (data: any, {previousData}: any) => {
        const temp = {...data}

        if (shallowEqual(temp, previousData)) {
            console.log("NO CHANGES")
            return {id: temp.id, last_updated_at: new Date().toISOString()}
        }
        return {...temp, last_updated_at: new Date().toISOString()}
    }

    const onSuccess = async (res: any) => {
        console.log("onSuccess :>> ", res)

        refresh()
        notify("Area updated", {type: "success"})
        if (close) {
            setTimeout(() => {
                close()
            }, 400)
        }
    }

    const onError = async (res: any) => {
        console.log("onError :>> ", res)

        refresh()
        notify("Could not update Area", {type: "error"})
        if (close) {
            setTimeout(() => {
                close()
            }, 400)
        }
    }

    if (renderUI) {
        return (
            <EditBase
                id={id}
                transform={transform}
                resource="sequent_backend_tally_session"
                mutationMode="pessimistic"
                mutationOptions={{onSuccess, onError}}
                redirect={false}
            >
                <PageHeaderStyles.Wrapper>
                    <RecordContext.Consumer>
                        {(incoming) => {
                            const parsedValue = parseValues(incoming)
                            console.log("parsedValue :>> ", parsedValue)
                            return (
                                <SimpleForm record={parsedValue} toolbar={<SaveButton />}>
                                    <>
                                        <PageHeaderStyles.Title>
                                            {t("tally.common.title")}
                                        </PageHeaderStyles.Title>
                                        <PageHeaderStyles.SubTitle>
                                            {t("tally.common.subTitle")}
                                        </PageHeaderStyles.SubTitle>

                                        {/* {trustees ? (
                                            <CheckboxGroupInput
                                                label={t("electionEventScreen.tally.trustees")}
                                                source="trustee_ids"
                                                choices={trustees}
                                                optionText="name"
                                                optionValue="id"
                                                row={false}
                                            />
                                        ) : null} */}

                                        {elections ? (
                                            <CheckboxGroupInput
                                                label={t("electionEventScreen.tally.elections")}
                                                source="election_ids"
                                                choices={elections}
                                                optionText="name"
                                                optionValue="id"
                                                row={false}
                                            />
                                        ) : null}
                                    </>
                                </SimpleForm>
                            )
                        }}
                    </RecordContext.Consumer>
                </PageHeaderStyles.Wrapper>
            </EditBase>
        )
    } else {
        return null
    }
}
