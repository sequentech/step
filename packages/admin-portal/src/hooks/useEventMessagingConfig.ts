// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {useMemo} from "react"
import {useQuery} from "@apollo/client"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {
    GET_EVENT_MESSAGING_CONFIG,
    IGetEventMessagingConfigResult,
} from "@/queries/GetEventMessagingConfig"
import {parseEventMessagingConfig} from "@/services/messaging"
import {roleContext, useMessagingReadRole} from "./useMessagingReadRole"

/**
 * The event's messaging configuration from `messaging_event_config`, read
 * with messaging-config-read. An event without a row has an empty
 * configuration. `canRead` is false without the role.
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
    const stored = data?.sequent_backend_messaging_event_config?.[0]?.config
    const config = useMemo(() => parseEventMessagingConfig(stored ?? null), [stored])
    return {config, loading, error, refetch, canRead: !!role}
}
