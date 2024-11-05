// SPDX-FileCopyrightText: 2024 Félix Robles <dev@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement} from "react"
import {
    List,
    DateField,
    FunctionField,
    TextField,
    DatagridConfigurable,
    Identifier,
    SelectInput,
} from "react-admin"
import {useTranslation} from "react-i18next"
import {Visibility} from "@mui/icons-material"
import {Action, ActionsColumn} from "@/components/ActionButons"
import {ListActions} from "@/components/ListActions"
import {StatusChip} from "@/components/StatusChip"
import {Sequent_Backend_Election_Event} from "@/gql/graphql"

export interface ListApprovalsProps {
    electionEventId: string
    electionId?: string
    onViewApproval: (id: Identifier) => void
    electionEventRecord: Sequent_Backend_Election_Event
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
            alwaysOn
        />,
    ]

    const actions: Action[] = [
        {
            icon: <Visibility />,
            action: onViewApproval,
        },
    ]

    return (
        <>
            <List
                actions={<ListActions withImport={false} withExport={false} />}
                resource="sequent_backend_applications"
                filters={filters}
                filter={{election_event_id: electionEventRecord?.id || undefined}}
                // storeKey={false}
                sort={{field: "created_at", order: "DESC"}}
                perPage={10}
                filterDefaultValues={{status: "pending"}}
            >
                <DatagridConfigurable omit={OMIT_FIELDS} bulkActionButtons={<></>}>
                    <TextField source="id" />
                    <DateField source="created_at" />
                    <DateField source="updated_at" />
                    <TextField source="applicant_id" />
                    <TextField source="verification_type" />
                    <FunctionField
                        label={t("approvalsScreen.column.status")}
                        render={(record: any) => <StatusChip status={record.status} />}
                    />
                    <ActionsColumn actions={actions} label={t("common.label.actions")} />
                </DatagridConfigurable>
            </List>
        </>
    )
}
