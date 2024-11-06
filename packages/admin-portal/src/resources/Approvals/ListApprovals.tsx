// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement, useEffect} from "react"
import {
    List,
    DateField,
    FunctionField,
    TextField,
    DatagridConfigurable,
    Identifier,
    SelectInput,
    useListController,
    TextInput,
} from "react-admin"
import {useTranslation} from "react-i18next"
import {Visibility} from "@mui/icons-material"
import {Action, ActionsColumn} from "@/components/ActionButons"
import {ListActions} from "@/components/ListActions"
import {Sequent_Backend_Election_Event} from "@/gql/graphql"
import {StatusApplicationChip} from "@/components/StatusApplicationChip"

export interface ListApprovalsProps {
    electionEventId: string
    electionId?: string
    onViewApproval: (id: Identifier) => void
    electionEventRecord: Sequent_Backend_Election_Event
}

const ApprovalsList = (props: any) => {
    const listContext = useListController(props)
    const {setFilters, filterValues} = listContext

    useEffect(() => {
        // Set the status filter to "pending" when the component mounts
        if (!filterValues?.status) {
            setFilters({...filterValues, status: "pending"}, {})
        }
    }, [])

    return (
        <div>
            <DatagridConfigurable {...props} omit={props.omit} bulkActionButtons={<></>}>
                <TextField source="id" />
                <DateField showTime source="created_at" />
                <DateField showTime source="updated_at" />
                <TextField source="applicant_id" />
                <TextField source="verification_type" />
                <FunctionField
                    label={props.t("approvalsScreen.column.status")}
                    render={(record: any) => <StatusApplicationChip status={record.status} />}
                />
                <ActionsColumn actions={props.actions} label={props.t("common.label.actions")} />
            </DatagridConfigurable>
        </div>
    )
}

export const ListApprovals: React.FC<ListApprovalsProps> = ({
    electionEventId,
    electionId,
    onViewApproval,
    electionEventRecord,
}) => {
    const {t} = useTranslation()

    const OMIT_FIELDS: string[] = []

    const filters: Array<ReactElement> = [
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

    const actions: Action[] = [
        {
            icon: <Visibility />,
            action: onViewApproval,
        },
    ]

    return (
        <List
            actions={<ListActions withImport={false} withExport={false} />}
            resource="sequent_backend_applications"
            filters={filters}
            filter={{election_event_id: electionEventRecord?.id || undefined}}
            sort={{field: "created_at", order: "DESC"}}
            perPage={10}
            filterDefaultValues={{status: "pending"}}
            disableSyncWithLocation
        >
            {/* <ResetFilters /> */}
            <ApprovalsList omit={OMIT_FIELDS} actions={actions} t={t} />
        </List>
    )
}
