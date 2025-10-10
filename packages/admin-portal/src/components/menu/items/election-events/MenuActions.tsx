// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
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
import styled from "@emotion/styled"
import {divContainer} from "@/components/styles/Menu"
import {useMutation} from "@apollo/client"
import {DeleteElectionEvent, DeleteElectionEventMutation} from "@/gql/graphql"
import {DELETE_ELECTION_EVENT} from "@/queries/DeleteElectionEvent"
import {IPermissions} from "@/types/keycloak"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import {useCreateElectionEventStore} from "@/providers/CreateElectionEventContextProvider"
import {useWidgetStore} from "@/providers/WidgetsContextProvider"
import {WidgetProps} from "@/components/Widget"
import {ETasksExecution} from "@/types/tasksExecution"

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

    const {refetch} = useTreeMenuData(isArchivedTab)

    const [openArchiveModal, setOpenArchiveModal] = React.useState(false)
    const [openDeleteModal, setOpenDeleteModal] = React.useState(false)
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

            if (data?.delete_election_event?.error_msg || errors) {
                updateWidgetFail(currWidget.identifier)
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
                    onError: () => {
                        setOpenDeleteModal(false)
                        notify(t("sideMenu.menuActions.messages.notification.error.delete"), {
                            type: "error",
                        })
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

    const StyledIconContainer = styled.p`
        ${divContainer}
        cursor: pointer
    `

    const StyledAddCircleIcon = styled(AddCircleIcon)`
        color: ${adminTheme.palette.brandColor};
    `

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
            <StyledIconContainer onClick={handleOpenItemActions}>
                {((!isArchivedTab && (canShowCreate || canShowDelete || canArchiveElectionEvent)) ||
                    (isArchivedTab && (canArchiveElectionEvent || canShowDelete))) && (
                    <MoreHorizIcon id={"MoreHorizIcon"} />
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
                cancel={t("common.label.cancel")}
                title={t("common.label.warning")}
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
                ok={t("common.label.delete")}
                cancel={t("common.label.cancel")}
                title={t("common.label.warning")}
                handleClose={(result: boolean) => {
                    if (result) {
                        confirmDeleteAction()
                    }
                    setOpenDeleteModal(false)
                }}
            >
                {t("sideMenu.menuActions.messages.confirm.delete")}
            </Dialog>
        </>
    )
}
