// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {ReactElement, useEffect, useMemo, useState} from "react"
import {
    List,
    DateField,
    FunctionField,
    TextField,
    DatagridConfigurable,
    Identifier,
    SelectInput,
    TextInput,
    useListContext,
    DatagridConfigurableProps,
    useRecordContext,
    useGetRecordId,
    ReferenceManyField,
    Datagrid,
    WithListContext,
} from "react-admin"
import {TFunction, useTranslation} from "react-i18next"
import {Visibility} from "@mui/icons-material"
import {Action, ActionsColumn} from "@/components/ActionButons"
import {ListActions} from "@/components/ListActions"
import {
    GetApplicantAttributesQuery,
    GetUserProfileAttributesQuery,
    Sequent_Backend_Applications,
    Sequent_Backend_Election_Event,
    UserProfileAttribute,
} from "@/gql/graphql"
import {StatusApplicationChip} from "@/components/StatusApplicationChip"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {useQuery} from "@apollo/client"
import {USER_PROFILE_ATTRIBUTES} from "@/queries/GetUserProfileAttributes"
import {convertToCamelCase, convertToSnakeCase} from "./UtilsApprovals"
import {getAttributeLabel} from "@/services/UserService"
import CustomDateField from "../User/CustomDateField"
import {styled} from "@mui/material/styles"
import eStyled from "@emotion/styled"
import {Chip, Typography} from "@mui/material"
import {GET_APPLICANT_ATTRIBUTES} from "@/queries/GetApplicantAttributes"
import {DataGrid} from "@mui/x-data-grid"

const StyledChip = styled(Chip)`
    margin: 4px;
`

const StyledNull = eStyled.div`
    display: block;
    padding-left: 18px;
`

export interface ListApprovalsProps {
    electionEventId: string
    electionId?: string
    onViewApproval: (id: Identifier) => void
    electionEventRecord: Sequent_Backend_Election_Event
}

interface ApprovalsListProps extends Omit<DatagridConfigurableProps, "children"> {
    omit: string[]
    actions: Action[]
    t: TFunction
    electionEventId: string
    userAttributes: GetUserProfileAttributesQuery | undefined
    tenantId: string | null
}

// Storage key for the status filter
const STATUS_FILTER_KEY = "approvals_status_filter"

const ApprovalsList = (props: ApprovalsListProps) => {
    const {filterValues, data, isLoading} = useListContext()

    const userBasicInfo = ["first_name", "last_name", "email", "username", "dateOfBirth"]
    const listFields = useMemo(() => {
        const basicInfoFields: UserProfileAttribute[] = []
        const attributesFields: UserProfileAttribute[] = []
        const omitFields: string[] = []

        props.userAttributes?.get_user_profile_attributes.forEach((attr) => {
            if (attr.name && userBasicInfo.includes(attr.name)) {
                basicInfoFields.push(attr)
            } else {
                omitFields.push(`applicant_data['${attr.name}']`)
                attributesFields.push(attr)
            }
        })

        return {basicInfoFields, attributesFields, omitFields}
    }, [props.userAttributes?.get_user_profile_attributes])

    useEffect(() => {
        console.log("omitted fields", listFields.omitFields)
    }, [listFields])

    // Monitor and save filter changes
    useEffect(() => {
        if (filterValues?.status) {
            localStorage.setItem(STATUS_FILTER_KEY, filterValues.status)
        }
    }, [filterValues?.status])

    const fetchApplicantAttributes = async (applicationId: string) => {}

    const RenderUserFields = (fields: UserProfileAttribute[]) => {
        console.log("fields", fields)
        return fields.map((attr) => {
            const attrMappedName = convertToCamelCase(getAttributeLabel(attr.name ?? ""))
            if (attr.multivalued) {
                return (
                    <ReferenceManyField
                        key={attr.name}
                        label={getAttributeLabel(attr.display_name ?? "")}
                        reference="sequent_backend_applicant_attributes"
                        target={"application_id"}
                    >
                        <WithListContext
                            render={(data) => {
                                const attribute = data.data.find((item: any) => {
                                    return item.applicant_attribute_name === attrMappedName
                                })
                                if (attribute) {
                                    const value = attribute.applicant_attribute_value
                                    const splitValue = value ? (value.split(";") as string[]) : null
                                    return (
                                        <>
                                            {splitValue ? (
                                                splitValue.map((item: any, index: number) => (
                                                    <StyledChip key={index} label={item} />
                                                ))
                                            ) : (
                                                <Typography variant="body1">-</Typography>
                                            )}
                                        </>
                                    )
                                } else {
                                    return <Typography variant="body1">-</Typography>
                                }
                            }}
                        />
                    </ReferenceManyField>
                )
            }
            return (
                <ReferenceManyField
                    key={attr.name}
                    label={getAttributeLabel(attr.display_name ?? "")}
                    reference="sequent_backend_applicant_attributes"
                    target={"application_id"}
                >
                    <WithListContext
                        render={(data) => {
                            console.log("data", data)
                            const attribute = data.data.find((item: any) => {
                                return item.applicant_attribute_name === attrMappedName
                            })
                            if (attribute) {
                                return (
                                    <>
                                        <Typography variant="body1">
                                            {attribute.applicant_attribute_value}
                                        </Typography>
                                    </>
                                )
                            } else {
                                return (
                                    <>
                                        <Typography variant="body1">-</Typography>
                                    </>
                                )
                            }
                        }}
                    />
                </ReferenceManyField>
            )
        })
    }

    return (
        <div>
            <DatagridConfigurable {...props} omit={listFields.omitFields} bulkActionButtons={<></>}>
                <TextField source="id" />
                <DateField showTime source="created_at" />
                <DateField showTime source="updated_at" />
                <TextField source="applicant_id" />
                <TextField source="verification_type" />
                <FunctionField
                    label={props.t("approvalsScreen.column.status")}
                    render={(record: any) => (
                        <StatusApplicationChip status={record.status.toUpperCase()} />
                    )}
                />
                {RenderUserFields(listFields.basicInfoFields)}
                {RenderUserFields(listFields.attributesFields)}
                <ActionsColumn actions={props.actions} label={props.t("common.label.actions")} />
            </DatagridConfigurable>
        </div>
    )
}

const CustomFilters = (userProfileAttribute: GetUserProfileAttributesQuery | undefined) => {
    const {t} = useTranslation()
    const dynamicFilters = userProfileAttribute?.get_user_profile_attributes?.map((attr) => {
        // Assume applicant_data is the JSONB field
        const source = `applicant_data.${attr.name}`

        return (
            <TextInput
                key={attr.name}
                source={source}
                label={getAttributeLabel(attr.display_name ?? "")}
            />
        )
    })
    return [
        <SelectInput
            source="status"
            key="status_filter"
            label={t("approvalsScreen.column.status")}
            choices={[
                {id: "pending", name: "Pending"},
                {id: "accepted", name: "Accepted"},
                {id: "rejected", name: "Rejected"},
            ]}
        />,
        <SelectInput
            source="verification_type"
            key="verification_type_filter"
            label={t("approvalsScreen.column.verificationType")}
            choices={[
                {id: "MANUAL", name: "Manual"},
                {id: "AUTOMATIC", name: "Automatic"},
            ]}
        />,
        <TextInput
            key={"applicant_id_filter"}
            source="applicant_id"
            label={t("approvalsScreen.column.applicantId")}
        />,
        <TextInput key={"id_filter"} source="id" label={t("approvalsScreen.column.id")} />,
        ...(dynamicFilters ?? []),
    ]
}

export const ListApprovals: React.FC<ListApprovalsProps> = ({
    electionEventId,
    electionId,
    onViewApproval,
    electionEventRecord,
}) => {
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const OMIT_FIELDS: string[] = []
    const {data: userAttributes} = useQuery<GetUserProfileAttributesQuery>(
        USER_PROFILE_ATTRIBUTES,
        {
            variables: {
                tenantId: tenantId,
                electionEventId: electionEventId,
            },
        }
    )
    console.log("userAttributes", userAttributes)
    const actions: Action[] = [
        {
            icon: <Visibility />,
            action: onViewApproval,
        },
    ]

    // Get initial status from localStorage or use "pending" as default
    const initialStatus = localStorage.getItem(STATUS_FILTER_KEY) || "pending"

    return (
        <List
            actions={<ListActions withImport={false} withExport={false} />}
            resource="sequent_backend_applications"
            filters={CustomFilters(userAttributes)}
            filter={{election_event_id: electionEventId || undefined}}
            sort={{field: "created_at", order: "DESC"}}
            perPage={10}
            filterDefaultValues={{status: initialStatus}}
            disableSyncWithLocation
            storeKey="approvals-list"
        >
            <ApprovalsList
                omit={OMIT_FIELDS}
                actions={actions}
                t={t}
                electionEventId={electionEventId}
                userAttributes={userAttributes}
                tenantId={tenantId}
            />
        </List>
    )
}
