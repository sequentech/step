// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {
    CheckboxGroupInput,
    EditBase,
    Identifier,
    List,
    RecordContext,
    SaveButton,
    SimpleForm,
    TextInput,
    useGetList,
    useListContext,
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
import {IUser} from "sequent-core"
import {FormControl, MenuItem, Select, SelectChangeEvent, TextField} from "@mui/material"
import {ElectionHeaderStyles} from "@/components/styles/ElectionHeaderStyles"
import {EditUsersInput, Sequent_Backend_Area} from "@/gql/graphql"
import {EDIT_USER} from "@/queries/EditUser"

interface EditUserFormProps {
    id?: Identifier
    electionEventId?: Identifier
    close?: () => void
}

export const EditUserForm: React.FC<EditUserFormProps> = (props) => {
    const {id, close, electionEventId} = props

    const {data, isLoading} = useListContext<IUser & {id: string}>()
    let userOriginal: IUser | undefined = data?.find((element) => element.id === id)
    const [user, setUser] = useState<IUser | undefined>(userOriginal)

    console.log("DATA :: ", data)
    console.log("USER :: ", user)

    const refresh = useRefresh()
    const notify = useNotify()
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()

    const [edit_user] = useMutation<EditUsersInput>(EDIT_USER)

    const {data: areas} = useGetList<Sequent_Backend_Area>("sequent_backend_area", {
        pagination: {page: 1, perPage: 9999},
        filter: {election_event_id: electionEventId, tenant_id: tenantId},
    })

    console.log("areas :>> ", areas)

    useEffect(() => {
        if (!isLoading && data) {
            let userOriginal: IUser | undefined = data?.find((element) => element.id === id)
            console.log("USER :: ", userOriginal?.attributes?.["area-id"]?.[0] || "---")
            console.log("TENANT :: ", tenantId)
            setUser(userOriginal)
        }
    }, [isLoading, data, id])

    const onSubmit = async () => {
        /*
          tenant_id: String!
                user_id: String!
                enabled: Boolean
                election_event_id: String
                attributes: jsonb
                email: String
                first_name: String
                groups: [String!]
                username: String
            */
        console.log("onSubmit :>> ", user)
        try {
            let {data, errors} = await edit_user({
                variables: {
                    body: {
                        user_id: user?.id,
                        tenant_id: tenantId,
                        election_event_id: electionEventId,
                        first_name: user?.first_name,
                        email: user?.email,
                        attributes: {
                            "area-id": [user?.attributes?.["area-id"]?.[0]],
                        },
                    },
                },
            })
            notify(t("usersAndRolesScreen.voters.errors.editSuccess"), {type: "success"})
            close?.()
        } catch (error) {
            notify(t("usersAndRolesScreen.voters.errors.editError"), {type: "error"})
            close?.()
        }
    }

    const handleChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
        const {name, value} = e.target
        setUser({...user, [name]: value})
    }

    if (!user) {
        return null
    }

    let areaIdAttribute = user?.attributes?.["area-id"] as Array<string> | undefined
    let defaultAreaId = areaIdAttribute?.[0] ?? undefined

    const handleSelectArea = async (e: SelectChangeEvent) => {
        if (!electionEventId) {
            return
        }

        setUser({
            ...user,
            attributes: {
                ...user.attributes,
                "area-id": [e.target.value],
            },
        })
    }

    return (
        <PageHeaderStyles.Wrapper>
            <SimpleForm
                toolbar={<SaveButton alwaysEnable />}
                onSubmit={onSubmit}
                sanitizeEmptyValues
            >
                <>
                    <PageHeaderStyles.Title>
                        {t(`usersAndRolesScreen.${electionEventId ? "voters" : "users"}.title`)}
                    </PageHeaderStyles.Title>
                    <PageHeaderStyles.SubTitle>
                        {t(`usersAndRolesScreen.${electionEventId ? "voters" : "users"}.subtitle`)}
                    </PageHeaderStyles.SubTitle>

                    <TextField
                        variant="outlined"
                        label={t("usersAndRolesScreen.users.fields.first_name")}
                        value={user.first_name || ""}
                        name={"first_name"}
                        onChange={handleChange}
                    />
                    <TextField
                        variant="outlined"
                        label={t("usersAndRolesScreen.users.fields.last_name")}
                        value={user.last_name || ""}
                        name={"last_name"}
                        onChange={handleChange}
                    />
                    <TextField
                        variant="outlined"
                        label={t("usersAndRolesScreen.users.fields.email")}
                        value={user.email || ""}
                        name={"email"}
                        onChange={handleChange}
                    />
                    <TextField
                        variant="outlined"
                        label={t("usersAndRolesScreen.users.fields.username")}
                        value={user.username || ""}
                        name={"username"}
                        onChange={handleChange}
                    />

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
                </>
            </SimpleForm>
        </PageHeaderStyles.Wrapper>
    )
}
