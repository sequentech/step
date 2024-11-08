// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement} from "react"
import {
    List,
    FunctionField,
    useRecordContext,
    SingleFieldList,
    SimpleList,
    SimpleListConfigurable,
} from "react-admin"
import {ListActions} from "@/components/ListActions"
import {Sequent_Backend_Election_Event} from "@/gql/graphql"
import Card from "@mui/material/Card"

export interface ElectoralLogListProps {
    aside?: ReactElement
    filterToShow?: ElectoralLogFilters
    filterValue?: string
    showActions?: boolean
}

export enum ElectoralLogFilters {
    ID = "id",
    STATEMENT_KIND = "statement_kind",
    USER_ID = "user_id",
}

type ListItemProps<T> = {
    record: T
}
const ListItem: React.FC<ListItemProps<Sequent_Backend_Election_Event>> = ({
    record,
}: ListItemProps<Sequent_Backend_Election_Event>) => {
    // const record = useRecordContext<Sequent_Backend_Election_Event>()

    console.log("aa RECORD :: ", record)

    return (
        <>
            <FunctionField
                source="message"
                render={(record: any) => {
                    const message = record.message
                    const messageObj = JSON.parse(message)
                    const resObj = Object.entries(
                        JSON.parse(messageObj.statement.body["SendCommunications"])
                    ).map(([key, value], index) => {
                        return (
                            <div key={index} style={{padding: "4px 0"}}>
                                <span style={{fontWeight: "bold"}}>{key}</span>:
                                <span>{value as string}</span>
                            </div>
                        )
                    })
                    return <span style={{display: "block", textAlign: "left"}}>{resObj}</span>
                }}
            />
        </>
    )
}

export const ElectoralLogConversation: React.FC<ElectoralLogListProps> = ({
    aside,
    filterToShow,
    filterValue,
    showActions = true,
}) => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const filters: Array<ReactElement> = []

    const filterObject: {[key: string]: any} = {
        election_event_id: record?.id || undefined,
    }

    if (filterToShow) {
        filterObject[filterToShow] = filterValue || undefined
    }

    return (
        <List
            resource="electoral_log"
            actions={
                showActions && (
                    <ListActions
                        withImport={false}
                        // openExportMenu={(e) => setAnchorEl(e.currentTarget)}
                        withExport={false}
                    />
                )
            }
            filters={filters}
            filter={filterObject}
            storeKey={false}
            sort={{
                field: "id",
                order: "DESC",
            }}
            aside={aside}
            disableSyncWithLocation
        >
            <SimpleListConfigurable
                linkType={false}
                primaryText={(record: Sequent_Backend_Election_Event) => (
                    <Card
                        component="span"
                        sx={{
                            display: "inline-block",
                            m: 2,
                            p: 2,
                            width: "50%",
                            position: "relative",
                            borderRadius: 4,
                        }}
                    >
                        <ListItem record={record} />
                    </Card>
                )}
            />

            {/* <SingleFieldList>
                <Card
                    component="span"
                    sx={{
                        display: "inline-block",
                        m: 2,
                        p: 2,
                        width: "50%",
                        position: "relative",
                        borderRadius: 4,
                    }}
                >
                    <ListItem />
                </Card>
            </SingleFieldList> */}
        </List>
    )
}
