// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {
    SaveButton,
    SimpleForm,
    useGetList,
    useListContext,
    useNotify,
    useRefresh,
} from "react-admin"
import {useMutation, useQuery} from "@apollo/client"
import {PageHeaderStyles} from "../../components/styles/PageHeaderStyles"
import {useTranslation} from "react-i18next"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {IRole, IUser} from "sequent-core"
import {
    FormControl,
    MenuItem,
    Select,
    SelectChangeEvent,
    FormControlLabel,
    Checkbox,
} from "@mui/material"
import {ElectionHeaderStyles} from "@/components/styles/ElectionHeaderStyles"
import {
    DeleteUserRoleMutation,
    EditUsersInput,
    ListUserRolesQuery,
    Sequent_Backend_Area,
    SetUserRoleMutation,
} from "@/gql/graphql"
import {EDIT_USER} from "@/queries/EditUser"
import {LIST_USER_ROLES} from "@/queries/ListUserRoles"
import {DataGrid, GridColDef, GridRenderCellParams} from "@mui/x-data-grid"
import {isUndefined} from "@sequentech/ui-essentials"
import {DELETE_USER_ROLE} from "@/queries/DeleteUserRole"
import {SET_USER_ROLE} from "@/queries/SetUserRole"
import { FormStyles } from "@/components/styles/FormStyles"

interface SendCommunicationFormProps {
    id?: string
    electionEventId?: string
    close?: () => void
}

export const SendCommunicationForm: React.FC<SendCommunicationFormProps> = ({
    id,
    close,
    electionEventId
}) => {
    const {data, isLoading} = useListContext<IUser & {id: string}>()
    let userOriginal: IUser | undefined = data?.find((element) => element.id === id)
    const [user, setUser] = useState<IUser | undefined>(userOriginal)
    const notify = useNotify()
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const refresh = useRefresh()

    const [edit_user] = useMutation<EditUsersInput>(EDIT_USER)

    useEffect(() => {
        if (!isLoading && data) {
            let userOriginal: IUser | undefined = data?.find((element) => element.id === id)
            setUser(userOriginal)
        }
    }, [isLoading, data, id])

    const onSubmit = async () => {
        try {
            //let {data} = await edit_user({
            //    variables: {
            //        body: {
            //            user_id: user?.id,
            //            tenant_id: tenantId,
            //            election_event_id: electionEventId,
            //            first_name: user?.first_name,
            //            last_name: user?.last_name,
            //            enabled: user?.enabled,
            //            password: (user?.password && user?.password.length > 0)
            //                ? user.password
            //                : undefined,
            //            email: user?.email,
            //            attributes: {
            //                "area-id": [user?.attributes?.["area-id"]?.[0]],
            //            },
            //        },
            //    },
            //})
            notify(t("usersAndRolesScreen.voters.errors.editSuccess"), {type: "success"})
            refresh()
            close?.()
        } catch (error) {
            notify(t("usersAndRolesScreen.voters.errors.editError"), {type: "error"})
            close?.()
        }
    }

    if (!user) {
        return null
    }

    let areaIdAttribute = user?.attributes?.["area-id"] as Array<string> | undefined
    let defaultAreaId = areaIdAttribute?.[0] ?? undefined

    return (
        <PageHeaderStyles.Wrapper>
            <SimpleForm
                toolbar={<SaveButton alwaysEnable />}
                record={user}
                onSubmit={onSubmit}
                sanitizeEmptyValues
            >
                <>
                    <PageHeaderStyles.Title>
                        {t(`sendCommunication.title`)}
                    </PageHeaderStyles.Title>
                    <PageHeaderStyles.SubTitle>
                        {t(`sendCommunication.subtitle`)}
                    </PageHeaderStyles.SubTitle>

                    {/*electionEventId ? (
                        <FormControl fullWidth>
                            <ElectionHeaderStyles.Title>
                                {t("usersAndRolesScreen.users.fields.area")}
                            </ElectionHeaderStyles.Title>

                            <Select
                                name="area"
                                defaultValue={defaultAreaId}
                                value={defaultAreaId}
                                onChange={handleSelectArea}
                            >
                                {areas?.map((area: Sequent_Backend_Area) => (
                                    <MenuItem key={area.id} value={area.id}>
                                        {area.name}
                                    </MenuItem>
                                ))}
                            </Select>
                        </FormControl>
                    ) : null*/}
                </>
            </SimpleForm>
        </PageHeaderStyles.Wrapper>
    )
}
