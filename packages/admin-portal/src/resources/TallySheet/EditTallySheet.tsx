// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {
    CheckboxGroupInput,
    CreateBase,
    EditBase,
    Identifier,
    RecordContext,
    ReferenceInput,
    SaveButton,
    SimpleForm,
    TextInput,
    Toolbar,
    useGetList,
    useNotify,
    useRefresh,
} from "react-admin"
import {useMutation, useQuery} from "@apollo/client"
import {PageHeaderStyles} from "../../components/styles/PageHeaderStyles"
import {useTranslation} from "react-i18next"
import {GET_AREAS_EXTENDED} from "@/queries/GetAreasExtended"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {INSERT_AREA_CONTESTS} from "../../queries/InsertAreaContest"
import {DELETE_AREA_CONTESTS} from "@/queries/DeleteAreaContest"
import {Sequent_Backend_Contest, Sequent_Backend_Tally_Sheet} from "@/gql/graphql"
import {FieldValues, SubmitHandler} from "react-hook-form"

interface EditTallySheetProps {
    contest: Sequent_Backend_Contest
    areaId?: Identifier | undefined
    id?: Identifier | undefined
    doSelectArea?: (areaId: Identifier) => void
    doEditedTalySheet?: (tallySheet: Sequent_Backend_Tally_Sheet) => void
    submitRef: any
}

export const EditTallySheet: React.FC<EditTallySheetProps> = (props) => {
    const {areaId, id, contest, doSelectArea, submitRef} = props

    const refresh = useRefresh()
    const notify = useNotify()
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()

    const [renderUI, setRenderUI] = useState(false)

    const onSubmit: SubmitHandler<FieldValues> = async (result) => {
        const temp = {...result}
        console.log("temp :>> ", temp)
        console.log("contest :>> ", contest)
        console.log("areaId :>> ", areaId)
    }

    const parseValues = (incoming: any) => {
        const temp = {...incoming}

        return temp
    }

    if (id) {
        return (
            <EditBase id={id} resource="sequent_backend_tally_sheet" redirect={false}>
                <PageHeaderStyles.Wrapper>
                    <RecordContext.Consumer>
                        {(incoming) => {
                            const parsedValue = parseValues(incoming)
                            console.log("parsedValue :>> ", parsedValue)
                            return (
                                <SimpleForm
                                    record={parsedValue}
                                    toolbar={<SaveButton />}
                                    onSubmit={onSubmit}
                                >
                                    <>
                                        <PageHeaderStyles.Title>
                                            {t("areas.common.title")}
                                        </PageHeaderStyles.Title>
                                        <PageHeaderStyles.SubTitle>
                                            {t("areas.common.subTitle")}
                                        </PageHeaderStyles.SubTitle>

                                        <TextInput source="name" />
                                        <TextInput source="description" />
                                    </>
                                </SimpleForm>
                            )
                        }}
                    </RecordContext.Consumer>
                </PageHeaderStyles.Wrapper>
            </EditBase>
        )
    } else {
        return (
            <CreateBase resource="sequent_backend_tally_sheet" redirect={false}>
                <SimpleForm toolbar={false} onSubmit={onSubmit}>
                    <>
                        <PageHeaderStyles.Title>{t("areas.common.title")}</PageHeaderStyles.Title>
                        <PageHeaderStyles.SubTitle>
                            {t("areas.common.subTitle")}
                        </PageHeaderStyles.SubTitle>
                        <TextInput source="name" />
                        <TextInput source="description" />
                        <ReferenceInput source="tenant_id" reference="sequent_backend_tally_contest">
                            <TextInput source defaultValue={tenantId} />
                        </ReferenceInput>

                        

                        <TextInput source="description" />
                        <TextInput source="description" />
                        <button ref={submitRef} type="submit" style={{display: "none"}} />
                    </>
                </SimpleForm>
            </CreateBase>
        )
    }
}

// const CreateToolBar = (props: any) => (
//     <Toolbar sx={{
//         display: "flex",
//         justifyContent: "end",
//         alignItems: "center",
//         mt: -36,
//     }}>
//         <SaveButton label="Show and Confirm" />
//     </Toolbar>
// )
