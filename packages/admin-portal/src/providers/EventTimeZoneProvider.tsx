// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {createContext, useContext, useMemo, type PropsWithChildren} from "react"
import {
    browserTimeZone,
    effectiveTimeZone,
    type IElectionEventPresentation,
    type IElectionPresentation,
} from "@sequentech/ui-core"

/** An election event as its screens' times need it. */
export interface IZonedEvent {
    id: string
    presentation?: IElectionEventPresentation | null
    /** The screen's zone: the election's on an election's screens, else the primary. */
    zone: string
}

const EventTimeZoneContext = createContext<IZonedEvent | null>(null)

/**
 * The election event whose screens are rendered below, so their times show
 * in its zones (VOTE-LIFECYCLE) without reading the event again. On an
 * election's screens, `election` makes them show in the election's zone.
 */
export const EventTimeZoneProvider: React.FC<
    PropsWithChildren<{
        event?: {id: unknown; presentation?: unknown} | null
        election?: {presentation?: unknown} | null
    }>
> = ({event, election, children}) => {
    const value = useMemo<IZonedEvent | null>(() => {
        if (!event?.id) return null
        const presentation = (event.presentation ?? null) as IElectionEventPresentation | null
        return {
            id: String(event.id),
            presentation,
            zone: effectiveTimeZone(
                presentation,
                (election?.presentation ?? null) as IElectionPresentation | null
            ),
        }
    }, [event?.id, event?.presentation, election?.presentation])
    return <EventTimeZoneContext.Provider value={value}>{children}</EventTimeZoneContext.Provider>
}

/** The event of the screen, if it is rendered inside one. */
export const useZonedEvent = (): IZonedEvent | null => useContext(EventTimeZoneContext)

const MyTimeZoneContext = createContext<string | null>(null)

/**
 * "My time": the zone of the person looking, which is the browser's. Stories
 * and tests set it explicitly so they never depend on the machine's zone.
 */
export const MyTimeZoneProvider: React.FC<PropsWithChildren<{zone: string}>> = ({
    zone,
    children,
}) => <MyTimeZoneContext.Provider value={zone}>{children}</MyTimeZoneContext.Provider>

/** The viewer's zone: the one set above, else the browser's. */
export const useMyTimeZone = (): string => useContext(MyTimeZoneContext) ?? browserTimeZone()
