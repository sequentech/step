// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {IPermissions} from "@/types/keycloak"

/**
 * Whether an event shows its Messaging tab. Reading needs
 * messaging-config-read; a user who can only write sees the tab too, where it
 * says that reading needs messaging-config-read, instead of the tab vanishing
 * with no reason.
 */
export const showMessagingTab = (
    isAuthorized: (permissions: IPermissions | IPermissions[]) => boolean,
    locked: boolean
): boolean =>
    !locked &&
    isAuthorized([IPermissions.MESSAGING_CONFIG_READ, IPermissions.MESSAGING_CONFIG_WRITE])
