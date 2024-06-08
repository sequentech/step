// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Result};
use std::collections::{HashMap, HashSet};

use crate::types::hasura::core::Area;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct TreeNodeArea {
    pub id: String,
    pub tenant_id: String,
    pub election_event_id: String,
    pub parent_id: Option<String>,
}

impl From<&Area> for TreeNodeArea {
    fn from(area: &Area) -> Self {
        TreeNodeArea {
            id: area.id.clone(),
            tenant_id: area.tenant_id.clone(),
            election_event_id: area.election_event_id.clone(),
            parent_id: area.parent_id.clone(),
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct TreeNode {
    pub area: Option<TreeNodeArea>,
    pub children: Vec<TreeNode>,
}

impl TreeNode {
    pub fn from_areas(areas: Vec<TreeNodeArea>) -> Result<TreeNode> {
        let mut nodes: HashMap<String, TreeNode> = HashMap::new();
        let mut parent_map: HashMap<String, Vec<String>> = HashMap::new();
        let mut root_ids: Vec<String> = Vec::new();

        // Initialize TreeNodes and parent map
        for area in areas.into_iter() {
            let id = area.id.clone();
            let parent_id = area.parent_id.clone();

            nodes.insert(
                id.clone(),
                TreeNode {
                    area: Some(area),
                    children: Vec::new(),
                },
            );

            if let Some(parent_id) = parent_id {
                parent_map.entry(parent_id).or_default().push(id);
            } else {
                root_ids.push(id);
            }
        }

        // Ensure all parent_ids are valid
        for (parent_id, _) in &parent_map {
            if !nodes.contains_key(parent_id) {
                return Err(anyhow!(
                    "Parent id {} not found in the tree structure",
                    parent_id
                ));
            }
        }

        let mut root_node = TreeNode {
            area: None,
            children: Vec::new(),
        };

        // Build the forest under a single root
        // as build_tree is recursive, we defined the visited var outside to
        // maintain state outside the multiple recursive calls
        let mut visited: HashSet<String> = HashSet::new();
        for root_id in root_ids {
            let child_node = TreeNode::build_tree(
                &root_id,
                &nodes,
                &mut visited,
                &parent_map,
            )?;
            root_node.children.push(child_node);
        }

        Ok(root_node)
    }

    fn build_tree<'a>(
        id: &'a str,
        nodes: &'a HashMap<String, TreeNode>,
        visited: &mut HashSet<String>,
        parent_map: &'a HashMap<String, Vec<String>>,
    ) -> Result<TreeNode> {
        if visited.contains(id) {
            return Err(anyhow!("Loop detected in the tree structure"));
        }

        visited.insert(id.to_string());
        let node = nodes.get(id).ok_or(anyhow!("Node not found"))?.clone();
        let mut new_node = TreeNode {
            area: node.area.clone(),
            children: Vec::new(),
        };

        if let Some(children_ids) = parent_map.get(id) {
            for child_id in children_ids {
                let child_node =
                    TreeNode::build_tree(child_id, nodes, visited, parent_map)?;
                new_node.children.push(child_node);
            }
        }

        visited.remove(id);
        Ok(new_node)
    }

    pub fn find_path_to_area(
        &self,
        area_id: &str,
    ) -> Option<Vec<TreeNodeArea>> {
        // Initialize the path and call the helper function
        let mut path: Vec<TreeNodeArea> = Vec::new();
        if Self::dfs(self, area_id, &mut path) {
            Some(path)
        } else {
            None
        }
    }

    // Depth First Helper function to recursively find the path
    fn dfs(
        node: &TreeNode,
        area_id: &str,
        path: &mut Vec<TreeNodeArea>,
    ) -> bool {
        // Add current node to the path if it has an area
        if let Some(area) = &node.area {
            path.push(area.clone());
        }

        // Check if the current node is the target node
        if node.area.as_ref().map_or(false, |area| area.id == area_id) {
            return true;
        }

        // Recur for each child
        for child in &node.children {
            if Self::dfs(child, area_id, path) {
                return true;
            }
        }

        // If not found, backtrack
        if node.area.is_some() {
            path.pop();
        }
        false
    }
}
