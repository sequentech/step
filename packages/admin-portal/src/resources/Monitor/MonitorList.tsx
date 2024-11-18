// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {ReactElement, useContext} from "react"
import {List, TextInput} from "react-admin"
import {Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {ResetFilters} from "@/components/ResetFilters"
import {ListActions} from "@/components/ListActions"

export interface MonitorListProps {
    aside?: ReactElement
    electionEventId?: string
    electionId?: string
}

export const MonitorList: React.FC<MonitorListProps> = ({aside}) => {
    const {t, i18n} = useTranslation()
    const {globalSettings} = useContext(SettingsContext)

    const Empty = () => (
        <ResourceListStyles.EmptyBox>
            <Typography variant="h4" paragraph>
                {t("monitor.empty.header")}
            </Typography>
        </ResourceListStyles.EmptyBox>
    )

    const Filters: Array<ReactElement> = [
        // TODO: fix sources
        <TextInput label="Country Name" source="name" key={0} />,
        <TextInput label="Country Code" source="description" key={1} />,
        <TextInput label="Post Date and Time" source="id" key={2} />,
        <TextInput label="Is Voting Started?" source="type" key={3} />,
        <TextInput label="Is Voting Closed?" source="type" key={4} />,
        <TextInput label="Generate ER?" source="type" key={5} />,
        <TextInput label="Transmitted" source="type" key={6} />,
    ]

    return (
        <>
            {
                <List
                    // TODO: fix resource
                    resource="user"
                    queryOptions={{
                        refetchInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
                    }}
                    empty={<Empty />}
                    actions={<ListActions withImport={false} withExport={false} />}
                    aside={aside}
                    filters={Filters}
                    disableSyncWithLocation
                >
                    <ResetFilters />
                    {/* TODO: add table content */}
                </List>
            }
        </>
    )
}
