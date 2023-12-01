// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {
    Identifier,
    SaveButton,
    SimpleForm,
    useGetList,
    useListContext,
    useNotify,
    useRefresh,
} from "react-admin"
import {useMutation} from "@apollo/client"
import {PageHeaderStyles} from "../../components/styles/PageHeaderStyles"
import {useTranslation} from "react-i18next"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {EDIT_USER} from "../../queries/EditUser"
import {IUser} from "sequent-core"
import {FormControl, FormLabel, MenuItem, Select, SelectChangeEvent, TextField} from "@mui/material"
import {EditUsersInput} from "@/gql/graphql"
import {ElectionHeaderStyles} from "@/components/styles/ElectionHeaderStyles"

interface EditUserFormProps {
    id?: Identifier | undefined
    electionEventId: Identifier | undefined
    close?: () => void
}

export const EditUserForm: React.FC<EditUserFormProps> = (props) => {
    const {id, close, electionEventId} = props

    const {data, isLoading} = useListContext()
    const [user, setUser] = useState<any | undefined>()

    const refresh = useRefresh()
    const notify = useNotify()
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()

    const [edit_user] = useMutation<EditUsersInput>(EDIT_USER)

    const {data: areas} = useGetList("sequent_backend_area", {
        pagination: {page: 1, perPage: 9999},
        filter: {election_event_id: electionEventId, tenant_id: tenantId},
    })

    console.log("areas :>> ", areas)

    useEffect(() => {
        if (!isLoading && data) {
            let userOriginal: IUser | undefined = data?.find((element) => element.id === id)
            console.log(
                "USER :: ",
                JSON.parse(JSON.stringify(userOriginal?.attributes?.["area-id"]))[0] ?? "---"
            )
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
                        first_name: user?.first_name,
                        email: user?.email,
                        attributes: {
                            "area-id": [user?.area],
                        },
                    },
                },
            })
            console.log("data after SAVE :>> ", data)
            console.log("errors after SAVE :>> ", errors)
        } catch (error) {
            console.log("error :>> ", error)
        }
    }

    const handleChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
        const {name, value} = e.target
        setUser({...user, [name]: value})
    }

    const handleSelectChange = async (e: SelectChangeEvent) => {
        const {name, value} = e.target
        setUser({...user, [name]: value})
    }

    const renderAreas = (areas: any) => {
        return areas.map((area: any) => {
            return (
                <MenuItem key={area.id} value={area.id}>
                    {area.name}
                </MenuItem>
            )
        })
    }

    if (user) {
        return (
            <SimpleForm
                toolbar={<SaveButton alwaysEnable />}
                onSubmit={onSubmit}
                sanitizeEmptyValues
            >
                <>
                    <PageHeaderStyles.Title>
                        {t("usersAndRolesScreen.voters.title")}
                    </PageHeaderStyles.Title>
                    <PageHeaderStyles.SubTitle>
                        {t("usersAndRolesScreen.voters.subtitle")}
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

                    <FormControl sx={{width: "100%"}}>
                        <ElectionHeaderStyles.Title>
                            {t("usersAndRolesScreen.users.fields.area")}
                        </ElectionHeaderStyles.Title>

                        <Select
                            name="area"
                            defaultValue={
                                JSON.parse(JSON.stringify(user?.attributes?.["area-id"]))[0] || null
                            }
                            onChange={handleSelectChange}
                        >
                            {areas ? renderAreas(areas) : null}
                        </Select>
                    </FormControl>
                </>
            </SimpleForm>
        )
    } else {
        return null
    }
}
