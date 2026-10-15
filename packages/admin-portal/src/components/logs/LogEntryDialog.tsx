// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useId} from "react"
import {Box, Chip, Divider, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import type {TFunction} from "i18next"
import {Dialog} from "@sequentech/ui-essentials"
import {useZonedFormat} from "@/hooks/useZonedFormat"
import {
    ELogChangeKind,
    ELogOrigin,
    logEntryView,
    shortened,
    type ILogChange,
    type ILogEntryView,
    type LogValue,
} from "./logEntryDetails"
import {logFieldLabelKey, logObjectTypeKey} from "./logFieldLabels"
import {logMessage} from "./logMessage"
import {ELogScope, type ILogTarget} from "./logScope"
import {readinessDescription, readinessDetailRows} from "./readinessLog"
import {StatementExplanation, explanationLines} from "./StatementExplanation"

/** A row of a log list, as listElectoralLog answers it. */
export interface ILogRow {
    id: number | string
    created?: number | null
    statement_timestamp?: number | null
    statement_kind?: string | null
    message?: string | null
    user_id?: string | null
}

export interface LogEntryDialogProps {
    /** The entry shown; the popup is closed without one. */
    entry: ILogRow | null
    /** The log the entry belongs to. */
    log: ILogTarget
    /** The zone the log shows this entry in. */
    zone: string
    onClose: () => void
}

const LOG_TYPE_ERROR = "ERROR"
const STATUS_RETIRED = "RETIRED"
const MONO = {fontFamily: "monospace", fontSize: "0.8125rem", overflowWrap: "anywhere"} as const

const Fact: React.FC<
    React.PropsWithChildren<{label: string; detail?: React.ReactNode; wide?: boolean}>
> = ({label, detail, wide, children}) => (
    <Box
        className="log-entry-fact"
        sx={{
            display: "grid",
            gridTemplateColumns: {xs: "1fr", sm: "8rem minmax(0, 1fr)"},
            columnGap: 2,
            gridColumn: wide ? "1 / -1" : undefined,
            minWidth: 0,
        }}
    >
        <Typography component="dt" variant="body2" sx={{m: 0, color: "text.secondary"}}>
            {label}
        </Typography>
        <Box component="dd" sx={{m: 0, minWidth: 0}}>
            <Typography component="div" variant="body2" sx={{m: 0, overflowWrap: "anywhere"}}>
                {children}
            </Typography>
            {detail ? (
                <Typography
                    component="div"
                    variant="body2"
                    sx={{m: 0, color: "text.secondary", overflowWrap: "anywhere"}}
                >
                    {detail}
                </Typography>
            ) : null}
        </Box>
    </Box>
)

const Facts: React.FC<React.PropsWithChildren> = ({children}) => (
    <Box
        component="dl"
        sx={{
            display: "grid",
            gridTemplateColumns: {xs: "minmax(0, 1fr)", md: "repeat(2, minmax(0, 1fr))"},
            columnGap: 4,
            rowGap: 1.5,
            m: 0,
        }}
    >
        {children}
    </Box>
)

const Section: React.FC<React.PropsWithChildren<{title: string}>> = ({title, children}) => (
    <Box component="section" sx={{display: "flex", flexDirection: "column", gap: 1.5}}>
        <Divider />
        <Typography component="h3" variant="subtitle2" sx={{m: 0, fontWeight: 700}}>
            {title}
        </Typography>
        {children}
    </Box>
)

/** A key, hash or signature: shortened on its line, whole in its title for copying. */
const Shortened: React.FC<{value: string}> = ({value}) => (
    <Box component="span" title={value} sx={MONO}>
        {shortened(value)}
    </Box>
)

const Mono: React.FC<React.PropsWithChildren> = ({children}) => (
    <Box component="span" sx={MONO}>
        {children}
    </Box>
)

const valueText = (t: TFunction, value: LogValue): string => {
    if (value === null) return String(t("logsScreen.entry.values.none"))
    if (typeof value === "boolean") {
        return String(t(value ? "logsScreen.entry.values.yes" : "logsScreen.entry.values.no"))
    }
    return value
}

const hashText = (t: TFunction, hash: string | null): string =>
    hash === null
        ? String(t("logsScreen.entry.values.none"))
        : String(t("logsScreen.entry.values.sha", {hash}))

const CELL = {
    py: 1,
    px: 1.5,
    textAlign: "start",
    verticalAlign: "top",
    borderTop: "1px solid",
    borderColor: "divider",
    overflowWrap: "anywhere",
} as const

const ChangesTable: React.FC<{objectType: string | null; changes: ILogChange[]}> = ({
    objectType,
    changes,
}) => {
    const {t} = useTranslation()
    return (
        <Box
            component="table"
            className="log-entry-changes"
            sx={{
                width: "100%",
                tableLayout: "fixed",
                borderCollapse: "collapse",
                border: "1px solid",
                borderColor: "divider",
                typography: "body2",
            }}
        >
            <thead>
                <tr>
                    {["field", "old", "new"].map((column) => (
                        <Box
                            component="th"
                            scope="col"
                            key={column}
                            sx={{...CELL, borderTop: 0, bgcolor: "action.hover", fontWeight: 700}}
                        >
                            {t(`logsScreen.entry.values.${column}`)}
                        </Box>
                    ))}
                </tr>
            </thead>
            <tbody>
                {changes.map((change) => {
                    const labelKey = logFieldLabelKey(objectType, change.path)
                    return (
                        <tr key={change.path}>
                            <Box component="th" scope="row" sx={{...CELL, fontWeight: 400}}>
                                {labelKey ? <Box component="span">{t(labelKey)}</Box> : null}
                                <Box
                                    component="span"
                                    sx={{
                                        ...MONO,
                                        display: "block",
                                        color: labelKey ? "text.secondary" : undefined,
                                    }}
                                >
                                    {change.path}
                                </Box>
                            </Box>
                            {change.kind === ELogChangeKind.SECRET ? (
                                <Box component="td" colSpan={2} sx={CELL}>
                                    {t("logsScreen.entry.values.secret")}
                                </Box>
                            ) : change.kind === ELogChangeKind.HASHES ? (
                                <>
                                    <Box component="td" sx={{...CELL, ...MONO}}>
                                        {hashText(t, change.oldSha256)}
                                    </Box>
                                    <Box component="td" sx={{...CELL, ...MONO}}>
                                        {hashText(t, change.newSha256)}
                                    </Box>
                                </>
                            ) : (
                                <>
                                    <Box component="td" sx={CELL}>
                                        {valueText(t, change.old)}
                                    </Box>
                                    <Box component="td" sx={CELL}>
                                        {valueText(t, change.new)}
                                    </Box>
                                </>
                            )}
                        </tr>
                    )
                })}
            </tbody>
        </Box>
    )
}

const originText = (t: TFunction, origin: ELogOrigin): string =>
    String(t(`logsScreen.entry.origin.${origin}`))

const objectText = (t: TFunction, view: ILogEntryView): string | null => {
    if (!view.object) return null
    const typeKey = logObjectTypeKey(view.object.type)
    const type = typeKey ? String(t(typeKey)) : view.object.type
    return view.object.name ? `${type}: ${view.object.name}` : type
}

const actionText = (t: TFunction, action: string): string =>
    String(t(`logsScreen.entry.action.${action}`, {defaultValue: action}))

const utcText = (seconds: number): string =>
    new Date(seconds * 1000).toISOString().replace(/\.\d{3}Z$/, "Z")

const entryNumber = (id: number | string, language: string): string => {
    const value = Number(id)
    if (!Number.isFinite(value)) return String(id)
    try {
        return new Intl.NumberFormat(language).format(value)
    } catch {
        return new Intl.NumberFormat("en").format(value)
    }
}

const EntryContent: React.FC<{entry: ILogRow; log: ILogTarget; zone: string}> = ({
    entry,
    log,
    zone,
}) => {
    const {t} = useTranslation()
    const format = useZonedFormat(zone, {seconds: true})
    const descriptionId = useId()
    const view = logEntryView(entry)
    const message = logMessage(entry)
    const extra = readinessDetailRows(t, entry.statement_kind, message) ?? view.extra
    const seconds = entry.statement_timestamp ?? entry.created
    const explained = !!explanationLines(t, entry.statement_kind, message)?.length
    const {who, signIn, keycloak, deletedEvent, exported, writer, signatures} = view

    const chips: Array<{title: string; label: string | null; error?: boolean}> = [
        {title: String(t("logsScreen.entry.statementKind")), label: view.statementKind},
        {title: String(t("logsScreen.entry.eventType")), label: view.eventType},
        {
            title: String(t("logsScreen.entry.logType")),
            label: view.logType,
            error: view.logType === LOG_TYPE_ERROR,
        },
    ]
    // The signer's name is the person's; the username is the account.
    const whoName = who.unknownAccount
        ? String(t("logsScreen.entry.unknownAccount"))
        : who.username && signatures.signedBy && signatures.signedBy !== who.username
          ? `${signatures.signedBy} (${who.username})`
          : (who.username ?? "-")
    const object = objectText(t, view)

    return (
        <Box
            className="log-entry"
            // A long entry scrolls: the keyboard reaches it here.
            role="group"
            tabIndex={0}
            aria-labelledby={descriptionId}
            sx={{display: "flex", flexDirection: "column", gap: 2, pt: 1, minWidth: 0}}
        >
            <Typography
                component="p"
                variant="subtitle1"
                id={descriptionId}
                className="log-entry-description"
                sx={{fontWeight: 500, overflowWrap: "anywhere", whiteSpace: "pre-wrap", m: 0}}
            >
                {readinessDescription(t, entry.statement_kind) ?? (view.description || "-")}
            </Typography>
            <Box sx={{display: "flex", flexWrap: "wrap", gap: 1}}>
                {chips.map(({title, label, error}) =>
                    label ? (
                        <Chip
                            key={title}
                            size="small"
                            title={title}
                            aria-label={`${title}: ${label}`}
                            label={label}
                            color={error ? "error" : "default"}
                            variant={error ? "filled" : "outlined"}
                        />
                    ) : null
                )}
            </Box>
            {view.unreadable ? null : (
                <Facts>
                    {seconds === null || seconds === undefined ? null : (
                        <Fact
                            label={String(t("logsScreen.entry.when"))}
                            detail={
                                <>
                                    {format.mine(seconds * 1000, zone) ? (
                                        <span className="log-entry-mine">
                                            {format.mine(seconds * 1000, zone)}
                                        </span>
                                    ) : null}
                                    <Box
                                        component="span"
                                        className="log-entry-utc"
                                        sx={{display: "block"}}
                                    >
                                        {t("logsScreen.entry.utc", {time: utcText(seconds)})}
                                    </Box>
                                </>
                            }
                        >
                            <span className="log-entry-time">
                                {format.format(seconds * 1000, zone)}
                            </span>
                        </Fact>
                    )}
                    <Fact
                        label={String(t("logsScreen.entry.who"))}
                        detail={
                            who.userId || who.role ? (
                                <>
                                    {who.userId ? (
                                        <Box component="span" sx={{display: "block"}}>
                                            {t("logsScreen.entry.userId", {id: who.userId})}
                                        </Box>
                                    ) : null}
                                    {who.role ? (
                                        <Box component="span" sx={{display: "block"}}>
                                            {t("logsScreen.entry.role", {role: who.role})}
                                        </Box>
                                    ) : null}
                                </>
                            ) : undefined
                        }
                    >
                        {whoName}
                    </Fact>
                    {view.origin && !keycloak ? (
                        <Fact label={String(t("logsScreen.entry.madeIn"))}>
                            {originText(t, view.origin)}
                        </Fact>
                    ) : null}
                    {object ? (
                        <Fact
                            label={String(t("logsScreen.entry.changed"))}
                            detail={
                                view.object?.id
                                    ? t("logsScreen.entry.objectId", {id: view.object.id})
                                    : undefined
                            }
                        >
                            {object}
                        </Fact>
                    ) : null}
                    {view.action ? (
                        <Fact label={String(t("logsScreen.entry.actionLabel"))}>
                            {actionText(t, view.action)}
                        </Fact>
                    ) : null}
                </Facts>
            )}
            {explained ? (
                <StatementExplanation kind={entry.statement_kind} message={message} />
            ) : null}
            {view.changes.length ? (
                <Section title={String(t("logsScreen.entry.values.title"))}>
                    <ChangesTable objectType={view.object?.type ?? null} changes={view.changes} />
                </Section>
            ) : null}
            {signIn ? (
                <Section title={String(t("logsScreen.entry.signIn.title"))}>
                    <Facts>
                        {signIn.event ? (
                            <Fact label={String(t("logsScreen.entry.signIn.event"))}>
                                <Mono>{signIn.event}</Mono>
                            </Fact>
                        ) : null}
                        {signIn.error ? (
                            <Fact label={String(t("logsScreen.entry.signIn.error"))}>
                                <Mono>{signIn.error}</Mono>
                            </Fact>
                        ) : null}
                        {signIn.authentication ? (
                            <Fact label={String(t("logsScreen.entry.signIn.authentication"))}>
                                {signIn.authentication}
                            </Fact>
                        ) : null}
                        {signIn.credential ? (
                            <Fact label={String(t("logsScreen.entry.signIn.credential"))}>
                                {signIn.credential}
                            </Fact>
                        ) : null}
                        {signIn.failures ? (
                            <Fact label={String(t("logsScreen.entry.signIn.failures"))}>
                                {signIn.failures}
                            </Fact>
                        ) : null}
                        {signIn.clientId ? (
                            <Fact label={String(t("logsScreen.entry.signIn.client"))}>
                                <Mono>{signIn.clientId}</Mono>
                            </Fact>
                        ) : null}
                        {signIn.ipAddress ? (
                            <Fact label={String(t("logsScreen.entry.signIn.ipAddress"))}>
                                <Mono>{signIn.ipAddress}</Mono>
                            </Fact>
                        ) : null}
                        {signIn.details.map(({name, value}) => (
                            <Fact key={name} label={name}>
                                {value}
                            </Fact>
                        ))}
                        {signIn.text ? (
                            <Fact label={String(t("logsScreen.entry.details"))} wide>
                                {signIn.text}
                            </Fact>
                        ) : null}
                    </Facts>
                </Section>
            ) : null}
            {keycloak ? (
                <Section title={String(t("logsScreen.entry.keycloak.title"))}>
                    <Facts>
                        <Fact label={String(t("logsScreen.entry.madeIn"))} wide>
                            {originText(t, keycloak.origin)}
                        </Fact>
                        <Fact
                            label={String(t("logsScreen.entry.keycloak.account"))}
                            detail={
                                keycloak.clientId || keycloak.ipAddress ? (
                                    <Mono>
                                        {[keycloak.clientId, keycloak.ipAddress]
                                            .filter(Boolean)
                                            .join(" · ")}
                                    </Mono>
                                ) : undefined
                            }
                        >
                            {[keycloak.username, keycloak.realm].filter(Boolean).join(" · ") || "-"}
                        </Fact>
                        <Fact label={String(t("logsScreen.entry.keycloak.operation"))}>
                            <Mono>
                                {[keycloak.operation, keycloak.resourceType]
                                    .filter(Boolean)
                                    .join(" ") || "-"}
                            </Mono>
                        </Fact>
                        {keycloak.appliesTo ? (
                            <Fact label={String(t("logsScreen.entry.keycloak.appliesTo"))}>
                                {keycloak.appliesTo}
                            </Fact>
                        ) : null}
                        {keycloak.error ? (
                            <Fact label={String(t("logsScreen.entry.keycloak.error"))}>
                                <Mono>{keycloak.error}</Mono>
                            </Fact>
                        ) : null}
                        {keycloak.resourcePath ? (
                            <Fact label={String(t("logsScreen.entry.keycloak.resource"))} wide>
                                <Mono>{keycloak.resourcePath}</Mono>
                            </Fact>
                        ) : null}
                        {keycloak.representation ? (
                            <Fact label={String(t("logsScreen.entry.keycloak.newValue"))} wide>
                                <Box
                                    component="pre"
                                    className="log-entry-representation"
                                    sx={{
                                        ...MONO,
                                        m: 0,
                                        p: 1,
                                        maxHeight: 240,
                                        overflow: "auto",
                                        whiteSpace: "pre-wrap",
                                        bgcolor: "action.hover",
                                    }}
                                    tabIndex={0}
                                >
                                    {keycloak.representation}
                                </Box>
                            </Fact>
                        ) : null}
                    </Facts>
                </Section>
            ) : null}
            {deletedEvent ? (
                <Section title={String(t("logsScreen.entry.deletedEvent.title"))}>
                    <Facts>
                        {deletedEvent.lastEntryId ? (
                            <Fact label={String(t("logsScreen.entry.deletedEvent.lastEntry"))}>
                                {deletedEvent.lastEntryId}
                            </Fact>
                        ) : null}
                        {deletedEvent.stateHash ? (
                            <Fact label={String(t("logsScreen.entry.deletedEvent.stateHash"))}>
                                <Shortened value={deletedEvent.stateHash} />
                            </Fact>
                        ) : null}
                        <Fact label={String(t("logsScreen.entry.deletedEvent.logDatabase"))} wide>
                            <Mono>{deletedEvent.logDatabase}</Mono>
                        </Fact>
                        {deletedEvent.status ? (
                            <Fact label={String(t("logsScreen.entry.deletedEvent.status"))} wide>
                                {deletedEvent.status === STATUS_RETIRED
                                    ? t("logsScreen.entry.deletedEvent.retired")
                                    : deletedEvent.status}
                            </Fact>
                        ) : null}
                        {deletedEvent.taskId ? (
                            <Fact label={String(t("logsScreen.entry.deletedEvent.task"))} wide>
                                <Mono>{deletedEvent.taskId}</Mono>
                            </Fact>
                        ) : null}
                    </Facts>
                </Section>
            ) : null}
            {exported ? (
                <Section title={String(t("logsScreen.entry.exported.title"))}>
                    <Facts>
                        {exported.format ? (
                            <Fact label={String(t("logsScreen.exportdialog.format"))}>
                                {exported.format}
                            </Fact>
                        ) : null}
                        {exported.scope ? (
                            <Fact label={String(t("logsScreen.exportdialog.scope.label"))}>
                                {t(`logsScreen.entry.exported.scope.${exported.scope}`, {
                                    defaultValue: exported.scope,
                                })}
                            </Fact>
                        ) : null}
                        {exported.username ? (
                            <Fact label={String(t("logsScreen.exportdialog.username"))}>
                                {exported.username}
                            </Fact>
                        ) : null}
                        {exported.electionId ? (
                            <Fact label={String(t("logsScreen.exportdialog.election"))}>
                                <Mono>{exported.electionId}</Mono>
                            </Fact>
                        ) : null}
                        {exported.areaId ? (
                            <Fact label={String(t("logsScreen.exportdialog.area"))}>
                                <Mono>{exported.areaId}</Mono>
                            </Fact>
                        ) : null}
                        {exported.createdFrom ? (
                            <Fact label={String(t("logsScreen.exportdialog.from"))}>
                                {exported.createdFrom}
                            </Fact>
                        ) : null}
                        {exported.createdTo ? (
                            <Fact label={String(t("logsScreen.exportdialog.to"))}>
                                {exported.createdTo}
                            </Fact>
                        ) : null}
                        {exported.timeZone ? (
                            <Fact label={String(t("logsScreen.exportdialog.timeZone"))}>
                                {exported.timeZone}
                            </Fact>
                        ) : null}
                        {exported.entries ? (
                            <Fact label={String(t("logsScreen.entry.exported.entries"))}>
                                {exported.entries}
                            </Fact>
                        ) : null}
                        {exported.sha256 ? (
                            <Fact label={String(t("logsScreen.entry.exported.sha256"))}>
                                <Shortened value={exported.sha256} />
                            </Fact>
                        ) : null}
                        {exported.documentId ? (
                            <Fact label={String(t("logsScreen.entry.exported.document"))} wide>
                                <Mono>{exported.documentId}</Mono>
                            </Fact>
                        ) : null}
                    </Facts>
                </Section>
            ) : null}
            {writer ? (
                <Section title={String(t("logsScreen.entry.writer.title"))}>
                    <Facts>
                        <Fact label={String(t("logsScreen.entry.writer.release"))}>
                            {writer.release ?? "-"}
                        </Fact>
                        <Fact label={String(t("logsScreen.entry.writer.lastEntry"))}>
                            {writer.lastEntryId ?? t("logsScreen.entry.values.none")}
                        </Fact>
                    </Facts>
                </Section>
            ) : null}
            {extra.length ? (
                <Section title={String(t("logsScreen.entry.details"))}>
                    <Facts>
                        {extra.map(({name, value}) => (
                            <Fact key={name} label={name}>
                                {value}
                            </Fact>
                        ))}
                    </Facts>
                </Section>
            ) : null}
            {view.unreadable ? null : (
                <Section title={String(t("logsScreen.entry.signatures.title"))}>
                    <Facts>
                        <Fact
                            label={String(t("logsScreen.entry.signatures.log"))}
                            detail={
                                log.kind === ELogScope.EVENT ? (
                                    <Mono>{log.electionEventId}</Mono>
                                ) : undefined
                            }
                        >
                            {log.kind === ELogScope.TENANT
                                ? t("logsScreen.entry.signatures.tenantLog")
                                : t("logsScreen.entry.signatures.eventLog")}
                        </Fact>
                        <Fact
                            label={String(t("logsScreen.entry.signatures.signedBy"))}
                            detail={
                                signatures.signedBy && signatures.senderKey ? (
                                    <Shortened value={signatures.senderKey} />
                                ) : undefined
                            }
                        >
                            {signatures.signedBy
                                ? t("logsScreen.entry.signatures.adminKey", {
                                      name: signatures.signedBy,
                                  })
                                : t("logsScreen.entry.signatures.systemOnly")}
                        </Fact>
                        {signatures.systemSignature ? (
                            <Fact label={String(t("logsScreen.entry.signatures.systemSignature"))}>
                                <Shortened value={signatures.systemSignature} />
                            </Fact>
                        ) : null}
                        {view.publicKey ? (
                            <Fact label={String(t("logsScreen.entry.signatures.publicKey"))}>
                                <Shortened value={view.publicKey} />
                            </Fact>
                        ) : null}
                    </Facts>
                </Section>
            )}
        </Box>
    )
}

/**
 * A log entry's details, for an election event's log and the tenant log: who,
 * when, what changed with its old and new values, the sign-in or Keycloak
 * details, and the log and keys that signed it.
 */
export const LogEntryDialog: React.FC<LogEntryDialogProps> = ({entry, log, zone, onClose}) => {
    const {t, i18n} = useTranslation()
    return (
        <Dialog
            variant="info"
            open={!!entry}
            fullWidth
            maxWidth="md"
            className="log-entry-dialog"
            title={String(
                t("logsScreen.entry.title", {
                    id: entry ? entryNumber(entry.id, i18n.language) : "",
                })
            )}
            ok={String(t("common.label.close"))}
            handleClose={onClose}
        >
            {entry ? <EntryContent entry={entry} log={log} zone={zone} /> : null}
        </Dialog>
    )
}
