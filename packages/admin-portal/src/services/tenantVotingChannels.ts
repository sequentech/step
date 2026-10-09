// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {IVotingChannelsConfig} from "@sequentech/ui-core"

export type TenantVotingChannels = Pick<IVotingChannelsConfig, "online" | "kiosk" | "telephone">
export type TenantVotingChannel = keyof TenantVotingChannels

// `??`, not `||`: disabling a channel stores `false`, and `false || true`
// reads back as enabled, so the online toggle could never be turned off.
export const toTenantVotingChannels = (
    channels?: Partial<TenantVotingChannels> | null
): TenantVotingChannels => ({
    online: channels?.online ?? true,
    kiosk: channels?.kiosk ?? false,
    telephone: channels?.telephone ?? false,
})
