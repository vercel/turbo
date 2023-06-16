use std::collections::HashMap;

use anyhow::Result;
use petgraph::graph::{Graph, NodeIndex};
use turbopath::AbsoluteSystemPath;

use super::{Dependency, PackageGraph, WorkspaceName, WorkspaceNode};
use crate::{package_json::PackageJson, package_manager::PackageManager};

pub struct PackageGraphBuilder<'a> {
    repo_root: &'a AbsoluteSystemPath,
    root_package_json: PackageJson,
    single: bool,
    package_manager: Option<PackageManager>,
}

impl<'a> PackageGraphBuilder<'a> {
    pub fn new(repo_root: &'a AbsoluteSystemPath, root_package_json: PackageJson) -> Self {
        Self {
            repo_root,
            root_package_json,
            single: false,
            package_manager: None,
        }
    }

    pub fn with_single_package_mode(mut self, is_single: bool) -> Self {
        self.single = is_single;
        self
    }

    pub fn with_package_manger(mut self, package_manager: Option<PackageManager>) -> Self {
        self.package_manager = package_manager;
        self
    }

    pub fn build(self) -> Result<PackageGraph> {
        match self.single {
            true => self.build_single_package_graph(),
            false => self.build_multi_package_graph(),
        }
    }

    fn build_single_package_graph(mut self) -> Result<PackageGraph> {
        let package_manager = self.package_manager()?;
        let Self {
            root_package_json, ..
        } = self;
        let mut package_jsons = HashMap::with_capacity(1);
        package_jsons.insert(WorkspaceName::Root, root_package_json);
        let mut workspace_graph = petgraph::Graph::new();
        let mut workspaces = HashMap::new();
        let root_index = Self::add_node(&mut workspace_graph, &mut workspaces, WorkspaceNode::Root);
        let root_workspace = Self::add_node(
            &mut workspace_graph,
            &mut workspaces,
            WorkspaceNode::Workspace(WorkspaceName::Root),
        );
        workspace_graph.add_edge(root_workspace, root_index, Dependency::Root);

        Ok(PackageGraph {
            workspace_graph,
            workspaces,
            package_jsons,
            package_manager,
            lockfile: None,
        })
    }

    fn build_multi_package_graph(self) -> Result<PackageGraph> {
        Ok(PackageGraph {
            workspace_graph: petgraph::Graph::new(),
            workspaces: HashMap::new(),
            package_jsons: HashMap::new(),
            package_manager: PackageManager::Npm,
            lockfile: Some(Box::<turborepo_lockfiles::NpmLockfile>::default()),
        })
    }

    fn package_manager(&mut self) -> Result<PackageManager, crate::package_manager::Error> {
        self.package_manager.take().map_or_else(
            || PackageManager::get_package_manager(self.repo_root, Some(&self.root_package_json)),
            Result::Ok,
        )
    }

    fn add_node<E>(
        graph: &mut Graph<WorkspaceNode, E>,
        workspaces: &mut HashMap<WorkspaceNode, NodeIndex>,
        node: WorkspaceNode,
    ) -> petgraph::graph::NodeIndex {
        let idx = graph.add_node(node.clone());
        workspaces.insert(node, idx);
        idx
    }
}
