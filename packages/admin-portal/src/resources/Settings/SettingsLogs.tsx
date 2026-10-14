// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement, useContext} from "react"
import {
    DatagridConfigurable,
    FunctionField,
    List,
    NumberField,
    SelectInput,
    TextField,
    TextInput,
    useRecordContext,
} from "react-admin"
import {Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {IPermissions} from "@/types/keycloak"
import {HeaderTitle} from "@/components/HeaderTitle"
import {ListActions} from "@/components/ListActions"
import {MessageField} from "@/components/MessageField"
import {ResetFilters} from "@/components/ResetFilters"
import {ThreeStateDatagridHeader} from "@/components/ThreeStateDatagridHeader"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import {LogRangeDateTimeInput} from "@/components/logs/LogRangeDateTimeInput"
import {LogTime} from "@/components/logs/LogTime"
import {logMessage} from "@/components/logs/logMessage"
import {StatementExplanation, explanationLines} from "@/components/logs/StatementExplanation"
import {useZonedFormat, type IZonedFormat} from "@/hooks/useZonedFormat"
import {ELECTORAL_LOG_DEFAULT_ZONE_FILTER} from "@/queries/ListElectoralLog"
import {TENANT_LOG_RESOURCE} from "@/queries/ListTenantLog"

/** The statement kinds a tenant's log holds. */
export enum TenantLogStatementKind {
    TENANT_LOG_OPENED = "TenantLogOpened",
    TENANT_ADMIN_ACTION = "TenantAdminAction",
    KEYCLOAK_USER_EVENT = "KeycloakUserEvent",
}

interface ITenantLogRow {
    id?: number
    created?: number
    statement_timestamp?: number
    statement_kind?: string
    message?: string
    user_id?: string | null
    username?: string | null
}

/** A missing value, or the `"null"` Keycloak's listener sends for one. */
const shown = (value?: string | null): string | null => (!value || value === "null" ? null : value)

const TenantLogRowTime: React.FC<{
    source: "created" | "statement_timestamp"
    format: IZonedFormat
}> = ({source, format}) => {
    const record = useRecordContext<ITenantLogRow>()
    return <LogTime seconds={record?.[source]} zone={format.zone} format={format} />
}

/**
 * The tenant's log: who signed in to the Admin Portal and what was asked of
 * the platform outside any election event. Times show in the viewer's zone.
 */
export const SettingsLogs: React.FC = () => {
    const {t} = useTranslation()
    const authContext = useContext(AuthContext)
    const [tenantId] = useTenantStore()
    const format = useZonedFormat(null, {seconds: true})
    const canReadLogs = authContext.isAuthorized(true, tenantId, IPermissions.TENANT_LOGS_READ)

    if (!canReadLogs) {
        return (
            <ResourceListStyles.EmptyBox>
                <Typography variant="h4" paragraph>
                    {t("tenantLogs.noPermissions")}
                </Typography>
            </ResourceListStyles.EmptyBox>
        )
    }

    const headField = (record: ITenantLogRow, field: "event_type" | "log_type") =>
        logMessage(record)?.statement?.head?.[field] ?? <span>-</span>

    const filters: Array<ReactElement> = [
        <LogRangeDateTimeInput
            defaultZone={format.zone}
            key="created_from"
            source="created_from"
            label={String(t("logsScreen.filter.createdFrom"))}
            alwaysOn
        />,
        <LogRangeDateTimeInput
            defaultZone={format.zone}
            key="created_to"
            source="created_to"
            label={String(t("logsScreen.filter.createdTo"))}
            alwaysOn
        />,
        <TextInput
            key="username"
            source="username"
            label={String(t("logsScreen.column.username"))}
        />,
        <TextInput key="user_id" source="user_id" label={String(t("logsScreen.column.user_id"))} />,
        <SelectInput
            key="statement_kind"
            source="statement_kind"
            label={String(t("logsScreen.column.statement_kind"))}
            choices={Object.values(TenantLogStatementKind).map((kind) => ({
                id: kind,
                name: String(t(`tenantLogs.kind.${kind}`)),
            }))}
        />,
    ]

    return (
        <>
            <HeaderTitle title="tenantLogs.title" subtitle="tenantLogs.subtitle" />
            <List
                resource={TENANT_LOG_RESOURCE}
                actions={
                    <ListActions
                        withColumns
                        withImport={false}
                        withExport={false}
                        withFilter
                        preferenceKey="tenant-log"
                    />
                }
                filters={filters}
                filter={{[ELECTORAL_LOG_DEFAULT_ZONE_FILTER]: format.zone}}
                storeKey={false}
                sort={{field: "id", order: "DESC"}}
                empty={false}
                disableSyncWithLocation
            >
                <ResetFilters />
                <DatagridConfigurable
                    header={ThreeStateDatagridHeader}
                    omit={["user_id", "created", "message"]}
                    bulkActionButtons={false}
                    preferenceKey="tenant-log"
                >
                    <NumberField source="id" label={String(t("logsScreen.column.id"))} />
                    <FunctionField
                        source="statement_timestamp"
                        label={String(t("logsScreen.column.statement_timestamp"))}
                        render={() => (
                            <TenantLogRowTime source="statement_timestamp" format={format} />
                        )}
                    />
                    <FunctionField
                        source="created"
                        label={String(t("logsScreen.column.created"))}
                        render={() => <TenantLogRowTime source="created" format={format} />}
                    />
                    <FunctionField
                        source="username"
                        label={String(t("logsScreen.column.username"))}
                        render={(record: ITenantLogRow) => shown(record.username) ?? <span>-</span>}
                    />
                    <FunctionField
                        source="user_id"
                        label={String(t("logsScreen.column.user_id"))}
                        render={(record: ITenantLogRow) => shown(record.user_id) ?? <span>-</span>}
                    />
                    <TextField
                        source="statement_kind"
                        label={String(t("logsScreen.column.statement_kind"))}
                    />
                    <FunctionField
                        source="event_type"
                        sortable={false}
                        label={String(t("logsScreen.column.event_type"))}
                        render={(record: ITenantLogRow) => headField(record, "event_type")}
                    />
                    <FunctionField
                        source="log_type"
                        sortable={false}
                        label={String(t("logsScreen.column.log_type"))}
                        render={(record: ITenantLogRow) => headField(record, "log_type")}
                    />
                    <FunctionField
                        source="description"
                        sortable={false}
                        label={String(t("logsScreen.column.description"))}
                        render={(record: ITenantLogRow) => {
                            const message = logMessage(record)
                            const explained = !!explanationLines(t, record.statement_kind, message)
                                ?.length
                            return (
                                <MessageField
                                    content={message?.statement?.head?.description || "-"}
                                    initialLength={80}
                                    details={
                                        explained ? (
                                            <StatementExplanation
                                                kind={record.statement_kind}
                                                message={message}
                                            />
                                        ) : undefined
                                    }
                                />
                            )
                        }}
                    />
                    <MessageField source="message" />
                </DatagridConfigurable>
            </List>
        </>
    )
}
