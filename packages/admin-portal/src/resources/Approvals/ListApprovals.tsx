// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useMemo} from "react"
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
    useSidebarState,
} from "react-admin"
import {TFunction, useTranslation} from "react-i18next"
import {Visibility} from "@mui/icons-material"
import {Action, ActionsColumn} from "@/components/ActionButons"
import {ListActions} from "@/components/ListActions"
import {GetUserProfileAttributesQuery, Sequent_Backend_Applications, Sequent_Backend_Election_Event, UserProfileAttribute} from "@/gql/graphql"
import {StatusApplicationChip} from "@/components/StatusApplicationChip"
import { useTenantStore } from "@/providers/TenantContextProvider"
import { useQuery } from "@apollo/client"
import { USER_PROFILE_ATTRIBUTES } from "@/queries/GetUserProfileAttributes"
import {styled} from "@mui/material/styles"
import eStyled from "@emotion/styled"
import {Chip, Typography} from "@mui/material"
import { convertToCamelCase } from "./UtilsApprovals"
import { getAttributeLabel } from "@/services/UserService"

const StyledChip = styled(Chip)`
    margin: 4px;
`

const StyledNull = eStyled.div`
    display: block;
    padding-left: 18px;
`

const DataGridContainerStyle = styled(DatagridConfigurable)<{isOpenSideBar?: boolean}>`
    @media (min-width: ${({theme}) => theme.breakpoints.values.md}px) {
        overflow-x: auto;
        width: 100%;
        ${({isOpenSideBar}) =>
            `max-width: ${isOpenSideBar ? "calc(100vw - 355px)" : "calc(100vw - 108px)"};`}
        &  > div:first-child {
            position: absolute;
            width: 100%;
        }
    }
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
    userAttributes: GetUserProfileAttributesQuery | undefined
}

// Storage key for the status filter
const STATUS_FILTER_KEY = "approvals_status_filter"

const ApprovalsList = (props: ApprovalsListProps) => {
    const {filterValues, data, isLoading} = useListContext()
    const [isOpenSidebar] = useSidebarState()
    const userBasicInfo = ["first_name", "last_name", "email", "username", "dateOfBirth"]
    const listFields = useMemo(() => {
        const basicInfoFields: UserProfileAttribute[] = []
        const attributesFields: UserProfileAttribute[] = []
        const omitFields: string[] = []

        props.userAttributes?.get_user_profile_attributes.forEach((attr) => {
            if (attr.name && userBasicInfo.includes(attr.name)) {
                basicInfoFields.push(attr)
            } else {
                omitFields.push(
                    `applicant_data[${convertToCamelCase(getAttributeLabel(attr.name ?? ""))}]`
                )
                attributesFields.push(attr)
            }
        })

        return {basicInfoFields, attributesFields, omitFields}
    }, [props.userAttributes?.get_user_profile_attributes])

    const renderUserFields = (fields: UserProfileAttribute[]) =>
        fields.map((attr) => {
            const attrMappedName = convertToCamelCase(getAttributeLabel(attr.name ?? ""))
            if (attr.annotations?.inputType === "html5-date") {
                return (
                    <FunctionField
                        key={attr.name}
                        source={`applicant_data['${attr.name}']`}
                        label={getAttributeLabel(attr.display_name ?? "")}
                        render={(
                            record: Sequent_Backend_Applications,
                            source: string | undefined
                        ) => {
                            const dateValue = record?.applicant_data[attrMappedName]
                            try {
                                const date = new Date(dateValue)
                                if (isNaN(date.getTime())) {
                                    throw new Error("Invalid date")
                                }
                                return <span>{date.toLocaleDateString()}</span>
                            } catch {
                                return <span>-</span>
                            }
                        }}
                    />
                )
            } else if (attr.multivalued) {
                return (
                    <FunctionField
                        key={attr.name}
                        source={`applicant_data[${attrMappedName}]`}
                        label={getAttributeLabel(attr.display_name ?? "")}
                        render={(record: Sequent_Backend_Applications) => {
                            let value = record?.applicant_data[attrMappedName]
                            let values = value ? value.split(";") : []
                            return (
                                <>
                                    {values ? (
                                        values.map((item: any, index: number) => (
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
                    <FunctionField
                        key={attr.name}
                        source={`applicant_data[${attrMappedName}]`}
                        label={getAttributeLabel(attr.display_name ?? "")}
                        render={(record: Sequent_Backend_Applications) => {
                            const attribute_value = record?.applicant_data[attrMappedName]
                            if (attribute_value) {
                                return String(attribute_value)
                            }
                            return <Typography>-</Typography>
                        }}
                    />
                )
            } else {
                return null
            }
        })

    const sx = {
        "@media (min-width: 960px)": {
            "overflowX": "auto",
            "width": "100%",
            "maxWidth": isOpenSidebar ? "calc(100vw - 355px)" : "calc(100vw - 108px)",
            "& > div:first-of-type": {
                position: "absolute",
                width: "100%",
            },
        },
    }

    // Monitor and save filter changes
    useEffect(() => {
        if (filterValues?.status) {
            localStorage.setItem(STATUS_FILTER_KEY, filterValues.status)
        }
    }, [filterValues?.status])

    return (
        <div>
        <DatagridConfigurable
                sx={sx}
                {...props}
                omit={listFields.omitFields}
                bulkActionButtons={<></>}
            >
                <TextField source="id" />
                <DateField showTime source="created_at" />
                <DateField showTime source="updated_at" />
                <FunctionField 
                source="applicant_id"
                render={(record: Sequent_Backend_Applications) => {
                    if (record.applicant_id && record.applicant_id != "null") {
                        return record.applicant_id
                    } else {
                        return "-"
                    }
                }}
                />
                <TextField source="verification_type" />
                <FunctionField
                    label={props.t("approvalsScreen.column.status")}
                    render={(record: any) => (
                        <StatusApplicationChip status={record.status.toUpperCase()} />
                    )}
                />
                {renderUserFields(listFields.basicInfoFields)}
                {renderUserFields(listFields.attributesFields)}
                <ActionsColumn actions={props.actions} label={props.t("common.label.actions")} />
            </DatagridConfigurable>
        </div>
    )
}

const CustomFilters = () => {
    const {t} = useTranslation()

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
    ]
}

export const ListApprovals: React.FC<ListApprovalsProps> = ({
    electionEventId,
    electionId,
    onViewApproval,
    electionEventRecord,
}) => {
    const {t} = useTranslation()
    const OMIT_FIELDS: string[] = []

    const actions: Action[] = [
        {
            icon: <Visibility />,
            action: onViewApproval,
        },
    ]

    const [tenantId] = useTenantStore()
    const {data: userAttributes} = useQuery<GetUserProfileAttributesQuery>(
        USER_PROFILE_ATTRIBUTES,
        {
            variables: {
                tenantId: tenantId,
                electionEventId: electionEventId,
            },
        }
    )

    // Get initial status from localStorage or use "pending" as default
    const initialStatus = localStorage.getItem(STATUS_FILTER_KEY) || "pending"

    return (
        <List
            actions={<ListActions withImport={false} withExport={false} />}
            resource="sequent_backend_applications"
            filters={CustomFilters()}
            filter={{election_event_id: electionEventId || undefined}}
            sort={{field: "created_at", order: "DESC"}}
            perPage={10}
            filterDefaultValues={{status: initialStatus}}
            disableSyncWithLocation
            storeKey="approvals-list"
        >
            <ApprovalsList omit={OMIT_FIELDS} actions={actions} t={t} userAttributes={userAttributes}/>
        </List>
    )
}
