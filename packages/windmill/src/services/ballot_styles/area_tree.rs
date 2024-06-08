// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Result};
use sequent_core::types::hasura::core::Area;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct TreeNode {
    pub area: Area,
    pub children: Vec<TreeNode>,
}

impl TreeNode {
    pub fn from_areas(areas: Vec<Area>) -> Result<TreeNode> {
        let mut nodes: HashMap<String, TreeNode> = HashMap::new();
        // Map<parent_id, Vec<children ids>>
        let mut parent_map: HashMap<String, Vec<String>> = HashMap::new();
        let mut root_id: Option<String> = None;

        // Initialize TreeNodes and parent map
        for area in areas.into_iter() {
            let id = area.id.clone();
            let parent_id = area.parent_id.clone();

            nodes.insert(
                id.clone(),
                TreeNode {
                    area,
                    children: Vec::new(),
                },
            );

            if let Some(parent_id) = parent_id {
                parent_map.entry(parent_id).or_default().push(id);
            } else {
                if root_id.is_some() {
                    return Err(anyhow!("Multiple roots detected"));
                }
                root_id = Some(id);
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

        let root_id = root_id.ok_or(anyhow!("No root found"))?;
        // as build_tree is recursive, we defined the visited var outside to
        // maintain state outside the multiple recursive calls
        let mut visited: HashSet<String> = HashSet::new();
        TreeNode::build_tree(&root_id, &nodes, &mut visited, &parent_map)
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
                let child_node = TreeNode::build_tree(child_id, nodes, visited, parent_map)?;
                new_node.children.push(child_node);
            }
        }

        visited.remove(id);
        Ok(new_node)
    }

    pub fn find_path_to_area(&self, area_id: &str) -> Option<Vec<Area>> {
        // Initialize the path and call the helper function
        let mut path: Vec<Area> = Vec::new();
        if Self::dfs(self, area_id, &mut path) {
            Some(path)
        } else {
            None
        }
    }

    // Depth First Helper function to recursively find the path
    fn dfs(node: &TreeNode, area_id: &str, path: &mut Vec<Area>) -> bool {
        // Add current node to the path
        path.push(node.area.clone());

        // Check if the current node is the target node
        if node.area.id == area_id {
            return true;
        }

        // Recur for each child
        for child in &node.children {
            if Self::dfs(child, area_id, path) {
                return true;
            }
        }

        // If not found, backtrack
        path.pop();
        false
    }
}
