// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box} from "@mui/material"
import React, {useState, useEffect} from "react"
import {ResourceOptions, ResourceDefinition, useResourceDefinitions, useGetList} from "react-admin"
import {CircularProgress} from "@mui/material"
import {useTenantStore} from "./CustomMenu"

interface Options extends ResourceOptions {
    isMenuParent?: boolean
    menuParent?: string
    foreignKeyFrom?: string
}

interface TreeMenuItemProps {
    resource: any
    treeResources: Array<ResourceDefinition<Options>>
}

const TreeMenuItem: React.FC<TreeMenuItemProps> = ({resource, treeResources}) => {
    const [tenantId] = useTenantStore()

    const {data, total, isLoading, error} = useGetList(treeResources[0]?.name || "", {
        pagination: {page: 1, perPage: 10},
        sort: {field: "created_at", order: "DESC"},
        filter: {
            tenant_id: tenantId,
            [treeResources[0]?.options?.foreignKeyFrom || ""]: resource.id || "",
        },
    })

    if (isLoading || error || !data) {
        return <Box>{resource.name || resource.id}</Box>
    }

    return (
        <Box>
            {resource.name || resource.id}
            {data?.map((resource, idx) => (
                <TreeMenuItem
                    resource={resource}
                    treeResources={treeResources.slice(1)}
                    key={idx}
                />
            ))}
        </Box>
    )
}

export const TreeMenu: React.FC = () => {
    let allResources = useResourceDefinitions()
    let [treeResources, setTreeResources] = useState<Array<ResourceDefinition<Options>>>([])
    const [tenantId] = useTenantStore()

    const {data, total, isLoading, error} = useGetList(treeResources[0]?.name || "", {
        pagination: {page: 1, perPage: 10},
        sort: {field: "created_at", order: "DESC"},
        filter: {
            tenant_id: tenantId,
        },
    })

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

    if (isLoading) {
        return <CircularProgress />
    }
    if (error) {
        return <p>ERROR</p>
    }

    return (
        <div>
            {data?.map((resource, idx) => (
                <TreeMenuItem
                    resource={resource}
                    treeResources={treeResources.slice(1)}
                    key={idx}
                />
            ))}
        </div>
    )
}
