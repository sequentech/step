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
import {useTreeMenuData} from "../use-tree-menu-hook"
import {useTranslation} from "react-i18next"

const mapRemoveResource: Record<ResourceName, string> = {
    sequent_backend_election_event: "sideMenu.menuActions.remove.electionEvent",
    sequent_backend_election: "sideMenu.addResourmenuActions.remove.election",
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

    const {refetch} = useTreeMenuData(false)

    const [openDeleteModal, setOpenDeleteModal] = React.useState(false)
    const [deleteItem, setDeleteItem] = React.useState<any | undefined>()

    const [anchorEl, setAnchorEl] = React.useState<HTMLParagraphElement | null>(null)

    const isItemElectionEventType = resourceType === "sequent_backend_election_event"

    function handleOpenItemActions(): void {
        setAnchorEl(menuItemRef.current)
    }

    async function handleAction(action: Action, payload: ActionPayload) {
        // close the popover
        setAnchorEl(null)

        if (action === Action.Add) {
            navigate(getNavLinkCreate(parentData, payload.type))
        } else if (payload.type === "sequent_backend_election_event") {
            if (action === Action.Archive) {
                console.log("1")
                await update(payload.type, {
                    id: payload.id,
                    data: {is_archived: true},
                    previousData: {is_archived: false},
                })
                console.log("2")

                refetch()

                console.log("3")
            } else if (action === Action.Unarchive) {
                console.log("1")
                await update(payload.type, {
                    id: payload.id,
                    data: {is_archived: false},
                    previousData: {is_archived: true},
                })
                console.log("2")

                refetch()

                console.log("3")
            }
        } else if (action === Action.Remove) {
            setDeleteItem(payload)
            setOpenDeleteModal(true)
        }
    }

    const handleCloseActionMenu = () => {
        setAnchorEl(null)
    }

    const onSuccess = async () => {
        refetch()
        setDeleteItem(undefined)
        notify(`${deleteItem.type} removed.`, {type: "success"})
    }

    const onError = async () => {
        setDeleteItem(undefined)
        notify(`Error removing ${deleteItem.type}`, {type: "error"})
    }

    const confirmDeleteAction = () => {
        deleteOne(deleteItem.type, {id: deleteItem.id}, {onSuccess, onError})
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
                    {!isArchivedTab && (
                        <MenuItem
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

                    {isItemElectionEventType && !isArchivedTab && <Divider />}

                    {isItemElectionEventType && (
                        <MenuItem
                            onClick={() =>
                                handleAction(isArchivedTab ? Action.Unarchive : Action.Archive, {
                                    id: resourceId,
                                    name: resourceName,
                                    type: resourceType,
                                })
                            }
                        >
                            <ListItemIcon>
                                <InventoryIcon className="text-brand-color" />
                            </ListItemIcon>
                            {isArchivedTab
                                ? t("sideMenu.menuActions.unarchive.electionEvent")
                                : t("sideMenu.menuActions.archive.electionEvent")}
                        </MenuItem>
                    )}

                    {!isArchivedTab && <Divider />}

                    {!isArchivedTab && (
                        <MenuItem
                            onClick={() =>
                                handleAction(Action.Remove, {
                                    id: resourceId,
                                    name: resourceName,
                                    type: resourceType,
                                })
                            }
                        >
                            <ListItemIcon>
                                <DeleteIcon className="text-brand-color" />
                            </ListItemIcon>

                            {t(mapRemoveResource[resourceType])}
                        </MenuItem>
                    )}
                </MenuList>
            </Popover>

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
                {t("common.message.delete")}
            </Dialog>
        </>
    )
}
