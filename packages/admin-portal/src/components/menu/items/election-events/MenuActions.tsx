// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {RefObject} from "react"
import {useNavigate} from "react-router-dom"
import {useDelete, useNotify, useRedirect, useUpdate} from "react-admin"
import MoreHorizIcon from "@mui/icons-material/MoreHoriz"
import AddCircleIcon from "@mui/icons-material/AddCircle"
import DeleteIcon from "@mui/icons-material/Delete"
import InventoryIcon from "@mui/icons-material/Inventory"
import {Divider, ListItemIcon, MenuItem, MenuList, Popover} from "@mui/material"
import {Dialog, adminTheme} from "@sequentech/ui-essentials"
import {DataTreeMenuType, ResourceName} from "../ElectionEvents"
import {getNavLinkCreate, mapAddResource, mapImportResource} from "./TreeMenu"
import {useActionPermissions, useTreeMenuData} from "../use-tree-menu-hook"
import {useTranslation} from "react-i18next"
import {styled} from "@mui/material/styles"
import {divContainer} from "@/components/styles/Menu"
import {useApolloClient, useMutation} from "@apollo/client"
import {DeleteElectionEvent, DeleteElectionEventMutation} from "@/gql/graphql"
import {DELETE_ELECTION_EVENT} from "@/queries/DeleteElectionEvent"
import {IPermissions} from "@/types/keycloak"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import {useCreateElectionEventStore} from "@/providers/CreateElectionEventContextProvider"
import {useWidgetStore} from "@/providers/WidgetsContextProvider"
import {WidgetProps} from "@/components/Widget"
import {ETasksExecution} from "@/types/tasksExecution"
import {isBallotBoxSealedError, sealText} from "@/services/ballotBoxSealErrors"
import {GET_BALLOT_BOX_SEALS, GET_EVENT_BALLOT_BOX_SEALS} from "@/queries/GetBallotBoxSeals"
import type {
    GetBallotBoxSealsQuery,
    GetBallotBoxSealsQueryVariables,
    GetEventBallotBoxSealsQuery,
} from "@/types/ballotBoxSeal"
import {useSealReadRole} from "@/hooks/useSealReadRole"
import {isSealAtClose} from "@/services/ballotBoxSealPolicy"

/** What the pre-check of a delete found about ballot box seals (VOTE-FREEZE). */
enum ESealCheck {
    /** No seals, or not an election or event. */
    NONE = "none",
    /** It has seals: it can't be deleted. */
    SEALED = "sealed",
    /** The seals could not be read. */
    UNKNOWN = "unknown",
}

const mapRemoveResource: Record<ResourceName, string> = {
    sequent_backend_election_event: "sideMenu.menuActions.remove.electionEvent",
    sequent_backend_election: "sideMenu.menuActions.remove.election",
    sequent_backend_contest: "sideMenu.menuActions.remove.contest",
    sequent_backend_candidate: "sideMenu.menuActions.remove.candidate",
}

enum Action {
    Add,
    Import,
    Remove,
    Archive,
    Unarchive,
}

type ActionPayload = {
    id: string
    name: string
    type: ResourceName
}

interface Props {
    isArchivedTab: boolean
    resourceId: string
    resourceName: string
    resourceType: ResourceName
    parentData: DataTreeMenuType
    menuItemRef: RefObject<HTMLDivElement | null>
    setAnchorEl: (val: HTMLParagraphElement | null) => void
    anchorEl: HTMLParagraphElement | null
    reloadTree: () => void
}

const StyledIconContainer = styled("p")`
    ${divContainer}
`

const StyledActionsButton = styled("button")`
    ${divContainer}
    padding: 0;
    border: 0;
    background: none;
    color: inherit;
    cursor: pointer;
`

const StyledAddCircleIcon = styled(AddCircleIcon)`
    color: ${adminTheme.palette.brandColor};
`

export default function MenuAction({
    isArchivedTab,
    resourceId,
    resourceName,
    resourceType,
    parentData,
    menuItemRef,
    setAnchorEl,
    anchorEl,
    reloadTree,
}: Props) {
    const {t, i18n} = useTranslation()

    const navigate = useNavigate()
    const redirect = useRedirect()

    const [deleteOne] = useDelete()
    const [update] = useUpdate()
    const [delete_election_event] = useMutation<DeleteElectionEventMutation>(
        DELETE_ELECTION_EVENT,
        {
            context: {
                headers: {
                    "x-hasura-role": IPermissions.ELECTION_EVENT_DELETE,
                },
            },
        }
    )

    const notify = useNotify()

    const {data: treeData, refetch} = useTreeMenuData(isArchivedTab)

    const [openArchiveModal, setOpenArchiveModal] = React.useState(false)
    const [openDeleteModal, setOpenDeleteModal] = React.useState(false)
    // What the pre-check found about the ballot box seals of the election or
    // event to remove (VOTE-FREEZE): sealed ones can't be deleted.
    const [sealCheck, setSealCheck] = React.useState<ESealCheck>(ESealCheck.NONE)
    const apollo = useApolloClient()
    const sealRole = useSealReadRole()

    /**
     * Whether the election or event has ballot box seals. The database (or
     * the delete task) refuses to delete it then, and outside Hasura's dev
     * mode its error doesn't say why, so the dialog says it before. The seals
     * are read with a role the user has; when none can read them, the dialog
     * says they could not be checked.
     */
    const checkSeals = async (payload: ActionPayload): Promise<ESealCheck> => {
        const isElection = payload.type === "sequent_backend_election"
        if (!isElection && payload.type !== "sequent_backend_election_event") {
            return ESealCheck.NONE
        }
        // Only an event that seals at close has seals: any other is deleted
        // as before, without reading them.
        const eventId = String(isElection ? parentData.id : payload.id)
        const event = (
            treeData?.sequent_backend_election_event as
                | Array<{id: string; presentation?: unknown}>
                | undefined
        )?.find((item) => String(item.id) === eventId)
        if (event && !isSealAtClose(event.presentation)) return ESealCheck.NONE
        if (!sealRole) return ESealCheck.UNKNOWN
        try {
            const context = {headers: {"x-hasura-role": sealRole}}
            const {data} = isElection
                ? await apollo.query<GetBallotBoxSealsQuery, GetBallotBoxSealsQueryVariables>({
                      query: GET_BALLOT_BOX_SEALS,
                      variables: {
                          electionEventId: parentData.id,
                          electionIds: [String(payload.id)],
                      },
                      fetchPolicy: "network-only",
                      context,
                  })
                : await apollo.query<GetEventBallotBoxSealsQuery>({
                      query: GET_EVENT_BALLOT_BOX_SEALS,
                      variables: {electionEventId: String(payload.id)},
                      fetchPolicy: "network-only",
                      context,
                  })
            return data?.sequent_backend_ballot_box_seal?.length
                ? ESealCheck.SEALED
                : ESealCheck.NONE
        } catch {
            return ESealCheck.UNKNOWN
        }
    }
    const [selectedActionModal, setSelectedActionModal] = React.useState<{
        action: Action
        payload: ActionPayload
    } | null>(null)
    const [addWidget, setWidgetTaskId, updateWidgetFail] = useWidgetStore()
    const isItemElectionEventType = resourceType === "sequent_backend_election_event"
    const {setElectionEventIdFlag, setElectionIdFlag, setContestIdFlag, setCandidateIdFlag} =
        useElectionEventTallyStore()

    function handleOpenItemActions(): void {
        setAnchorEl(menuItemRef.current)
    }

    async function handleAction(action: Action, payload: ActionPayload) {
        // close the popover
        setAnchorEl(null)

        if (action === Action.Add) {
            navigate(getNavLinkCreate(parentData, payload.type))
        } else if (
            payload.type === "sequent_backend_election_event" &&
            (action === Action.Archive || action === Action.Unarchive)
        ) {
            setSelectedActionModal({action, payload})
            setOpenArchiveModal(true)
        } else if (action === Action.Remove) {
            setSealCheck(await checkSeals(payload))
            setSelectedActionModal({action, payload})
            setOpenDeleteModal(true)
        }
    }

    function handleCloseActionMenu() {
        setAnchorEl(null)
    }

    async function confirmArchiveAction() {
        if (!selectedActionModal) {
            return
        }

        const {action, payload} = selectedActionModal

        if (action === Action.Archive) {
            update(
                payload.type,
                {
                    id: payload.id,
                    data: {is_archived: true},
                    previousData: {is_archived: false},
                },
                {
                    onSuccess() {
                        refetch()
                        notify(t("sideMenu.menuActions.messages.notification.success.archive"), {
                            type: "success",
                        })
                    },
                    onError() {
                        notify(t("sideMenu.menuActions.messages.notification.error.archive"), {
                            type: "error",
                        })
                    },
                }
            )
        } else if (action === Action.Unarchive) {
            await update(
                payload.type,
                {
                    id: payload.id,
                    data: {is_archived: false},
                    previousData: {is_archived: true},
                },

                {
                    onSuccess() {
                        refetch()
                        notify(t("sideMenu.menuActions.messages.notification.success.unarchive"), {
                            type: "success",
                        })
                    },
                    onError() {
                        notify(t("sideMenu.menuActions.messages.notification.error.unarchive"), {
                            type: "error",
                        })
                    },
                }
            )
        }
    }

    const deleteElectionEventAction = async (payload: ActionPayload) => {
        const currWidget: WidgetProps = addWidget(ETasksExecution.DELETE_ELECTION_EVENT, undefined)
        try {
            const {data, errors} = await delete_election_event({
                variables: {
                    electionEventId: payload.id,
                },
            })

            const errorMessage = data?.delete_election_event?.error_msg
            if (errorMessage || errors) {
                // Say why (e.g. the event has sealed ballot boxes), and let the
                // widget show the task's log when there is a task.
                if (errorMessage) notify(sealText(t, errorMessage), {type: "error"})
                const failedTask = data?.delete_election_event?.task_execution?.id
                if (failedTask) {
                    setWidgetTaskId(currWidget.identifier, failedTask)
                } else {
                    updateWidgetFail(currWidget.identifier)
                }
                return
            }
            const taskId = data?.delete_election_event?.task_execution?.id
            setWidgetTaskId(currWidget.identifier, taskId, () => {
                setSelectedActionModal(null)
                setElectionEventIdFlag(null)
                setElectionIdFlag(null)
                setContestIdFlag(null)
                reloadTree()
            })
        } catch (error) {
            updateWidgetFail(currWidget.identifier)
        }
    }

    async function confirmDeleteAction() {
        const payload = selectedActionModal?.payload ?? null

        if (!payload) {
            return
        }

        if (payload.type === "sequent_backend_election_event") {
            deleteElectionEventAction(payload)
        } else {
            deleteOne(
                payload.type,
                {id: payload.id},
                {
                    onSuccess: () => {
                        reloadTree()
                        refetch()

                        notify(t("sideMenu.menuActions.messages.notification.success.delete"), {
                            type: "success",
                        })
                        if (parentData?.__typename === "sequent_backend_election_event") {
                            setElectionEventIdFlag("")
                            setElectionIdFlag("")
                            navigate("/sequent_backend_election_event/" + parentData.id)
                        }
                        if (parentData?.__typename === "sequent_backend_election") {
                            setElectionIdFlag("")
                            setContestIdFlag("")
                            navigate("/sequent_backend_election/" + parentData.id)
                        }
                        if (parentData?.__typename === "sequent_backend_contest") {
                            setElectionIdFlag("")
                            setContestIdFlag("")
                            navigate("/sequent_backend_contest/" + parentData.id)
                        }
                    },
                    onError: (error: unknown) => {
                        setOpenDeleteModal(false)
                        // A sealed election can't be deleted (VOTE-FREEZE): say why.
                        notify(
                            t(
                                isBallotBoxSealedError(error)
                                    ? "sideMenu.menuActions.messages.notification.error.deleteSealedElection"
                                    : sealCheck === ESealCheck.UNKNOWN
                                      ? "sideMenu.menuActions.messages.notification.error.deleteMaybeSealed"
                                      : "sideMenu.menuActions.messages.notification.error.delete"
                            ),
                            {type: "error"}
                        )
                    },
                    onSettled: () => {
                        setSelectedActionModal(null)
                    },
                }
            )
        }
    }

    const openActionMenu = Boolean(anchorEl)
    const idActionMenu = openActionMenu ? "action-menu" : undefined

    /**
     * Permissions
     */

    const {
        canCreateElectionEvent,
        canDeleteElectionEvent,
        canArchiveElectionEvent,
        canCreateContest,
        canDeleteContest,
        canCreateCandidate,
        canDeleteCandidate,
        canCreateElection,
        canDeleteElection,
    } = useActionPermissions()

    const canShowCreate =
        (resourceType === "sequent_backend_election_event" && canCreateElectionEvent) ||
        (resourceType === "sequent_backend_election" && canCreateElection) ||
        (resourceType === "sequent_backend_contest" && canCreateContest && canCreateElection) ||
        (resourceType === "sequent_backend_candidate" &&
            canCreateCandidate &&
            canCreateElection &&
            canCreateContest)

    const canShowDelete =
        (resourceType === "sequent_backend_election_event" && canDeleteElectionEvent) ||
        (resourceType === "sequent_backend_election" && canDeleteElection) ||
        (resourceType === "sequent_backend_contest" && canDeleteContest && canDeleteElection) ||
        (resourceType === "sequent_backend_candidate" &&
            canDeleteCandidate &&
            canDeleteElection &&
            canDeleteContest)
    /**
     * ======
     */

    /**
     * Create and import ee from drawer
     */
    const {openCreateDrawer, openImportDrawer} = useCreateElectionEventStore()

    const handleOpenCreateElectionEventForm = (e: React.MouseEvent<HTMLElement>) => {
        setAnchorEl(null)
        openCreateDrawer?.()
    }

    const handleOpenImportElectionEventForm = (e: React.MouseEvent<HTMLElement>) => {
        setAnchorEl(null)
        openImportDrawer?.()
    }
    /**
     * ======
     */

    return (
        <>
            <StyledIconContainer>
                {((!isArchivedTab && (canShowCreate || canShowDelete || canArchiveElectionEvent)) ||
                    (isArchivedTab && (canArchiveElectionEvent || canShowDelete))) && (
                    <StyledActionsButton
                        type="button"
                        aria-label={`${t("common.label.actions")}: ${resourceName}`}
                        aria-haspopup="menu"
                        onClick={handleOpenItemActions}
                    >
                        <MoreHorizIcon id={"MoreHorizIcon"} />
                    </StyledActionsButton>
                )}
            </StyledIconContainer>
            <Popover
                id={idActionMenu}
                open={openActionMenu}
                anchorEl={anchorEl}
                onClose={handleCloseActionMenu}
                anchorOrigin={{
                    vertical: "bottom",
                    horizontal: "right",
                }}
            >
                <MenuList dense>
                    {!isArchivedTab && canShowCreate && (
                        <MenuItem
                            dir={i18n.dir(i18n.language)}
                            key={Action.Add}
                            className={`menu-action-add-${resourceType}`}
                            onClick={(e) => {
                                if (resourceType === "sequent_backend_election_event") {
                                    handleOpenCreateElectionEventForm(e)
                                } else {
                                    handleAction(Action.Add, {
                                        id: resourceId,
                                        name: resourceName,
                                        type: resourceType,
                                    })
                                }
                            }}
                        >
                            <ListItemIcon>
                                <StyledAddCircleIcon />
                            </ListItemIcon>
                            {t(mapAddResource[resourceType])}
                        </MenuItem>
                    )}

                    {isItemElectionEventType &&
                        !isArchivedTab &&
                        canShowCreate &&
                        canShowDelete && <Divider key="divider0" />}

                    {!isArchivedTab &&
                    canShowCreate &&
                    resourceType === "sequent_backend_election_event" ? (
                        <MenuItem
                            dir={i18n.dir(i18n.language)}
                            key={Action.Import}
                            className={`menu-action-add-${resourceType}`}
                            onClick={handleOpenImportElectionEventForm}
                        >
                            <ListItemIcon>
                                <StyledAddCircleIcon />
                            </ListItemIcon>
                            {t(mapImportResource[resourceType])}
                        </MenuItem>
                    ) : null}

                    {isItemElectionEventType &&
                        !isArchivedTab &&
                        canShowCreate &&
                        canShowDelete && <Divider key="divider1" />}

                    {isItemElectionEventType && canArchiveElectionEvent && (
                        <MenuItem
                            dir={i18n.dir(i18n.language)}
                            key={Action.Archive}
                            className={`menu-action-archive-${resourceType}`}
                            onClick={() =>
                                handleAction(isArchivedTab ? Action.Unarchive : Action.Archive, {
                                    id: resourceId,
                                    name: resourceName,
                                    type: resourceType,
                                })
                            }
                        >
                            <ListItemIcon>
                                <InventoryIcon color="error" />
                            </ListItemIcon>
                            {isArchivedTab
                                ? t("sideMenu.menuActions.unarchive.electionEvent")
                                : t("sideMenu.menuActions.archive.electionEvent")}
                        </MenuItem>
                    )}

                    {canArchiveElectionEvent && canShowCreate && canShowDelete && (
                        <Divider key="divider2" />
                    )}

                    {canShowDelete && (
                        <MenuItem
                            dir={i18n.dir(i18n.language)}
                            key={Action.Remove}
                            className={`menu-action-delete-${resourceType}`}
                            onClick={() =>
                                handleAction(Action.Remove, {
                                    id: resourceId,
                                    name: resourceName,
                                    type: resourceType,
                                })
                            }
                        >
                            <ListItemIcon>
                                <DeleteIcon color="error" />
                            </ListItemIcon>

                            {t(mapRemoveResource[resourceType])}
                        </MenuItem>
                    )}
                </MenuList>
            </Popover>

            <Dialog
                variant="warning"
                open={openArchiveModal}
                ok={
                    selectedActionModal?.action === Action.Archive
                        ? t("common.label.archive")
                        : t("common.label.unarchive")
                }
                cancel={String(t("common.label.cancel"))}
                title={String(t("common.label.warning"))}
                handleClose={(result: boolean) => {
                    if (result) {
                        confirmArchiveAction()
                    }
                    setOpenArchiveModal(false)
                }}
            >
                {selectedActionModal?.action === Action.Archive
                    ? t("sideMenu.menuActions.messages.confirm.archive")
                    : t("sideMenu.menuActions.messages.confirm.unarchive")}
            </Dialog>

            <Dialog
                variant="warning"
                open={openDeleteModal}
                ok={sealCheck === ESealCheck.SEALED ? undefined : String(t("common.label.delete"))}
                cancel={String(t("common.label.cancel"))}
                title={String(t("common.label.warning"))}
                handleClose={(result: boolean) => {
                    if (result && sealCheck !== ESealCheck.SEALED) {
                        confirmDeleteAction()
                    }
                    setOpenDeleteModal(false)
                }}
            >
                {sealCheck === ESealCheck.SEALED
                    ? t(
                          selectedActionModal?.payload.type === "sequent_backend_election_event"
                              ? "sideMenu.menuActions.messages.notification.error.deleteSealedEvent"
                              : "sideMenu.menuActions.messages.notification.error.deleteSealedElection"
                      )
                    : sealCheck === ESealCheck.UNKNOWN
                      ? `${t("sideMenu.menuActions.messages.confirm.delete")} ${t(
                            "sideMenu.menuActions.messages.confirm.sealsUnknown"
                        )}`
                      : t("sideMenu.menuActions.messages.confirm.delete")}
            </Dialog>
        </>
    )
}
