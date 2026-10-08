// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The Hasura role of each signing widget call. Hasura serves an action only to
// the roles its permissions list (hasura/metadata/actions.yaml) and refuses a
// role the user doesn't hold, so the portal's default role is no good for
// staff without admin-user. Harvest authorizes from the JWT itself.

import {IPermissions} from "@/types/keycloak"
import {SIGNING_ACTIONS, SigningAction} from "./types"

/** The widget's calls, grouped by the roles their Hasura action allows. */
export enum SigningOperation {
    /** signingGetRequest: admin-user, every sign-<action> and signing-requests-read. */
    GetRequest = "get-request",
    /**
     * signingCheckCertificate, signingPdfPrepare, signingApprove, signingOpenFailure and
     * signingHandover: admin-user and the sign-<action> permissions.
     */
    Sign = "sign",
    /**
     * signingCancel: admin-user, signing-requests-cancel, the sign-<action> permissions and
     * the permissions that start an action (a requester cancels their own request;
     * Harvest checks they started it).
     */
    Cancel = "cancel",
}

const SIGN_PERMISSIONS = Object.values(SIGNING_ACTIONS).map((info) => info.signPermission)

/**
 * The permission of the route that starts each action, which its requester holds:
 * Initialize voting and the election returns come from the tally, Open and Close
 * voting from the Post's status, a report from its generation, a transmission
 * from its package, a voter from the application's decision, a configuration
 * version from Publish, and the trustee steps from the ceremonies.
 */
const START_PERMISSIONS: Record<SigningAction, IPermissions> = {
    [SigningAction.InitializeVoting]: IPermissions.ADMIN_CEREMONY,
    [SigningAction.OpenVoting]: IPermissions.ELECTION_STATE_WRITE,
    [SigningAction.CloseVoting]: IPermissions.ELECTION_STATE_WRITE,
    [SigningAction.GenerateElectionReturns]: IPermissions.ADMIN_CEREMONY,
    [SigningAction.GenerateReports]: IPermissions.REPORT_READ,
    [SigningAction.TransmitResults]: IPermissions.MIRU_CREATE,
    [SigningAction.ApproveVoter]: IPermissions.APPLICATION_WRITE,
    [SigningAction.ApproveConfiguration]: IPermissions.PUBLISH_WRITE,
    [SigningAction.ConfirmKeyShare]: IPermissions.TRUSTEE_CEREMONY,
    [SigningAction.ContributeKeyShare]: IPermissions.TRUSTEE_CEREMONY,
}

/**
 * The role to send: one the operation's action allows and the user holds, the
 * request's own sign permission first. `null` when the user holds none.
 */
export const signingOperationRole = (
    operation: SigningOperation,
    action: SigningAction | null,
    holds: (permission: IPermissions) => boolean
): IPermissions | null => {
    const held = (...candidates: Array<IPermissions | undefined>) =>
        candidates.find((candidate) => candidate !== undefined && holds(candidate)) ?? null
    const ownSign = action ? SIGNING_ACTIONS[action]?.signPermission : undefined
    switch (operation) {
        case SigningOperation.Cancel:
            return held(
                IPermissions.SIGNING_REQUESTS_CANCEL,
                ownSign,
                IPermissions.ADMIN_USER,
                action ? START_PERMISSIONS[action] : undefined
            )
        case SigningOperation.GetRequest:
            return held(
                ownSign,
                IPermissions.SIGNING_REQUESTS_READ,
                IPermissions.ADMIN_USER,
                ...SIGN_PERMISSIONS
            )
        case SigningOperation.Sign:
            return held(ownSign, IPermissions.ADMIN_USER, ...SIGN_PERMISSIONS)
    }
}

/** The comparison query reads subjects only for configuration signers or request readers. */
export const configurationApprovalRole = (
    holds: (permission: IPermissions) => boolean
): IPermissions | null =>
    [
        IPermissions.SIGN_APPROVE_CONFIGURATION,
        IPermissions.SIGNING_REQUESTS_READ,
        IPermissions.ADMIN_USER,
    ].find(holds) ?? null
