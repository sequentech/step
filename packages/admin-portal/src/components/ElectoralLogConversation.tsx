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
    InfiniteList,
} from "react-admin"
import {ListActions} from "@/components/ListActions"
import {Sequent_Backend_Election_Event} from "@/gql/graphql"
import Card from "@mui/material/Card"
import {useTranslation} from "react-i18next"
import ElectionHeader from "./ElectionHeader"

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
    const {t} = useTranslation()

    return (
        <>
            <FunctionField
                source="message"
                render={(record: any) => {
                    const message = record.message
                    const messageObj = JSON.parse(message)

                    let resObj = null
                    const date = new Date(record.created * 1000) // Multiply by 1000 to convert seconds to milliseconds

                    if (messageObj.statement.body["SendCommunications"]) {
                        resObj = [
                            <ConversationRow
                                key="timestamp"
                                rowKey={t(`logsScreen.column.timestamp`)}
                                value={date.toLocaleDateString() as string}
                            />,
                        ]
                        resObj = [
                            ...resObj,
                            ...Object.entries(
                                JSON.parse(messageObj.statement.body["SendCommunications"])
                            ).map(([key, value], index) => {
                                return (
                                    <ConversationRow
                                        key={index}
                                        rowKey={t(`logsScreen.column.${key}`) || t(key)}
                                        value={value as string}
                                    />
                                )
                            }),
                        ]
                    }
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
    showActions = false,
}) => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const filters: Array<ReactElement> = []
    const {t} = useTranslation()

    const filterObject: {[key: string]: any} = {
        election_event_id: record?.id || undefined,
    }

    if (filterToShow) {
        filterObject[filterToShow] = filterValue || undefined
    }

    return (
        <InfiniteList
            perPage={5}
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
            sx={{
                "padding": 0,
                "& .RaList-actions": {
                    display: "none",
                },
            }}
        >
            <SimpleListConfigurable
                linkType={false}
                secondaryText={<></>}
                primaryText={(record: Sequent_Backend_Election_Event) => (
                    <Card
                        component="span"
                        sx={{
                            display: "inline-block",
                            mx: 2,
                            p: 2,
                            width: "40%",
                            minWidth: "250px",
                            position: "relative",
                            borderRadius: "15px 15px 15px 0px",
                            backgroundColor: "#ECFDF5",
                        }}
                    >
                        <ListItem record={record} />
                    </Card>
                )}
            />
        </InfiniteList>
    )
}

const ConversationRow = ({rowKey, value}: {rowKey: string; value: string}) => {
    return (
        <div
            style={{
                display: "flex",
                flexDirection: "column",
                alignItems: "flex-start",
                margin: "4px 0",
            }}
        >
            <div
                style={{
                    backgroundColor: "#fff",
                    borderRadius: "15px 15px 15px 0px",
                    padding: "10px 15px",
                    maxWidth: "95%",
                    boxShadow: "0 1px 2px rgba(0, 0, 0, 0.15)",
                }}
            >
                <div
                    style={{
                        fontWeight: "bold",
                        marginBottom: "4px",
                        color: "#0F054C",
                    }}
                >
                    {rowKey}
                </div>
                <div style={{color: "#303030"}}>{value}</div>
            </div>
        </div>
    )
}
