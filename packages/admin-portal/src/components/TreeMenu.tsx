// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box} from "@mui/material"
import React, {useState, useEffect} from "react"
import {
    ResourceOptions,
    ResourceDefinition,
    useResourceDefinitions,
    useGetList,
    MenuItemLink,
} from "react-admin"
import {CircularProgress} from "@mui/material"
import {useTenantStore} from "./CustomMenu"
import {faAngleRight, faAngleDown} from "@fortawesome/free-solid-svg-icons"
import {Icon} from "@sequentech/ui-essentials"
import {styled} from "@mui/material/styles"
import Tabs from '@mui/material/Tabs'
import Tab from '@mui/material/Tab'

const Horizontal = styled(Box)`
    display: flex;
    flex-direction: row;
    align-items: center;
    cursor: pointer;
`

const LeavesWrapper = styled(Box)`
    display: flex;
    flex-direction: column;
    margin-left: 12px;
`

const StyledIcon = styled(Icon)`
    width: 24px;
    margin-right: 8px;
`

interface Options extends ResourceOptions {
    isMenuParent?: boolean
    menuParent?: string
    foreignKeyFrom?: string
}

interface TreeLeavesProps {
    isOpen: boolean
    resourceId?: string
    treeResources: Array<ResourceDefinition<Options>>
    filter?: object
}

const TreeLeaves: React.FC<TreeLeavesProps> = ({isOpen, resourceId, treeResources, filter}) => {
    const [tenantId] = useTenantStore()

    const {data, total, isLoading, error} = useGetList(treeResources[0]?.name || "", {
        pagination: {page: 1, perPage: 10},
        sort: {field: "created_at", order: "DESC"},
        filter: resourceId
            ? {
                  tenant_id: tenantId,
                  [treeResources[0]?.options?.foreignKeyFrom || ""]: resourceId,
                  ...filter
              }
            : {
                  tenant_id: tenantId,
                  ...filter
              },
    })

    if (isLoading) {
        return <CircularProgress />
    }

    if (error || !data) {
        return null
    }

    return (
        <LeavesWrapper>
            {data?.map((resource, idx) => (
                <TreeMenuItem
                    isOpen={isOpen}
                    resourceType={treeResources[0].name}
                    resource={resource}
                    treeResources={treeResources.slice(1)}
                    key={idx}
                />
            ))}
        </LeavesWrapper>
    )
}

interface TreeMenuItemProps {
    isOpen: boolean
    resourceType: string
    resource: any
    treeResources: Array<ResourceDefinition<Options>>
}

const TreeMenuItem: React.FC<TreeMenuItemProps> = ({
    isOpen,
    resourceType,
    resource,
    treeResources,
}) => {
    const [open, setOpen] = useState(false)
    const onClick = () => setOpen(!open)
    const hasLeaves = treeResources.length > 0

    return (
        <Box>
            <Horizontal>
                {hasLeaves ? (
                    <StyledIcon icon={open ? faAngleDown : faAngleRight} onClick={onClick} />
                ) : null}

                {isOpen ? (
                    <MenuItemLink
                        key={resource.name}
                        to={`/${resourceType}/${resource.id}`}
                        primaryText={resource.name}
                        sx={{paddingLeft: 0}}
                    />
                ) : null}
            </Horizontal>
            {open ? (
                <TreeLeaves
                    isOpen={isOpen}
                    resourceId={resource.id}
                    treeResources={treeResources}
                />
            ) : null}
        </Box>
    )
}

interface TreeMenuProps {
    isOpen: boolean
}
export const TreeMenu: React.FC<TreeMenuProps> = ({isOpen}) => {
    let allResources = useResourceDefinitions()
    let [treeResources, setTreeResources] = useState<Array<ResourceDefinition<Options>>>([])
    const [archivedMenu, setArchivedMenu] = useState(0)

    useEffect(() => {
        const resources: Array<ResourceDefinition<Options>> = Object.keys(allResources).map(
            (name) => allResources[name]
        )
        let parent = resources.find((resource) => resource.options?.isMenuParent)

        if (!parent) {
            return
        }

        let tree: Array<ResourceDefinition<Options>> = [parent]
        let finished = false
        while (!finished) {
            let lastLeave = tree[tree.length - 1]
            let leave = resources.find(
                (resource) => resource.options?.menuParent === lastLeave.name
            )
            if (!leave) {
                finished = true
            } else {
                tree.push(leave)
            }
        }
        setTreeResources(tree)
    }, [allResources])

    const handleChange = (event: React.SyntheticEvent, newValue: number) => {
        console.log(`new value ${newValue}`)
        setArchivedMenu(newValue);
      }

    if (0 === treeResources.length) {
        return null
    }

    return (<>
        <Tabs value={archivedMenu} onChange={handleChange} sx={{marginBottom: "4px"}}>
            <Tab label="Active" />
            <Tab label="Archived" />
        </Tabs>
        <Box sx={{marginLeft: "20px"}}>
            <TreeLeaves
                treeResources={treeResources}
                isOpen={isOpen}
                filter={{is_archived: 1 === archivedMenu}}
            />
        </Box>
    </>)
}
