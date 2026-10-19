// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement, useContext, useState} from "react"
import {
    DatagridConfigurable,
    FunctionField,
    List,
    TextField,
    useListContext,
    useRecordContext,
} from "react-admin"
import {Box, Button, Chip, IconButton, Menu, MenuItem, Stack, Typography} from "@mui/material"
import MoreVertIcon from "@mui/icons-material/MoreVert"
import {useTranslation} from "react-i18next"
import {
    ENetworkDataPolicy,
    type IElectionEventPresentation,
    type IForensicAddress,
} from "@sequentech/ui-core"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {AuthContext} from "@/providers/AuthContextProvider"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {useZonedEvent} from "@/providers/EventTimeZoneProvider"
import SelectElection from "@/components/election/SelectElection"
import {COUNTRIES} from "@/lib/countries"
import {FormStyles} from "@/components/styles/FormStyles"
import {ForensicExportActions} from "./ForensicExportActions"
import {Sequent_Backend_Election} from "@/gql/graphql"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"
import {useContainerWidth} from "@/hooks/useContainerWidth"
import {AdminDateTime} from "@/components/AdminDateTime"
import {IPermissions} from "@/types/keycloak"
import {ForensicAddressDialog} from "./ForensicAddressDialog"

export interface ListIpAddressProps {
    aside?: ReactElement
}
export interface RecordVoteCloudflareData {
    ip?: string
    country?: string
}

const Empty: React.FC = () => {
    const {t} = useTranslation()
    return <Typography sx={{p: 2}}>{t("dashboard.ipAddress.emptyState")}</Typography>
}

const SelectedForensicAddress: React.FC<{
    electionEventId: string
    electionId?: string
    selected?: IForensicAddress
    onClose: () => void
}> = ({electionEventId, electionId, selected, onClose}) => {
    const {filterValues} = useListContext()
    const effectiveElectionId = electionId ?? filterValues.election_id
    return selected ? (
        <ForensicAddressDialog
            key={`${effectiveElectionId ?? "all"}:${selected.ip}`}
            electionEventId={electionEventId}
            electionId={effectiveElectionId}
            ip={selected.ip}
            onClose={onClose}
        />
    ) : null
}

const AddressActions: React.FC<{row: IForensicAddress; onOpen: () => void}> = ({row, onOpen}) => {
    const {t} = useTranslation()
    const [anchor, setAnchor] = useState<HTMLElement | null>(null)
    const [copyError, setCopyError] = useState(false)
    const copy = async () => {
        setAnchor(null)
        try {
            await navigator.clipboard.writeText(row.ip)
            setCopyError(false)
        } catch {
            setCopyError(true)
        }
    }
    return (
        <>
            <IconButton
                aria-label={t("common.label.actions")}
                onClick={(event) => {
                    event.stopPropagation()
                    setAnchor(event.currentTarget)
                }}
            >
                <MoreVertIcon />
            </IconButton>
            <Menu anchorEl={anchor} open={!!anchor} onClose={() => setAnchor(null)}>
                <MenuItem
                    onClick={() => {
                        setAnchor(null)
                        onOpen()
                    }}
                >
                    {t("ballotPrivacy.forensic.view")}
                </MenuItem>
                <MenuItem onClick={() => void copy()}>{t("ballotPrivacy.forensic.copy")}</MenuItem>
            </Menu>
            {copyError && (
                <Typography variant="caption" color="error">
                    {row.ip}
                </Typography>
            )}
        </>
    )
}

export const ListIpAddress: React.FC<ListIpAddressProps> = ({aside}) => {
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const {globalSettings} = useContext(SettingsContext)
    const auth = useContext(AuthContext)
    const aliasRenderer = useAliasRenderer()
    const record = useRecordContext<Sequent_Backend_Election>()
    const event = useZonedEvent()
    const electionEventId = record?.election_event_id ?? record?.id
    const electionId = record?.election_event_id ? record.id : undefined
    const presentation = (
        record?.election_event_id ? event?.presentation : record?.presentation
    ) as IElectionEventPresentation | undefined
    const [selected, setSelected] = useState<IForensicAddress>()
    const [ref, width] = useContainerWidth<HTMLDivElement>()
    const compact = width !== null && width < 1000
    const canRead = auth.isAuthorized(true, tenantId, IPermissions.FORENSIC_READ)
    if (
        !electionEventId ||
        !canRead ||
        presentation?.network_data_policy !== ENetworkDataPolicy.FORENSIC_STORE
    )
        return null
    const preview = (row: IForensicAddress) => (
        <Stack direction="row" useFlexGap flexWrap="wrap" gap={0.5}>
            {(row.pseudonyms ?? []).slice(0, compact ? 1 : 3).map((pseudonym) => (
                <Chip key={pseudonym} size="small" label={pseudonym} />
            ))}
            {(row.pseudonym_count ?? row.pseudonyms?.length) > (compact ? 1 : 3) && (
                <Typography variant="caption">
                    {t("ballotPrivacy.forensic.more", {
                        count: (row.pseudonym_count ?? row.pseudonyms.length) - (compact ? 1 : 3),
                    })}
                </Typography>
            )}
        </Stack>
    )
    return (
        <Box ref={ref} className="print-hidden" sx={{minWidth: 0, maxWidth: "100%"}}>
            <Typography variant="h4">{t("dashboard.ipAddress.title")}</Typography>
            <Typography variant="body2" sx={{my: 1}}>
                {t("ballotPrivacy.forensic.subtitle")}
            </Typography>
            <Chip size="small" label={t("ballotPrivacy.forensic.store")} />
            <List
                resource="ip_address"
                queryOptions={{refetchInterval: globalSettings.QUERY_POLL_INTERVAL_MS}}
                filter={{
                    tenant_id: tenantId,
                    election_event_id: electionEventId,
                    ...(electionId ? {election_id: electionId} : {}),
                }}
                storeKey={false}
                aside={aside}
                empty={<Empty />}
                filters={[
                    <FormStyles.TextInput
                        key="ip"
                        source="ip"
                        label={t("dashboard.ipAddress.ip")}
                    />,
                    <FormStyles.AutocompleteInput
                        key="country"
                        source="country"
                        label={t("dashboard.ipAddress.country")}
                        choices={COUNTRIES}
                        optionValue="code"
                    />,
                    ...(!electionId
                        ? [
                              <SelectElection
                                  key="election"
                                  source="election_id"
                                  label={t("dashboard.ipAddress.ElectionName")}
                                  tenantId={tenantId}
                                  electionEventId={electionEventId}
                              />,
                          ]
                        : []),
                ]}
                actions={
                    <ForensicExportActions
                        electionEventId={electionEventId}
                        electionId={electionId}
                    />
                }
                sx={{
                    "& .RaFilterForm-form": {flexWrap: "wrap"},
                    "& .RaList-actions": {flexWrap: "wrap"},
                    "& .RaDatagrid-table": {tableLayout: "fixed"},
                    "& .RaDatagrid-rowCell": {overflowWrap: "anywhere"},
                }}
            >
                <DatagridConfigurable
                    key={compact ? "compact" : "table"}
                    omit={compact ? [] : ["first_ballot"]}
                    bulkActionButtons={
                        <ForensicExportActions
                            electionEventId={electionEventId}
                            electionId={electionId}
                            selectedOnly
                        />
                    }
                >
                    {compact ? (
                        <FunctionField
                            source="ip"
                            sortable={false}
                            label={t("dashboard.ipAddress.ip")}
                            render={(row: IForensicAddress) => (
                                <Stack spacing={0.5}>
                                    <Stack
                                        direction="row"
                                        useFlexGap
                                        flexWrap="wrap"
                                        alignItems="center"
                                        gap={1}
                                    >
                                        <Button sx={{p: 0}} onClick={() => setSelected(row)}>
                                            {row.ip}
                                        </Button>
                                        {row.shared && (
                                            <Chip
                                                size="small"
                                                label={t("ballotPrivacy.forensic.shared")}
                                            />
                                        )}
                                        <Typography variant="body2">
                                            {t("ballotPrivacy.ballots", {
                                                count: row.vote_count,
                                            })}
                                        </Typography>
                                    </Stack>
                                    <Typography variant="caption">
                                        {row.country ?? "—"} ·{" "}
                                        <AdminDateTime
                                            value={row.last_ballot}
                                            event={electionEventId}
                                            seconds
                                        />
                                    </Typography>
                                    {preview(row)}
                                </Stack>
                            )}
                        />
                    ) : (
                        [
                            <FunctionField
                                source="ip"
                                sortable={false}
                                label={t("dashboard.ipAddress.ip")}
                                render={(row: IForensicAddress) => (
                                    <>
                                        <Button
                                            sx={{p: 0, overflowWrap: "anywhere"}}
                                            onClick={() => setSelected(row)}
                                        >
                                            {row.ip}
                                        </Button>
                                        {row.shared && (
                                            <Chip
                                                size="small"
                                                title={t("ballotPrivacy.forensic.sharedHelp")}
                                                label={t("ballotPrivacy.forensic.shared")}
                                            />
                                        )}
                                    </>
                                )}
                            />,
                            <TextField
                                emptyText="—"
                                source="country"
                                sortable={false}
                                label={t("dashboard.ipAddress.country")}
                            />,
                            <TextField
                                source="vote_count"
                                sortable={false}
                                label={t("dashboard.ipAddress.VoteCount")}
                            />,
                            <FunctionField
                                source="election_presentation"
                                sortable={false}
                                label={t("dashboard.ipAddress.ElectionName")}
                                render={(row: IForensicAddress) =>
                                    row.election_presentation
                                        ? aliasRenderer(row.election_presentation)
                                        : t("ballotPrivacy.forensic.allElections")
                                }
                            />,
                            <FunctionField
                                source="pseudonyms"
                                sortable={false}
                                label={t("ballotPrivacy.forensic.pseudonyms")}
                                render={preview}
                            />,
                            <FunctionField
                                source="first_ballot"
                                sortable={false}
                                label={t("ballotPrivacy.forensic.first")}
                                render={(row: IForensicAddress) => (
                                    <AdminDateTime
                                        value={row.first_ballot}
                                        event={electionEventId}
                                        seconds
                                    />
                                )}
                            />,
                            <FunctionField
                                source="last_ballot"
                                sortable={false}
                                label={t("ballotPrivacy.forensic.last")}
                                render={(row: IForensicAddress) => (
                                    <AdminDateTime
                                        value={row.last_ballot}
                                        event={electionEventId}
                                        seconds
                                    />
                                )}
                            />,
                        ]
                    )}
                    <FunctionField
                        source="actions"
                        sortable={false}
                        label={t("common.label.actions")}
                        render={(row: IForensicAddress) => (
                            <AddressActions row={row} onOpen={() => setSelected(row)} />
                        )}
                    />
                </DatagridConfigurable>
                <SelectedForensicAddress
                    electionEventId={electionEventId}
                    electionId={electionId}
                    selected={selected}
                    onClose={() => setSelected(undefined)}
                />
            </List>
        </Box>
    )
}
