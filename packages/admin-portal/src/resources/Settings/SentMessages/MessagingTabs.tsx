// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useCallback} from "react"
import {useSearchParams} from "react-router-dom"
import {useTranslation} from "react-i18next"
import {Box, Tab, Tabs} from "@mui/material"
import {SentMessages} from "./SentMessages"
import {useFakeGetStarted, useSentMessages} from "./useSentMessages"
import {
    SENT_MESSAGES_PARAM,
    SENT_MESSAGES_REFRESH_MS,
    SentMessagesMode,
    modeParam,
    parseMode,
} from "./types"

type EMessagingTab = "ACCOUNTS" | "SENT_MESSAGES"

export interface IMessagingTabsProps {
    /** The Accounts tab: the account list. */
    accounts: React.ReactNode
    /** How often the open Sent messages tab reads the list again; 0 never (stories). */
    refreshMs?: number
}

/**
 * Settings > Messaging: Accounts, and Sent messages where the dev fake
 * provider routes are mounted. The Sent messages tab exists only once
 * `messaging_fake_provider_messages` answers; where it fails (the routes
 * aren't mounted, or the user can't write accounts) there are no tabs, just
 * the accounts. There's no other switch.
 *
 * Its own query parameter, `?channel=`, keeps the mode and selects the tab:
 * the page's shared Tabs reads `tabIndex` from the URL.
 */
export const MessagingTabs: React.FC<IMessagingTabsProps> = ({
    accounts,
    refreshMs = SENT_MESSAGES_REFRESH_MS,
}) => {
    const {t} = useTranslation()
    const [searchParams, setSearchParams] = useSearchParams()
    const param = searchParams.get(SENT_MESSAGES_PARAM)
    const mode = parseMode(param)
    // Read once to decide whether the tab exists; refreshed while it's open.
    const sent = useSentMessages({active: param !== null, refreshMs})
    const getStarted = useFakeGetStarted()
    const tab: EMessagingTab = param !== null && sent.available ? "SENT_MESSAGES" : "ACCOUNTS"

    const setParam = useCallback(
        (value: string | null) =>
            setSearchParams(
                (current) => {
                    const next = new URLSearchParams(current)
                    if (value === null) {
                        next.delete(SENT_MESSAGES_PARAM)
                    } else {
                        next.set(SENT_MESSAGES_PARAM, value)
                    }
                    return next
                },
                {replace: true}
            ),
        [setSearchParams]
    )

    const {refresh} = sent
    const onGetStarted = useCallback(
        async (link: string) => {
            const answer = await getStarted(link)
            refresh()
            return answer
        },
        [getStarted, refresh]
    )

    // The same tree with or without the tabs, so the account list never remounts.
    return (
        <Box sx={{display: "flex", flexDirection: "column"}}>
            {sent.available ? (
                <Tabs
                    value={tab}
                    onChange={(_event, value: EMessagingTab) =>
                        setParam(value === "SENT_MESSAGES" ? modeParam(mode) : null)
                    }
                    aria-label={t("sentMessages.tabsLabel")}
                    variant="scrollable"
                    allowScrollButtonsMobile
                    sx={{px: 2, borderBottom: 1, borderColor: "divider"}}
                >
                    <Tab value="ACCOUNTS" label={t("sentMessages.accountsTab")} />
                    <Tab value="SENT_MESSAGES" label={t("sentMessages.tab")} />
                </Tabs>
            ) : null}
            {tab === "SENT_MESSAGES" ? (
                <Box sx={{p: 2}}>
                    <SentMessages
                        messages={sent.messages}
                        mode={mode}
                        onModeChange={(next: SentMessagesMode) => setParam(modeParam(next))}
                        onRefresh={sent.refresh}
                        refreshing={sent.refreshing}
                        refreshFailed={!!sent.error}
                        onGetStarted={onGetStarted}
                    />
                </Box>
            ) : (
                accounts
            )}
        </Box>
    )
}
