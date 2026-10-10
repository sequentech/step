// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {useMemo} from "react"
import {useQuery} from "@apollo/client"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {
    GET_EVENT_MESSAGING_CONFIG,
    GET_EVENT_MESSAGING_CONFIG_AUTHOR,
    IGetEventMessagingConfigAuthorResult,
    IGetEventMessagingConfigResult,
} from "@/queries/GetEventMessagingConfig"
import {parseEventMessagingConfig} from "@/services/messaging"
import {roleContext, useMessagingReadRole} from "./useMessagingReadRole"

/**
 * The event's messaging configuration from `messaging_event_config`, read
 * with messaging-config-read. An event without a row has an empty `config`
 * and a null `storedConfig`. `canRead` is false without the role.
 */
export const useEventMessagingConfig = (
    electionEventId: string | null | undefined,
    options: {skip?: boolean} = {}
) => {
    const [tenantId] = useTenantStore()
    const role = useMessagingReadRole()
    const {data, loading, error, refetch} = useQuery<IGetEventMessagingConfigResult>(
        GET_EVENT_MESSAGING_CONFIG,
        {
            variables: {tenantId, electionEventId},
            skip: options.skip || !tenantId || !electionEventId || !role,
            context: roleContext(role),
        }
    )
    const row = data?.sequent_backend_messaging_event_config?.[0]
    const stored = row?.config
    const config = useMemo(() => parseEventMessagingConfig(stored ?? null), [stored])
    // An event without a row isn't the same as one whose configuration is empty:
    // the backend sends notices by email and SMS only without a row.
    const storedConfig = row ? config : null
    return {config, storedConfig, loading, error, refetch, canRead: !!role}
}

/**
 * Who last wrote the event's configuration (`updated_by`), or undefined when
 * it can't be read. Separate from the configuration, so its failure never
 * hides the tab.
 */
export const useEventMessagingConfigAuthor = (electionEventId: string | null | undefined) => {
    const [tenantId] = useTenantStore()
    const role = useMessagingReadRole()
    const {data, refetch} = useQuery<IGetEventMessagingConfigAuthorResult>(
        GET_EVENT_MESSAGING_CONFIG_AUTHOR,
        {
            variables: {tenantId, electionEventId},
            skip: !tenantId || !electionEventId || !role,
            context: roleContext(role),
        }
    )
    return {updatedBy: data?.sequent_backend_messaging_event_config?.[0]?.updated_by, refetch}
}
