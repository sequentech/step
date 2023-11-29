// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
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
    TextInput,
    useGetList,
    useNotify,
    useRefresh,
} from "react-admin"
import {useQuery} from "@apollo/client"
import {PageHeaderStyles} from "../../components/styles/PageHeaderStyles"
import {useTranslation} from "react-i18next"
import {GET_AREAS_EXTENDED} from "@/queries/GetAreasExtended"

interface EditAreaProps {
    id?: Identifier | undefined
    electionEventId: Identifier | undefined
    close?: () => void
}

export const EditArea: React.FC<EditAreaProps> = (props) => {
    const {id, close, electionEventId} = props
    const refresh = useRefresh()
    const notify = useNotify()
    const {t} = useTranslation()

    const [renderUI, setRenderUI] = useState(false)

    const {data: contests} = useGetList("sequent_backend_contest", {
        pagination: {page: 1, perPage: 9999},
        filter: {election_event_id: electionEventId},
    })

    const {data: areas} = useQuery(GET_AREAS_EXTENDED, {
        variables: {
            electionEventId,
            areaId: id,
        },
    })

    useEffect(() => {
        if (contests && areas) {
            setRenderUI(true)
        }
    }, [areas, contests])

    const parseValues = (incoming: any) => {
        const temp = {...incoming}

        temp.area_contest_ids = areas?.sequent_backend_area_contest?.map(
            (area: any) => area.contest.id
        )

        return temp
    }

    const transform = (data: any) => {
        const temp = {...data}

        
        console.log("DATA RECEIVED :: ", data);
        delete temp.area_contest_ids
        console.log("DATA TO SAVE :: ", temp);

        return temp
    }

    const onSuccess = async (res: any) => {
        refresh()
        notify("Area updated", {type: "success"})
        if (close) {
            close()
        }
    }

    const onError = async (res: any) => {
        console.log("res :>> ", res);
        
        refresh()
        notify("Could not update Area", {type: "error"})
        if (close) {
            close()
        }
    }

    if (renderUI) {
        return (
            <EditBase
                id={id}
                transform={transform}
                resource="sequent_backend_area"
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
                                            {t("areas.common.title")}
                                        </PageHeaderStyles.Title>
                                        <PageHeaderStyles.SubTitle>
                                            {t("areas.common.subTitle")}
                                        </PageHeaderStyles.SubTitle>

                                        <TextInput source="name" />

                                        {contests ? (
                                            <CheckboxGroupInput
                                                source="area_contest_ids"
                                                choices={contests}
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
