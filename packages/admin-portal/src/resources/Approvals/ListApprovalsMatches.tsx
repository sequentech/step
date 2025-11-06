// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {ReactElement, useContext, useMemo, useState} from "react"
import {
    DatagridConfigurable,
    List,
    TextField,
    TextInput,
    Identifier,
    useRefresh,
    useNotify,
    FunctionField,
    BooleanInput,
    DateInput,
} from "react-admin"
import {useTenantStore} from "@/providers/TenantContextProvider"
import CheckCircleOutlineIcon from "@mui/icons-material/CheckCircleOutline"
import {ListActions} from "@/components/ListActions"
import {Chip, Tooltip, Typography} from "@mui/material"
import {Dialog} from "@sequentech/ui-essentials"
import {useTranslation} from "react-i18next"
import {Action, ActionsColumn} from "@/components/ActionButons"
import {AuthContext} from "@/providers/AuthContextProvider"
import {
    ChangeApplicationStatusMutation,
    GetUserProfileAttributesQuery,
    Sequent_Backend_Applications,
    UserProfileAttribute,
} from "@/gql/graphql"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import {IUser} from "@sequentech/ui-core"
import {USER_PROFILE_ATTRIBUTES} from "@/queries/GetUserProfileAttributes"
import {getAttributeLabel, getTranslationLabel, userBasicInfo} from "@/services/UserService"
import CustomDateField from "../User/CustomDateField"
import {styled} from "@mui/material/styles"
import eStyled from "@emotion/styled"
import SelectArea from "@/components/area/SelectArea"
import ElectionHeader from "@/components/ElectionHeader"
import {CHANGE_APPLICATION_STATUS} from "@/queries/ChangeApplicationStatus"
import {useMutation, useQuery} from "@apollo/client"
import {PreloadedList} from "./PreloadedList"
import {convertToSnakeCase, convertToCamelCase, convertOneToSnakeCase} from "./UtilsApprovals"
import {ApplicationsError} from "@/types/applications"

const StyledChip = styled(Chip)`
    margin: 4px;
`

const StyledNull = eStyled.div`
    display: block;
    padding-left: 18px;
`

export interface ListUsersProps {
    electionEventId?: string
    electionId?: string
    task: Sequent_Backend_Applications
    goBack: () => void
}

export const ListApprovalsMatches: React.FC<ListUsersProps> = ({
    electionEventId,
    electionId,
    task,
    goBack,
}) => {
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const notify = useNotify()
    const refresh = useRefresh()

    const [openApproveModal, setOpenApproveModal] = React.useState(false)
    const [userId, setUserId] = useState<string | undefined>()
    const authContext = useContext(AuthContext)

    // const canEditUsers = authContext.isAuthorized(true, tenantId, IPermissions.VOTER_WRITE)
    const [approveVoter] = useMutation<ChangeApplicationStatusMutation>(CHANGE_APPLICATION_STATUS)

    const userApprovalInfo = Object.entries(convertToSnakeCase(task.applicant_data)).map(
        ([key, value]) => key
    )

    const searchAttrs = task?.annotations?.["search-attributes"]
        .split(",")
        .map((s: string) => convertOneToSnakeCase(s))

    const {data: userAttributes} = useQuery<GetUserProfileAttributesQuery>(
        USER_PROFILE_ATTRIBUTES,
        {
            variables: {
                tenantId: tenantId,
                electionEventId: electionEventId,
            },
        }
    )

    const defaultFilters = useMemo(() => {
        if (!userAttributes?.get_user_profile_attributes) {
            return {}
        }

        let filters: Record<string, {IsLike: string}> = {}
        for (const attr of userAttributes.get_user_profile_attributes) {
            if (attr.name && searchAttrs.includes(`${attr.name}`)) {
                filters[attr.name] = {IsLike: ""}
                filters[attr.name].IsLike =
                    task.applicant_data?.[convertToCamelCase(attr.name)] ?? ""
            }
        }
        return filters
    }, [userAttributes?.get_user_profile_attributes, searchAttrs, task.applicant_data])

    const Filters = useMemo(() => {
        let filters: ReactElement[] = []
        if (userAttributes?.get_user_profile_attributes) {
            filters = userAttributes.get_user_profile_attributes.map((attr) => {
                //covert to valid source string (if attr name is for example sequent.read-only.otp-method)
                const source = attr.name?.replaceAll(".", "%")
                if (attr.annotations?.inputType === "html5-date") {
                    return (
                        <DateInput
                            key={attr.name}
                            source={`attributes.${attr.name}`}
                            label={getTranslationLabel(attr.name, attr.display_name, t)}
                        />
                    )
                }
                return (
                    <TextInput
                        key={attr.name}
                        source={
                            attr.name && userBasicInfo.includes(attr.name)
                                ? `${attr.name}.IsLike`
                                : `attributes.${source}`
                        }
                        label={getTranslationLabel(attr.name, attr.display_name, t)}
                        // alwaysOn={searchAttrs.includes(`${attr.name}`)}
                    />
                )
            })
            filters.push(<BooleanInput key="enabled" source={"enabled"} />)
            filters.push(<BooleanInput key="email_verified" source={"email_verified"} />)
            if (electionEventId) {
                filters.push(
                    <SelectArea
                        tenantId={tenantId}
                        electionEventId={electionEventId}
                        source="attributes.area-id"
                        label={t("usersAndRolesScreen.users.fields.area")}
                    />
                )
            }
            if (electionEventId && !electionId) {
                filters.push(
                    <BooleanInput
                        key="has_voted"
                        source={"has_voted"}
                        label={t("usersAndRolesScreen.users.fields.has_voted")}
                    />
                )
            }
        }
        return filters
    }, [userAttributes?.get_user_profile_attributes])

    const approveAction = (id: Identifier) => {
        if (!electionEventId && authContext.userId === id) {
            return
        }
        setOpenApproveModal(true)
        setUserId(id as string)
    }

    const confirmApproveAction = async () => {
        const {data, errors} = await approveVoter({
            variables: {
                tenant_id: tenantId,
                id: task?.id,
                user_id: userId,
                area_id: task?.area_id,
                election_event_id: electionEventId,
            },
        })
        if (data?.ApplicationChangeStatus?.error || errors) {
            let errorMessage =
                data?.ApplicationChangeStatus?.error &&
                data?.ApplicationChangeStatus?.error === ApplicationsError.APPROVED_VOTER
                    ? t(`approvalsScreen.notifications.VoterApprovedAlready`)
                    : t(`approvalsScreen.notifications.approveError`)
            notify(errorMessage, {type: "error"})
            console.log(
                `Error approve user: ${errors ?? ""} ${data?.ApplicationChangeStatus?.error ?? ""}`
            )
            return
        }
        notify(t(`approvalsScreen.notifications.approveSuccess`), {type: "success"})
        setUserId(undefined)
        goBack()
    }

    const actions: Action[] = [
        {
            icon: (
                <Tooltip title={t(`common.label.approve`)} placement="right">
                    <CheckCircleOutlineIcon
                        color="success"
                        className="approve-voter-icon"
                        sx={{
                            "cursor": "pointer",
                            "transform": "scale(1.2)",
                            "width": "48px",
                            "padding": "1px",
                            "borderRadius": "4px",
                            "backgroundColor": "rgba(0, 128, 0, 0.1)",
                            "transition": "transform 0.2s, background-color 0.2s, box-shadow 0.2s",
                            "&:hover": {
                                backgroundColor: "rgba(0, 128, 0, 0.2)",
                                boxShadow: "0 4px 8px rgba(0, 0, 0, 0.2)",
                                transform: "scale(1.2)",
                            },
                        }}
                    />
                </Tooltip>
            ),
            action: approveAction,
            showAction: () => task?.status === "PENDING" || task?.status === "REJECTED",
            label: t(`common.label.delete`),
            className: "approve-voter-icon",
        },
    ]

    const Empty = () => (
        <ResourceListStyles.EmptyBox>
            <Typography variant="h4" paragraph>
                {t(`usersAndRolesScreen.${electionEventId ? "voters" : "users"}.emptyHeader`)}
            </Typography>
        </ResourceListStyles.EmptyBox>
    )

    const listFields = useMemo(() => {
        const basicInfoFields: UserProfileAttribute[] = []
        const attributesFields: UserProfileAttribute[] = []
        const omitFields = [
            "id",
            "email_verified",
            "username",
            "emailAndOrMobile",
            "sequent.read-only.mobile-number",
            "sequent.read-only.otp-method",
            "embassy",
            "country",
            "sequent.read-only.id-card-type",
            "sequent.read-only.id-card-number",
            "sequent.read-only.id-card-number-validated",
            "authorized-election-ids",
        ]

        userAttributes?.get_user_profile_attributes.forEach((attr) => {
            if (attr.name && userApprovalInfo.includes(attr.name)) {
                basicInfoFields.push(attr)
            } else {
                omitFields.push(`attributes['${attr.name}']`)
                attributesFields.push(attr)
            }
        })

        return {basicInfoFields, attributesFields, omitFields}
    }, [userAttributes?.get_user_profile_attributes])

    const renderFields = (fields: UserProfileAttribute[]) =>
        fields.map((attr) => {
            if (attr.annotations?.inputType === "html5-date") {
                return (
                    <FunctionField
                        key={attr.name}
                        source={`attributes['${attr.name}']`}
                        label={getTranslationLabel(attr.name, attr.display_name, t)}
                        render={(record: IUser, source: string | undefined) => {
                            return (
                                <CustomDateField
                                    key={attr.name}
                                    base="attributes"
                                    source={`${attr.name}`}
                                    label={getTranslationLabel(attr.name, attr.display_name, t)}
                                    emptyText="-"
                                />
                            )
                        }}
                    />
                )
            } else if (attr.multivalued) {
                return (
                    <FunctionField
                        key={attr.name}
                        label={getTranslationLabel(attr.name, attr.display_name, t)}
                        render={(record: IUser, source: string | undefined) => {
                            let value: any =
                                attr.name && userApprovalInfo.includes(attr.name)
                                    ? (record as any)[attr.name]
                                    : attr?.name
                                    ? (record as any).attributes[attr?.name]
                                    : "-"

                            return (
                                <>
                                    {value ? (
                                        value.map((item: any, index: number) => (
                                            <StyledChip key={index} label={item} />
                                        ))
                                    ) : (
                                        <StyledNull>-</StyledNull>
                                    )}
                                </>
                            )
                        }}
                    />
                )
            }

            if (attr.name) {
                return (
                    <TextField
                        key={attr.name}
                        source={
                            userBasicInfo.includes(attr.name)
                                ? attr.name
                                : `attributes['${attr.name}']`
                        }
                        label={getTranslationLabel(attr.name, attr.display_name, t)}
                        emptyText="-"
                    />
                )
            } else {
                return null
            }
        })

    return (
        <>
            <ElectionHeader title="approvalsScreen.title" subtitle="approvalsScreen.subtitle" />

            <List
                resource="user"
                empty={<Empty />}
                actions={<ListActions withImport={false} withExport={false} />}
                filter={{
                    tenant_id: tenantId,
                    election_event_id: electionEventId,
                    election_id: electionId,
                }}
                storeKey={false}
                filters={Filters}
                filterDefaultValues={defaultFilters}
                disableSyncWithLocation
            >
                <PreloadedList defaultFilters={defaultFilters} resource="user">
                    {userAttributes?.get_user_profile_attributes && (
                        <DatagridConfigurable
                            omit={listFields.omitFields}
                            bulkActionButtons={false}
                        >
                            <TextField source="id" sx={{display: "block", width: "280px"}} />
                            {renderFields(listFields?.basicInfoFields)}
                            {renderFields(listFields?.attributesFields)}
                            <ActionsColumn actions={actions} label={t("common.label.actions")} />
                        </DatagridConfigurable>
                    )}
                </PreloadedList>
            </List>

            <Dialog
                variant="info"
                open={openApproveModal}
                ok={t("common.label.approve")}
                cancel={t("common.label.cancel")}
                title={t("common.label.warning")}
                handleClose={(result: boolean) => {
                    if (result) {
                        confirmApproveAction()
                    }
                    setOpenApproveModal(false)
                }}
            >
                {t(`approvalsScreen.approve.body`)}
            </Dialog>
        </>
    )
}
