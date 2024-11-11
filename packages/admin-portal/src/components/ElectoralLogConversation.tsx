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

    console.log("aa RECORD :: ", record)

    return (
        <>
            <FunctionField
                source="message"
                render={(record: any) => {
                    const message = record.message
                    const messageObj = JSON.parse(message)
                    console.log("aa MESSAGE OBJ :: ", messageObj)

                    let resObj = null
                    const date = new Date(record.created * 1000) // Multiply by 1000 to convert seconds to milliseconds

                    if (messageObj.statement.body["SendCommunications"]) {
                        console.log(
                            "aa MESSAGE OBJ SENDCOMM:: ",
                            JSON.parse(messageObj.statement.body["SendCommunications"])
                        )
                        resObj = [
                            <div
                                key="timestamp"
                                style={{
                                    display: "flex",
                                    flexDirection: "row",
                                    justifyContent: "start",
                                    padding: "2px 0",
                                }}
                            >
                                <div
                                    style={{
                                        fontWeight: "bold",
                                        marginRight: 1,
                                        minWidth: "120px",
                                    }}
                                >
                                    {t(`logsScreen.column.timestamp`)}
                                </div>
                                <div>{date.toLocaleDateString() as string}</div>
                            </div>,
                        ]
                        resObj = [
                            ...resObj,
                            ...Object.entries(
                                JSON.parse(messageObj.statement.body["SendCommunications"])
                            ).map(([key, value], index) => {
                                return (
                                    <div
                                        key={index}
                                        style={{
                                            display: "flex",
                                            flexDirection: "row",
                                            justifyContent: "start",
                                            padding: "2px 0",
                                        }}
                                    >
                                        <div
                                            style={{
                                                fontWeight: "bold",
                                                marginRight: 1,
                                                minWidth: "120px",
                                            }}
                                        >
                                            {t(`logsScreen.column.${key}`) || t(key)}
                                        </div>
                                        <div style={{flex: 1}}>{value as string}</div>
                                    </div>
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
    showActions = true,
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
        <>
            <ElectionHeader title={t("logsScreen.conversation")} subtitle={""} />
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
                    padding: 0,
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
                                width: "50%",
                                position: "relative",
                                borderRadius: 4,
                                backgroundColor: "#efe",
                            }}
                        >
                            <ListItem record={record} />
                        </Card>
                    )}
                />
            </InfiniteList>
        </>
    )
}
