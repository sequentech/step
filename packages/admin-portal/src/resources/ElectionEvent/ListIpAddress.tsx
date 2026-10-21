// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement, useContext, useId, useState} from "react"
import {
    DatagridConfigurable,
    FunctionField,
    List,
    TextField,
    useListContext,
    useRecordContext,
} from "react-admin"
import {Box, Chip, IconButton, Menu, MenuItem, Stack, Typography} from "@mui/material"
import LockOutlinedIcon from "@mui/icons-material/LockOutlined"
import LanguageIcon from "@mui/icons-material/Language"
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
import {
    ForensicPseudonym,
    ForensicCountry,
    ForensicBallotCount,
    ForensicAddressButton,
} from "./ForensicFields"

export interface ListIpAddressProps {
    aside?: ReactElement
}
export interface RecordVoteCloudflareData {
    ip?: string
    country?: string
}

const Empty: React.FC = () => {
    const {t} = useTranslation()
    return (
        <Stack alignItems="center" spacing={1} sx={{py: 6, px: 3, textAlign: "center"}}>
            <LanguageIcon sx={{fontSize: 36, color: "text.secondary"}} />
            <Typography variant="body2" color="text.secondary">
                {t("ballotPrivacy.forensic.emptyList")}
            </Typography>
        </Stack>
    )
}

const SelectedForensicAddress: React.FC<{
    electionEventId: string
    electionId?: string
    selected?: IForensicAddress
    onClose: () => void
}> = ({electionEventId, electionId, selected, onClose}) => {
    const {filterValues} = useListContext()
    const aliasRenderer = useAliasRenderer()
    const {t} = useTranslation()
    const effectiveElectionId = electionId ?? filterValues.election_id
    return selected ? (
        <ForensicAddressDialog
            key={`${effectiveElectionId ?? "all"}:${selected.ip}`}
            electionEventId={electionEventId}
            electionId={effectiveElectionId}
            ip={selected.ip}
            shared={selected.shared}
            electionLabel={
                selected.election_presentation
                    ? aliasRenderer(selected.election_presentation)
                    : t("ballotPrivacy.forensic.allElections")
            }
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
                size="small"
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
    const titleId = useId()
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
    const pseudonymLimit = width !== null && width < 600 ? 1 : 2
    const preferenceKey = `forensic_network.${electionId ? "election" : "event"}.${compact ? "compact" : "table"}`
    const canRead = auth.isAuthorized(true, tenantId, IPermissions.FORENSIC_READ)
    if (
        !electionEventId ||
        !canRead ||
        presentation?.network_data_policy !== ENetworkDataPolicy.FORENSIC_STORE
    )
        return null
    const preview = (row: IForensicAddress) => (
        <Stack direction="row" useFlexGap flexWrap="wrap" gap={0.5}>
            {(row.pseudonyms ?? []).slice(0, pseudonymLimit).map((pseudonym) => (
                <ForensicPseudonym key={pseudonym} code={pseudonym} />
            ))}
            {(row.pseudonym_count ?? row.pseudonyms?.length) > pseudonymLimit && (
                <Typography
                    variant="caption"
                    color="#0F054C"
                    sx={{alignSelf: "center", whiteSpace: "nowrap", fontWeight: 600}}
                >
                    {t("ballotPrivacy.forensic.more", {
                        count: (row.pseudonym_count ?? row.pseudonyms.length) - pseudonymLimit,
                    })}
                </Typography>
            )}
        </Stack>
    )
    return (
        <Box
            ref={ref}
            component="section"
            aria-labelledby={titleId}
            className="print-hidden forensic-network-panel"
            sx={{minWidth: 0, maxWidth: "100%", mt: 3}}
        >
            <Stack
                direction="row"
                justifyContent="space-between"
                alignItems="flex-start"
                useFlexGap
                flexWrap="wrap"
                gap={1}
                sx={{mb: 1}}
            >
                <Box sx={{minWidth: 0}}>
                    <Typography
                        component="h2"
                        variant="h5"
                        id={titleId}
                        sx={{fontWeight: 600, fontSize: 24}}
                    >
                        {t("dashboard.ipAddress.title")}
                    </Typography>
                    <Typography variant="body2" color="text.secondary" sx={{mt: 0.5}}>
                        {t("ballotPrivacy.forensic.subtitle")}
                    </Typography>
                </Box>
                <Chip
                    size="small"
                    icon={<LockOutlinedIcon />}
                    label={t("ballotPrivacy.forensic.store")}
                    sx={{
                        "height": 26,
                        "borderRadius": "100px",
                        "fontSize": 12,
                        "fontWeight": 500,
                        "color": "#0F054C",
                        "bgcolor": "#F1F0FA",
                        "& .MuiChip-icon": {fontSize: 15, color: "inherit"},
                    }}
                />
            </Stack>
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
                        preferenceKey={preferenceKey}
                    />
                }
                sx={{
                    "minWidth": 0,
                    "& .RaList-main": {minWidth: 0},
                    "& .RaList-content": {
                        border: "1px solid",
                        borderColor: "divider",
                        borderRadius: 1,
                        boxShadow: "none",
                    },
                    "& .RaFilterForm-form": {flexWrap: "wrap", minWidth: 0},
                    "& .RaFilterForm-filterFormInput": {maxWidth: "100%", minWidth: 0},
                    "& .RaList-actions": {flexWrap: "wrap", p: 0, mb: 2, gap: 1},
                    "& .RaList-actions > form": {flex: "1 1 auto", minHeight: 0, p: 0, m: 0},
                    "& .RaList-actions > .list-actions": {ml: "auto", maxWidth: "100%"},
                    "& .RaList-actions .MuiToolbar-root": {
                        minHeight: 0,
                        p: 0,
                        borderRight: 0,
                        gap: 0,
                    },
                    "& .RaList-actions .MuiButton-root": {
                        "bgcolor": "transparent",
                        "color": "#0F054C",
                        "border": "1px solid",
                        "borderColor": "#0F054C",
                        "borderRadius": 0,
                        "minHeight": 36,
                        "minWidth": 36,
                        "textTransform": "uppercase",
                        "fontSize": 12,
                        "fontWeight": 500,
                        "&:hover": {bgcolor: "action.hover"},
                        "& + .MuiButton-root": {borderLeft: 0},
                    },
                    "& .MuiToolbar-root:last-child": {borderRight: 0},
                    "& .MuiToolbar-root": {flexWrap: "wrap", gap: 0.5},
                    "& .RaDatagrid-table": {tableLayout: "fixed", width: "100%"},
                    "& .RaDatagrid-headerCell": {
                        bgcolor: "background.paper",
                        color: "text.secondary",
                        fontWeight: 600,
                        py: 1,
                    },
                    "& .RaDatagrid-rowCell": {
                        overflowWrap: "anywhere",
                        py: compact ? 2 : 1.5,
                        verticalAlign: "middle",
                    },
                    "& .RaDatagrid-checkbox": {width: 42, p: 0},
                    "& .MuiCheckbox-root": {width: 36, height: 36, p: 0},
                    "& td.MuiTableCell-paddingCheckbox, & th.MuiTableCell-paddingCheckbox": {
                        width: 42,
                        px: 1,
                        py: 0,
                    },
                    "& .column-ip": {width: compact ? "auto" : "18%"},
                    "& .column-country": {width: 80},
                    "& .column-vote_count": {width: 130, textAlign: "right"},
                    "& .column-election_presentation": {width: "16%"},
                    "& .column-first_ballot, & .column-last_ballot": {width: "19%"},
                    "& .column-actions": {width: 56, textAlign: "right"},
                    "& .RaPagination-root": {px: 1},
                }}
            >
                <DatagridConfigurable
                    key={compact ? "compact" : "table"}
                    preferenceKey={preferenceKey}
                    omit={compact ? [] : ["first_ballot", "election_presentation"]}
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
                            label={t("ballotPrivacy.forensic.address")}
                            render={(row: IForensicAddress) => (
                                <Stack spacing={1}>
                                    <Stack
                                        direction="row"
                                        alignItems="flex-start"
                                        justifyContent="space-between"
                                        spacing={1}
                                    >
                                        <ForensicAddressButton
                                            ip={row.ip}
                                            onOpen={() => setSelected(row)}
                                        />
                                        <ForensicBallotCount
                                            count={row.vote_count}
                                            shared={row.shared}
                                        />
                                    </Stack>
                                    <Stack
                                        direction="row"
                                        alignItems="center"
                                        useFlexGap
                                        flexWrap="wrap"
                                        gap={1}
                                    >
                                        <ForensicCountry code={row.country} showName />
                                        <Typography variant="body2">
                                            <AdminDateTime
                                                value={row.last_ballot}
                                                event={electionEventId}
                                                seconds
                                            />
                                        </Typography>
                                    </Stack>
                                    {preview(row)}
                                </Stack>
                            )}
                        />
                    ) : (
                        [
                            <FunctionField
                                source="ip"
                                sortable={false}
                                label={t("ballotPrivacy.forensic.address")}
                                render={(row: IForensicAddress) => (
                                    <ForensicAddressButton
                                        ip={row.ip}
                                        onOpen={() => setSelected(row)}
                                    />
                                )}
                            />,
                            <FunctionField
                                source="country"
                                sortable={false}
                                label={t("dashboard.ipAddress.country")}
                                render={(row: IForensicAddress) => (
                                    <ForensicCountry code={row.country} />
                                )}
                            />,
                            <FunctionField
                                source="vote_count"
                                sortable={false}
                                label={t("ballotPrivacy.forensic.ballots")}
                                render={(row: IForensicAddress) => (
                                    <ForensicBallotCount
                                        count={row.vote_count}
                                        shared={row.shared}
                                    />
                                )}
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
                                    <Typography
                                        variant="body2"
                                        color="text.secondary"
                                        sx={{fontSize: 12, lineHeight: 1.7}}
                                    >
                                        <AdminDateTime
                                            value={row.last_ballot}
                                            event={electionEventId}
                                            seconds
                                        />
                                    </Typography>
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
