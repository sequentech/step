// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::types::hasura::core::{Area, AreaContest, Contest};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet, VecDeque};

// A tree node that corresponds to an area
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct TreeNodeArea {
    pub id: String, // area id
    pub tenant_id: String,
    pub annotations: Option<Value>,
    pub election_event_id: String,
    pub parent_id: Option<String>,
}

// Extra data for an area. We'll use that to create a tree
// where all nodes have in "contest_ids" both their directly assigned
// contests and the contests inherited from their ancestors.
#[derive(PartialEq, Eq, Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContestsData {
    contest_ids: HashSet<String>,
}

impl From<&Area> for TreeNodeArea {
    fn from(area: &Area) -> Self {
        TreeNodeArea {
            id: area.id.clone(),
            tenant_id: area.tenant_id.clone(),
            annotations: area.annotations.clone(),
            election_event_id: area.election_event_id.clone(),
            parent_id: area.parent_id.clone(),
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct TreeNode<T = ()> {
    pub area: Option<TreeNodeArea>,
    pub children: Vec<TreeNode<T>>,
    pub data: T,
}

impl<T> TreeNode<T>
where
    T: Clone + Default,
{
    // returns all nodes in the tree
    pub fn get_all_children(&self) -> Vec<TreeNodeArea> {
        let mut children: Vec<TreeNodeArea> = vec![];
        if let Some(area) = self.area.clone() {
            children.push(area);
        };
        let sub_children: Vec<TreeNodeArea> = self
            .children
            .iter()
            .map(|child| child.get_all_children())
            .flatten()
            .collect();
        children.extend(sub_children);
        children
    }

    // creates a tree from the list of nodes
    pub fn from_areas(areas: Vec<TreeNodeArea>) -> Result<TreeNode<T>> {
        let mut nodes: HashMap<String, TreeNode<T>> = HashMap::new();
        let mut parent_map: HashMap<String, Vec<String>> = HashMap::new();
        let mut root_ids: Vec<String> = Vec::new();

        // Initialize TreeNodes and parent map
        for area in areas.into_iter() {
            let id = area.id.clone();
            let parent_id = area.parent_id.clone();

            nodes.insert(
                id.clone(),
                TreeNode::<T> {
                    area: Some(area),
                    children: Vec::new(),
                    data: Default::default(), // this should change?
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

        let mut root_node = TreeNode::<T> {
            area: None,
            children: Vec::new(),
            data: Default::default(),
        };

        // Build the forest under a single root
        // as build_tree is recursive, we defined the visited var outside to
        // maintain state outside the multiple recursive calls
        let mut visited: HashSet<String> = HashSet::new();
        for root_id in root_ids {
            let child_node = TreeNode::<T>::build_tree(
                &root_id,
                &nodes,
                &mut visited,
                &parent_map,
            )?;
            root_node.children.push(child_node);
        }

        Ok(root_node)
    }

    // internal function used by from_areas
    fn build_tree<'a>(
        id: &'a str,
        nodes: &'a HashMap<String, TreeNode<T>>,
        visited: &mut HashSet<String>,
        parent_map: &'a HashMap<String, Vec<String>>,
    ) -> Result<TreeNode<T>> {
        if visited.contains(id) {
            return Err(anyhow!("Loop detected in the tree structure"));
        }

        visited.insert(id.to_string());
        let node = nodes.get(id).ok_or(anyhow!("Node not found"))?.clone();
        let mut new_node = TreeNode::<T> {
            area: node.area.clone(),
            children: Vec::new(),
            data: Default::default(), // this should change?
        };

        if let Some(children_ids) = parent_map.get(id) {
            for child_id in children_ids {
                let child_node = TreeNode::<T>::build_tree(
                    child_id, nodes, visited, parent_map,
                )?;
                new_node.children.push(child_node);
            }
        }

        visited.remove(id);
        Ok(new_node)
    }

    // find an area in the tree
    pub fn find_area(&self, area_id: &str) -> Option<TreeNode<T>> {
        if let Some(area) = self.area.clone() {
            if &area.id == area_id {
                return Some(self.clone());
            }
        }
        for leave in self.children.iter() {
            if let Some(area) = leave.find_area(area_id) {
                return Some(area);
            }
        }
        None
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
        node: &TreeNode<T>,
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

    // generate a tree where each node has the data of all related contests
    // note that areas spread down the tree
    pub fn get_contests_data_tree(
        &self,
        area_contests: &Vec<AreaContest>,
    ) -> TreeNode<ContestsData> {
        // Map<area_id, Set<contest_id>>
        let mut areas_map: HashMap<String, HashSet<String>> = HashMap::new();
        for area_contest in area_contests.iter() {
            areas_map
                .entry(area_contest.area_id.clone())
                .and_modify(|contest_ids| {
                    contest_ids.insert(area_contest.contest_id.clone());
                })
                .or_insert_with(|| {
                    let mut set = HashSet::new();
                    set.insert(area_contest.contest_id.clone());
                    set
                });
        }
        let root_data: ContestsData = Default::default();
        self.contests_data_tree(&root_data, &areas_map)
    }

    fn contests_data_tree(
        &self,
        parent_data: &ContestsData,
        // Map<area_id, Set<contest_id>>
        areas_map: &HashMap<String, HashSet<String>>,
    ) -> TreeNode<ContestsData> {
        // get contest ids inherited from parent
        let mut contest_ids = parent_data.contest_ids.clone();
        // get contest ids from self area
        if let Some(area) = self.area.clone() {
            if let Some(self_contests) = areas_map.get(&area.id) {
                contest_ids.extend(self_contests.clone());
            }
        }
        let data = ContestsData { contest_ids };
        let children: Vec<TreeNode<ContestsData>> = self
            .children
            .iter()
            .map(|child| {
                child.contests_data_tree(
                    &data, // Map<area_id, Set<contest_id>>
                    &areas_map,
                )
            })
            .collect();
        TreeNode::<ContestsData> {
            area: self.area.clone(),
            children: children,
            data: data,
        }
    }
}

impl TreeNode<ContestsData> {
    // For a given TreeNode of type ContestsData, return all
    // area-contests. Note that this will include
    // indirect/inherited ones.
    pub fn get_contest_matches(
        &self,
        contest_ids: &HashSet<String>,
    ) -> HashSet<AreaContest> {
        let mut set = HashSet::new();
        if let Some(area) = self.area.clone() {
            let own_area_contests: HashSet<AreaContest> = self
                .data
                .contest_ids
                .iter()
                .map(|contest_id| AreaContest {
                    id: area.id.clone(),
                    area_id: area.id.clone(),
                    contest_id: contest_id.clone(),
                })
                .collect();
            set.extend(own_area_contests);
        }
        for child in self.children.iter() {
            let child_set = child.get_contest_matches(contest_ids);
            set.extend(child_set);
        }
        set
    }
}

pub struct TreeNodeIter<'a, T> {
    queue: VecDeque<&'a TreeNode<T>>,
}

impl<'a, T> Iterator for TreeNodeIter<'a, T> {
    type Item = &'a TreeNode<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(node) = self.queue.pop_front() {
            for child in &node.children {
                self.queue.push_back(child);
            }
            Some(node)
        } else {
            None
        }
    }
}

impl<T> TreeNode<T> {
    pub fn iter(&self) -> TreeNodeIter<T> {
        let mut queue = VecDeque::new();
        queue.push_back(self);
        TreeNodeIter { queue }
    }
}

#[cfg(test)]
mod tests {
    use crate::services::area_tree::*;

    fn get_fixture1() -> Vec<TreeNodeArea> {
        vec![
            TreeNodeArea {
                id: "grandad".into(), // area id
                tenant_id: "tenant".into(),
                election_event_id: "election".into(),
                annotations: None,
                parent_id: None,
            },
            TreeNodeArea {
                id: "father1".into(), // area id
                tenant_id: "tenant".into(),
                election_event_id: "election".into(),
                annotations: None,
                parent_id: Some("grandad".into()),
            },
            TreeNodeArea {
                id: "father2".into(), // area id
                tenant_id: "tenant".into(),
                election_event_id: "election".into(),
                annotations: None,
                parent_id: Some("grandad".into()),
            },
            TreeNodeArea {
                id: "child1".into(), // area id
                tenant_id: "tenant".into(),
                election_event_id: "election".into(),
                annotations: None,
                parent_id: Some("father1".into()),
            },
            TreeNodeArea {
                id: "child2".into(), // area id
                tenant_id: "tenant".into(),
                election_event_id: "election".into(),
                annotations: None,
                parent_id: Some("father1".into()),
            },
            TreeNodeArea {
                id: "child3".into(), // area id
                tenant_id: "tenant".into(),
                election_event_id: "election".into(),
                annotations: None,
                parent_id: Some("father2".into()),
            },
        ]
    }

    #[test]
    fn test_find_path() {
        assert_eq!(1, 1);
        let node_areas = get_fixture1();
        let tree = TreeNode::<()>::from_areas(node_areas).unwrap();
        let path = tree.find_path_to_area("child2").unwrap();
        let str_path: Vec<String> =
            path.into_iter().map(|val| val.id.clone()).collect();
        let expected_path: Vec<String> =
            vec!["grandad".into(), "father1".into(), "child2".into()];
        assert_eq!(str_path, expected_path);

        let path = tree.find_path_to_area("child3").unwrap();
        let str_path: Vec<String> =
            path.into_iter().map(|val| val.id.clone()).collect();
        let expected_path: Vec<String> =
            vec!["grandad".into(), "father2".into(), "child3".into()];
        assert_eq!(str_path, expected_path);
    }
}
