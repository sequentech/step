// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {PropsWithChildren, ReactElement} from "react"
import {
    BooleanField,
    DatagridConfigurable,
    FunctionField,
    List,
    ReferenceField,
    ReferenceManyField,
    TextField,
    TextInput,
} from "react-admin"
import {ListActions} from "../../components/ListActions"
import {ChipList} from "../../components/ChipList"
import {Typography} from "@mui/material"
import {generateRowClickHandler} from "../../services/RowClickService"
import {useTenantStore} from "../../providers/TenantContextProvider"
import {electionSearchQuery, electionSearchText} from "../../services/ElectionSearch"
import {translateFromPresentation} from "@sequentech/ui-core"
import {useTranslation} from "react-i18next"

const OMIT_FIELDS = ["id", "is_consolidated_ballot_encoding", "spoil_ballot_option"]

const Filters: Array<ReactElement> = [
    <TextInput
        label="Name"
        source="_or"
        key={0}
        parse={electionSearchQuery}
        format={electionSearchText}
    />,
    <TextInput label="Description" source="description" key={1} />,
    <TextInput label="ID" source="id" key={2} />,
    <TextInput label="Election Event ID" source="election_event_id" key={3} />,
]

export interface ListElectionProps {
    aside?: ReactElement
}

/** The record's name in the interface language, from its presentation. */
const PresentationName: React.FC<{label?: string}> = () => {
    const {i18n} = useTranslation()
    return (
        <FunctionField
            render={(record) =>
                translateFromPresentation(record, "name", i18n.language, {
                    defaultLanguageCode: "en",
                })
            }
        />
    )
}

export const ListElection: React.FC<ListElectionProps & PropsWithChildren> = ({aside}) => {
    const [tenantId] = useTenantStore()
    const [openDrawer, setOpenDrawer] = React.useState<boolean>(false)

    const rowClickHandler = generateRowClickHandler(["election_event_id"])

    return (
        <>
            <Typography variant="h5">Elections</Typography>
            <List
                actions={
                    <ListActions open={openDrawer} setOpen={setOpenDrawer} withFilter={true} />
                }
                filter={{tenant_id: tenantId || undefined}}
                aside={aside}
                filters={Filters}
            >
                <DatagridConfigurable rowClick={rowClickHandler} omit={OMIT_FIELDS}>
                    <TextField source="id" />
                    <PresentationName label="Name" />
                    <TextField source="description" />
                    <BooleanField source="is_consolidated_ballot_encoding" />
                    <BooleanField source="spoil_ballot_option" />
                    <ReferenceManyField
                        label="Contests"
                        reference="sequent_backend_contest"
                        target="election_id"
                    >
                        <ChipList
                            source="sequent_backend_contest"
                            filterFields={["election_event_id", "election_id"]}
                        />
                    </ReferenceManyField>
                    <ReferenceField
                        source="election_event_id"
                        reference="sequent_backend_election_event"
                    >
                        <PresentationName />
                    </ReferenceField>
                </DatagridConfigurable>
            </List>
        </>
    )
}
