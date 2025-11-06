// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {ReactElement, useContext, useMemo} from "react"
import {DatagridConfigurable, FunctionField, List, TextField, useSidebarState} from "react-admin"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {Box, Typography} from "@mui/material"
import {styled} from "@mui/material/styles"
import {useTranslation} from "react-i18next"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import SelectElection from "@/components/election/SelectElection"
import {COUNTRIES} from "@/lib/countries"
import {FormStyles} from "@/components/styles/FormStyles"
import {ListActions} from "@/components/ListActions"

const ListStyle = styled(List)`
    button.RaFilterFormInput-hideButton {
        margin-bottom: 26px !important;
    }
`

export interface ListIpAddressProps {
    aside?: ReactElement
    electionEventId?: string
    electionId?: string
}

export interface RecordVoteCloudflareData {
    ip?: string
    country?: string
}

export const ListIpAddress: React.FC<ListIpAddressProps> = ({
    aside,
    electionEventId,
    electionId,
}) => {
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const {globalSettings} = useContext(SettingsContext)

    const Filters = useMemo(
        () => [
            <FormStyles.TextInput key="ip" source="ip" label={`${t(`dashboard.ipAddress.ip`)}`} />,
            <FormStyles.AutocompleteInput
                key="country"
                source="country"
                label={`${t(`dashboard.ipAddress.country`)}`}
                choices={COUNTRIES}
                optionValue="code"
                fullWidth
            />,
            <SelectElection
                key="election"
                source="election_id"
                label={`${t(`dashboard.ipAddress.ElectionName`)}`}
                tenantId={tenantId}
                electionEventId={electionEventId}
            />,
        ],
        []
    )

    const Empty = () => (
        <ResourceListStyles.EmptyBox style={{margin: "8px"}}>
            <Typography variant="h4" paragraph>
                {t(`dashboard.ipAddress.emptyState`)}
            </Typography>
        </ResourceListStyles.EmptyBox>
    )

    const filters = () => {
        const filters: any = {
            tenant_id: tenantId,
        }

        if (electionEventId) {
            filters["election_event_id"] = electionEventId
        }

        if (electionId) {
            filters["election_id"] = electionId
        }
        return filters
    }

    return (
        <Box className="print-hidden">
            <Typography variant="h4">{t(`dashboard.ipAddress.title`)}</Typography>
            <ListStyle
                resource="ip_address"
                queryOptions={{
                    refetchInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
                }}
                empty={<Empty />}
                filter={filters()}
                storeKey={false}
                aside={aside}
                filters={Filters}
                actions={<ListActions withImport={false} defaultExport />}
            >
                <DatagridConfigurable omit={["voters_id"]}>
                    <FunctionField
                        source="ip"
                        sortable={false}
                        label={`${t(`dashboard.ipAddress.ip`)}`}
                        render={(record: RecordVoteCloudflareData) => (record.ip ? record.ip : "-")}
                    />
                    <FunctionField
                        source="country"
                        sortable={false}
                        label={`${t(`dashboard.ipAddress.country`)}`}
                        render={(record: RecordVoteCloudflareData) =>
                            record.country ? record.country : "-"
                        }
                    />
                    <TextField
                        source="vote_count"
                        sortable={false}
                        label={`${t(`dashboard.ipAddress.VoteCount`)}`}
                    />
                    <TextField
                        source="election_name"
                        sortable={false}
                        label={`${t(`dashboard.ipAddress.ElectionName`)}`}
                    />
                    <TextField
                        source="voters_id"
                        sortable={false}
                        label={`${t(`dashboard.ipAddress.VotersId`)}`}
                    />
                </DatagridConfigurable>
            </ListStyle>
        </Box>
    )
}
