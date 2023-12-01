// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {RefObject} from "react"
import {useNavigate} from "react-router-dom"
import {useDelete, useNotify, useUpdate} from "react-admin"
import MoreHorizIcon from "@mui/icons-material/MoreHoriz"
import AddCircleIcon from "@mui/icons-material/AddCircle"
import DeleteIcon from "@mui/icons-material/Delete"
import InventoryIcon from "@mui/icons-material/Inventory"
import {Divider, ListItemIcon, MenuItem, MenuList, Popover} from "@mui/material"
import {Dialog} from "@sequentech/ui-essentials"
import {DataTreeMenuType, ResourceName} from "../ElectionEvents"
import {getNavLinkCreate, mapAddResource} from "./TreeMenu"
import {useActionPermissions, useTreeMenuData} from "../use-tree-menu-hook"
import {useTranslation} from "react-i18next"

const mapRemoveResource: Record<ResourceName, string> = {
    sequent_backend_election_event: "sideMenu.menuActions.remove.electionEvent",
    sequent_backend_election: "sideMenu.menuActions.remove.election",
    sequent_backend_contest: "sideMenu.menuActions.remove.contest",
    sequent_backend_candidate: "sideMenu.menuActions.remove.candidate",
}

enum Action {
    Add,
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
}

export default function MenuAction({
    isArchivedTab,
    resourceId,
    resourceName,
    resourceType,
    parentData,
    menuItemRef,
}: Props) {
    const {t} = useTranslation()

    const navigate = useNavigate()

    const [deleteOne] = useDelete()
    const [update] = useUpdate()

    const notify = useNotify()

    // TODO : need to change this use query
    const {refetch} = useTreeMenuData(false)

    const {canCreateElectionEvent, canEditElectionEvent} = useActionPermissions()

    const [openArchiveModal, setOpenArchiveModal] = React.useState(false)
    const [openDeleteModal, setOpenDeleteModal] = React.useState(false)
    const [selectedActionModal, setSelectedActionModal] = React.useState<{
        action: Action
        payload: ActionPayload
    } | null>(null)

    const [anchorEl, setAnchorEl] = React.useState<HTMLParagraphElement | null>(null)

    const isItemElectionEventType = resourceType === "sequent_backend_election_event"

    function handleOpenItemActions(): void {
        setAnchorEl(menuItemRef.current)
    }

    async function handleAction(action: Action, payload: ActionPayload) {
        console.log(
            "LS -> src/components/menu/items/election-events/MenuActions.tsx:83 -> action: ",
            action
        )

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
            console.log(
                "LS -> src/components/menu/items/election-events/MenuActions.tsx:92 -> action: ",
                action
            )

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

    function confirmDeleteAction() {
        const payload = selectedActionModal?.payload ?? null

        if (!payload) {
            return
        }

        deleteOne(
            payload.type,
            {id: payload.id},
            {
                onSuccess: () => {
                    refetch()
                    notify(t("sideMenu.menuActions.messages.notification.success.delete"), {
                        type: "success",
                    })
                },
                onError: () => {
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

    const openActionMenu = Boolean(anchorEl)
    const idActionMenu = openActionMenu ? "action-menu" : undefined

    return (
        <>
            <p className="flex-none w-6 h-6 cursor-pointer" onClick={handleOpenItemActions}>
                <MoreHorizIcon />
            </p>
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
                    {!isArchivedTab && canCreateElectionEvent && (
                        <MenuItem
                            key={Action.Add}
                            onClick={() =>
                                handleAction(Action.Add, {
                                    id: resourceId,
                                    name: resourceName,
                                    type: resourceType,
                                })
                            }
                        >
                            <ListItemIcon>
                                <AddCircleIcon className="text-brand-color" />
                            </ListItemIcon>
                            {t(mapAddResource[resourceType])}
                        </MenuItem>
                    )}

                    {isItemElectionEventType && !isArchivedTab && canEditElectionEvent && (
                        <Divider key="divider1" />
                    )}

                    {isItemElectionEventType && canEditElectionEvent && (
                        <MenuItem
                            key={Action.Archive}
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

                    {!isArchivedTab && canEditElectionEvent && <Divider key="divider2" />}

                    {!isArchivedTab && canEditElectionEvent && (
                        <MenuItem
                            key={Action.Remove}
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
