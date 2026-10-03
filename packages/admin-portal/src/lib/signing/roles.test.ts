// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {readFileSync} from "fs"
import {join} from "path"
import {parse} from "yaml"
import {IPermissions} from "@/types/keycloak"
import {SigningOperation, signingOperationRole} from "./roles"
import {DocumentKind, SIGNING_ACTIONS, SigningAction} from "./types"

const holding =
    (...held: IPermissions[]) =>
    (permission: IPermissions) =>
        held.includes(permission)

describe("signingOperationRole", () => {
    // The role must be one the Hasura action allows (actions.yaml) and the user holds.
    it.each([
        // A widget step: the request's own sign permission first.
        [
            "an admin signs with the action's sign permission",
            SigningOperation.Sign,
            SigningAction.CloseVoting,
            [IPermissions.ADMIN_USER, IPermissions.SIGN_CLOSE_VOTING],
            IPermissions.SIGN_CLOSE_VOTING,
        ],
        [
            "an admin without it signs as admin-user, so Harvest answers",
            SigningOperation.Sign,
            SigningAction.CloseVoting,
            [IPermissions.ADMIN_USER, IPermissions.SIGN_OPEN_VOTING],
            IPermissions.ADMIN_USER,
        ],
        [
            "a signer without admin-user signs with the action's sign permission",
            SigningOperation.Sign,
            SigningAction.ApproveConfiguration,
            [IPermissions.SIGN_APPROVE_CONFIGURATION],
            IPermissions.SIGN_APPROVE_CONFIGURATION,
        ],
        [
            "a signer of another action sends a sign permission it holds",
            SigningOperation.Sign,
            SigningAction.CloseVoting,
            [IPermissions.SIGN_APPROVE_VOTER],
            IPermissions.SIGN_APPROVE_VOTER,
        ],
        [
            "before the request loads, a signer sends a sign permission it holds",
            SigningOperation.Sign,
            null,
            [IPermissions.SIGNING_REQUESTS_READ, IPermissions.SIGN_TALLY_KEY],
            IPermissions.SIGN_TALLY_KEY,
        ],
        // Loading the request: the sign permission, then the read permission.
        [
            "a signer loads the request with the action's sign permission",
            SigningOperation.GetRequest,
            SigningAction.CloseVoting,
            [IPermissions.SIGNING_REQUESTS_READ, IPermissions.SIGN_CLOSE_VOTING],
            IPermissions.SIGN_CLOSE_VOTING,
        ],
        [
            "a reader loads it with signing-requests-read",
            SigningOperation.GetRequest,
            SigningAction.CloseVoting,
            [IPermissions.SIGNING_REQUESTS_READ, IPermissions.SIGN_APPROVE_VOTER],
            IPermissions.SIGNING_REQUESTS_READ,
        ],
        [
            "an admin loads an unknown request as admin-user",
            SigningOperation.GetRequest,
            null,
            [IPermissions.ADMIN_USER, IPermissions.SIGN_CLOSE_VOTING],
            IPermissions.ADMIN_USER,
        ],
        [
            "a signer loads an unknown request with a sign permission it holds",
            SigningOperation.GetRequest,
            null,
            [IPermissions.ELECTION_EVENT_READ, IPermissions.SIGN_CLOSE_VOTING],
            IPermissions.SIGN_CLOSE_VOTING,
        ],
        // Cancelling: the cancel permission, then the action's sign permission (the
        // requester's own request), then admin-user.
        [
            "an operator cancels with signing-requests-cancel",
            SigningOperation.Cancel,
            SigningAction.CloseVoting,
            [IPermissions.ADMIN_USER, IPermissions.SIGNING_REQUESTS_CANCEL],
            IPermissions.SIGNING_REQUESTS_CANCEL,
        ],
        [
            "a requester without admin-user cancels with the action's sign permission",
            SigningOperation.Cancel,
            SigningAction.CloseVoting,
            [IPermissions.SIGN_CLOSE_VOTING],
            IPermissions.SIGN_CLOSE_VOTING,
        ],
        [
            "an admin requester cancels with the action's sign permission",
            SigningOperation.Cancel,
            SigningAction.CloseVoting,
            [IPermissions.ADMIN_USER, IPermissions.SIGN_CLOSE_VOTING],
            IPermissions.SIGN_CLOSE_VOTING,
        ],
        [
            "an admin requester without it cancels as admin-user",
            SigningOperation.Cancel,
            SigningAction.CloseVoting,
            [IPermissions.ADMIN_USER, IPermissions.SIGN_OPEN_VOTING],
            IPermissions.ADMIN_USER,
        ],
        [
            "a requester who only starts the action cancels with its start permission",
            SigningOperation.Cancel,
            SigningAction.CloseVoting,
            [IPermissions.ELECTION_STATE_WRITE],
            IPermissions.ELECTION_STATE_WRITE,
        ],
        [
            "a requester who only initializes voting cancels with admin-ceremony",
            SigningOperation.Cancel,
            SigningAction.InitializeVoting,
            [IPermissions.ADMIN_CEREMONY, IPermissions.PUBLISH_WRITE],
            IPermissions.ADMIN_CEREMONY,
        ],
    ])("%s", (_label, operation, action, held, role) => {
        expect(signingOperationRole(operation, action, holding(...held))).toBe(role)
    })

    it.each([
        [SigningOperation.Sign, [IPermissions.SIGNING_REQUESTS_READ]],
        [SigningOperation.GetRequest, [IPermissions.ELECTION_EVENT_READ]],
        // signingCancel goes with the request's own sign permission, not another action's.
        [SigningOperation.Cancel, [IPermissions.SIGN_OPEN_VOTING]],
    ])("names no role for %s when the user holds none the action allows", (operation, held) => {
        expect(signingOperationRole(operation, SigningAction.CloseVoting, holding(...held))).toBe(
            null
        )
    })

    it("names no sign permission to cancel a request whose action isn't known yet", () => {
        expect(
            signingOperationRole(
                SigningOperation.Cancel,
                null,
                holding(IPermissions.SIGN_CLOSE_VOTING)
            )
        ).toBe(null)
    })
})

describe("the roles the widget sends", () => {
    // hasura/metadata/actions.yaml: Hasura serves an action only to the roles it lists.
    const metadata = parse(
        readFileSync(join(__dirname, "../../../../../hasura/metadata/actions.yaml"), "utf8")
    ) as {actions: Array<{name: string; permissions?: Array<{role: string}>}>}
    const allowed = (name: string): Set<string> => {
        const action = metadata.actions.find((candidate) => candidate.name === name)
        expect(action).toBeDefined()
        return new Set((action?.permissions ?? []).map(({role}) => role))
    }
    const ACTIONS: Record<SigningOperation, string[]> = {
        [SigningOperation.GetRequest]: ["signingGetRequest"],
        [SigningOperation.Sign]: [
            "signingCheckCertificate",
            "signingPdfPrepare",
            "signingApprove",
            "signingOpenFailure",
            "signingHandover",
        ],
        [SigningOperation.Cancel]: ["signingCancel"],
    }
    const permissions = Object.values(IPermissions)

    it.each(Object.values(SigningOperation))(
        "are ones Hasura allows for %s, whichever permission the user holds",
        (operation) => {
            for (const action of [...Object.values(SigningAction), null]) {
                // Only a holder of the request's sign permission signs it.
                const signer =
                    operation === SigningOperation.Sign && action
                        ? SIGNING_ACTIONS[action].signPermission
                        : null
                for (const held of permissions) {
                    const role = signingOperationRole(
                        operation,
                        action,
                        (p) => p === held || p === signer
                    )
                    if (role === null) continue
                    for (const name of ACTIONS[operation]) {
                        // Only a loaded PDF request prepares a PDF revision.
                        if (
                            name === "signingPdfPrepare" &&
                            (!action || SIGNING_ACTIONS[action].document !== DocumentKind.Pdf)
                        ) {
                            continue
                        }
                        expect([name, action, role, allowed(name).has(role)]).toEqual([
                            name,
                            action,
                            role,
                            true,
                        ])
                    }
                }
            }
        }
    )
})
