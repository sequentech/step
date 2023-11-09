// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box, Typography} from "@mui/material"
import React, {useState, useEffect} from "react"
import {ResourceOptions, ResourceDefinition, useResourceDefinitions, useGetList} from "react-admin"
import {CircularProgress} from "@mui/material"
import {useTenantStore} from "./CustomMenu"
import {faAngleRight, faAngleDown} from "@fortawesome/free-solid-svg-icons"
import {Icon} from "@sequentech/ui-essentials"
import {styled} from "@mui/material/styles"

const Horizontal = styled(Box)`
    display: flex;
    flex-direction: row;
    align-items: center;
    cursor: pointer;
`

const LeavesWrapper = styled(Box)`
    display: flex;
    flex-direction: column;
    margin-left: 24px;
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
    resourceId?: string
    treeResources: Array<ResourceDefinition<Options>>
}

const TreeLeaves: React.FC<TreeLeavesProps> = ({resourceId, treeResources}) => {
    const [tenantId] = useTenantStore()

    const {data, total, isLoading, error} = useGetList(treeResources[0]?.name || "", {
        pagination: {page: 1, perPage: 10},
        sort: {field: "created_at", order: "DESC"},
        filter: resourceId
            ? {
                  tenant_id: tenantId,
                  [treeResources[0]?.options?.foreignKeyFrom || ""]: resourceId,
              }
            : {
                  tenant_id: tenantId,
              },
    })

    if (isLoading || error || !data) {
        return null
    }

    return (
        <LeavesWrapper>
            {data?.map((resource, idx) => (
                <TreeMenuItem
                    resource={resource}
                    treeResources={treeResources.slice(1)}
                    key={idx}
                />
            ))}
        </LeavesWrapper>
    )
}

interface TreeMenuItemProps {
    resource: any
    treeResources: Array<ResourceDefinition<Options>>
}

const TreeMenuItem: React.FC<TreeMenuItemProps> = ({resource, treeResources}) => {
    const [open, setOpen] = useState(false)
    const onClick = () => setOpen(!open)
    const hasLeaves = treeResources.length > 0

    return (
        <Box>
            <Horizontal>
                {hasLeaves ? (
                    <StyledIcon icon={open ? faAngleDown : faAngleRight} onClick={onClick}/>
                ) : null}
                <Typography fontSize="16px" margin={0} paddingLeft={hasLeaves ? 0: "24px"}>
                    {resource.name || resource.id}
                </Typography>
            </Horizontal>
            {open ? <TreeLeaves resourceId={resource.id} treeResources={treeResources} /> : null}
        </Box>
    )
}

export const TreeMenu: React.FC = () => {
    let allResources = useResourceDefinitions()
    let [treeResources, setTreeResources] = useState<Array<ResourceDefinition<Options>>>([])

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

    if (0 === treeResources.length) {
        return null
    }

    return (
        <Box>
            <TreeLeaves treeResources={treeResources} />
        </Box>
    )
}
