// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useRef, useState} from "react"
import {useNavigate} from "react-router-dom"
import {useDataProvider, useNotify, useUpdate} from "react-admin"
import {useTranslation} from "react-i18next"
import ChevronRightIcon from "@mui/icons-material/ChevronRight"
import MoreHorizIcon from "@mui/icons-material/MoreHoriz"
import DeleteIcon from "@mui/icons-material/Delete"
import {ListItemIcon, MenuItem, MenuList, Popover} from "@mui/material"
import {Dialog} from "@sequentech/ui-essentials"
import {IElectionPresentation, ISlate, getSlateName} from "@sequentech/ui-core"
import {MenuStyles, TreeMenuItemContainer} from "@/components/styles/Menu"
import {Sequent_Backend_Election} from "@/gql/graphql"
import {readSlatesConfig, removeSlate, writeSlatesConfig} from "@/utils/slates"
import {createSlatePath, slatePath} from "@/resources/Slate/slatePaths"
import {ElectionType} from "../ElectionEvents"
import {useActionPermissions} from "../use-tree-menu-hook"

interface SlateTreeItemProps {
    election: ElectionType
    slate: ISlate
    name: string
    canRemove: boolean
}

const SlateTreeItem: React.FC<SlateTreeItemProps> = ({election, slate, name, canRemove}) => {
    const {t, i18n} = useTranslation()
    const navigate = useNavigate()
    const notify = useNotify()
    const dataProvider = useDataProvider()
    const [update] = useUpdate()
    const menuItemRef = useRef<HTMLDivElement | null>(null)
    const [anchorEl, setAnchorEl] = useState<HTMLDivElement | null>(null)
    const [isConfirmOpen, setIsConfirmOpen] = useState(false)
    const isRtl = i18n.dir(i18n.language) === "rtl"
    const to = slatePath(election.id, slate.id)

    const remove = async () => {
        try {
            const {data: current} = await dataProvider.getOne<Sequent_Backend_Election>(
                "sequent_backend_election",
                {id: election.id}
            )
            const config = readSlatesConfig(current.annotations)
            if (!config) {
                throw new Error(t("slateScreen.messages.unreadable"))
            }
            await update(
                "sequent_backend_election",
                {
                    id: election.id,
                    data: {
                        annotations: writeSlatesConfig(
                            current.annotations,
                            removeSlate(config, slate.id)
                        ),
                    },
                    previousData: current,
                },
                {mutationMode: "pessimistic", returnPromise: true}
            )
            notify(t("sideMenu.menuActions.messages.notification.success.delete"), {
                type: "success",
            })
            navigate(`/sequent_backend_election/${election.id}`)
        } catch (error) {
            notify(t("sideMenu.menuActions.messages.notification.error.delete"), {type: "error"})
        }
    }

    return (
        <>
            <TreeMenuItemContainer ref={menuItemRef} isClicked={!!anchorEl}>
                <MenuStyles.TreeMenuIconContaier isActive onClick={() => navigate(to)}>
                    <ChevronRightIcon
                        className="menu-item-collapsed"
                        style={{transform: isRtl ? "rotate(180deg)" : "rotate(0)"}}
                    />
                </MenuStyles.TreeMenuIconContaier>
                <MenuStyles.StyledSideBarNavLink
                    title={name}
                    end
                    className={({isActive}) => (isActive ? "active menu-item-slate" : "")}
                    to={to}
                    style={{textAlign: isRtl ? "end" : "start"}}
                >
                    <MenuStyles.ItemContainer>
                        <MenuStyles.SpanContainer>{name}</MenuStyles.SpanContainer>
                    </MenuStyles.ItemContainer>
                </MenuStyles.StyledSideBarNavLink>
                <MenuStyles.MenuActionContainer className="menu-actions-slate">
                    {canRemove && (
                        <MenuStyles.StyledIconContainer
                            onClick={() => setAnchorEl(menuItemRef.current)}
                        >
                            <MoreHorizIcon id="MoreHorizIcon" />
                        </MenuStyles.StyledIconContainer>
                    )}
                </MenuStyles.MenuActionContainer>
            </TreeMenuItemContainer>
            <Popover
                open={!!anchorEl}
                anchorEl={anchorEl}
                onClose={() => setAnchorEl(null)}
                anchorOrigin={{vertical: "bottom", horizontal: "right"}}
            >
                <MenuList dense>
                    <MenuItem
                        dir={i18n.dir(i18n.language)}
                        className="menu-action-delete-slate"
                        onClick={() => {
                            setAnchorEl(null)
                            setIsConfirmOpen(true)
                        }}
                    >
                        <ListItemIcon>
                            <DeleteIcon color="error" />
                        </ListItemIcon>
                        {t("sideMenu.menuActions.remove.slate")}
                    </MenuItem>
                </MenuList>
            </Popover>
            <Dialog
                variant="warning"
                open={isConfirmOpen}
                ok={String(t("common.label.delete"))}
                cancel={String(t("common.label.cancel"))}
                title={String(t("common.label.warning"))}
                handleClose={async (result: boolean) => {
                    setIsConfirmOpen(false)
                    if (result) {
                        await remove()
                    }
                }}
            >
                {t("sideMenu.menuActions.messages.confirm.delete")}
            </Dialog>
        </>
    )
}

interface SlateTreeLeavesProps {
    election: ElectionType
    isArchivedElectionEvents: boolean
}

/** The slates of an election in the tree, before its contests. */
export const SlateTreeLeaves: React.FC<SlateTreeLeavesProps> = ({
    election,
    isArchivedElectionEvents,
}) => {
    const {t, i18n} = useTranslation()
    const {canReadElection, canWriteElection} = useActionPermissions()
    const config = readSlatesConfig(election.annotations)
    const isRtl = i18n.dir(i18n.language) === "rtl"

    if (!config || !canReadElection) {
        return null
    }

    const presentation = election.presentation as IElectionPresentation | null | undefined
    const defaultLanguage = presentation?.language_conf?.default_language_code ?? undefined
    const canChange = canWriteElection && !isArchivedElectionEvents

    return (
        <MenuStyles.TreeLeavesContainer>
            {config.slates.map((slate) => (
                <SlateTreeItem
                    key={slate.id}
                    election={election}
                    slate={slate}
                    name={getSlateName(slate, i18n.language, defaultLanguage)}
                    canRemove={canChange}
                />
            ))}
            {canChange && (
                <MenuStyles.CreateElectionContainer
                    style={{justifyContent: isRtl ? "end" : "start"}}
                >
                    <MenuStyles.StyledAddIcon style={{display: isRtl ? "none" : "block"}} />
                    <MenuStyles.StyledNavLink
                        className="sequent_backend_slate"
                        to={createSlatePath(election.id)}
                        style={{textAlign: isRtl ? "end" : "start"}}
                    >
                        {t("createResource.slate")}
                    </MenuStyles.StyledNavLink>
                    <MenuStyles.StyledAddIcon style={{display: isRtl ? "block" : "none"}} />
                    <MenuStyles.StyledHiddenDiv />
                </MenuStyles.CreateElectionContainer>
            )}
        </MenuStyles.TreeLeavesContainer>
    )
}
