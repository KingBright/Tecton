//! Stable author IDs for joining semantics to a native automatic layout.
use std::collections::HashMap;

use tecton_layout::layout_context::{LayoutNode, LayoutResult};
use tecton_routing::route::Route;
use tecton_types::{
    error::{Result, TectonError},
    geometry::Rect,
    id::{EdgeId, NodeId, PortId},
};

use crate::{DiagramDsl, DslParser, Orchestrator, OrchestratorConfig};

/// Declared relation identities. These are not physical port positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundEdge {
    pub edge_id: EdgeId,
    pub source_port: PortId,
    pub target_port: PortId,
}

/// Author names map to runtime IDs for exactly one computed graph.
#[derive(Debug)]
pub struct DiagramBindings {
    pub(crate) nodes: HashMap<String, NodeId>,
    pub(crate) ports: HashMap<String, PortId>,
    pub(crate) edges: HashMap<String, BoundEdge>,
}

impl DiagramBindings {
    pub fn node_id(&self, name: &str) -> Option<NodeId> {
        self.nodes.get(name).copied()
    }

    pub fn port_id(&self, reference: &str) -> Option<PortId> {
        self.ports.get(reference).copied()
    }

    pub fn edge(&self, name: &str) -> Option<BoundEdge> {
        self.edges.get(name).copied()
    }
}

/// A native layout/routing result with every explicitly named relation accounted for.
/// Author IDs are stable; internal IDs are allocated anew per run.
pub struct NamedAutomaticLayout {
    orchestrator: Orchestrator,
    bindings: DiagramBindings,
}

impl NamedAutomaticLayout {
    pub(crate) fn compute(diagram: &DiagramDsl) -> Result<Self> {
        let (graph, bindings) = DslParser::new()
            .to_layout_graph_with_bindings(diagram)
            .map_err(TectonError::parse)?;
        let config = OrchestratorConfig {
            canvas_bounds: Rect::new(0.0, 0.0, diagram.canvas.width, diagram.canvas.height),
            ..OrchestratorConfig::default()
        };
        let mut orchestrator = Orchestrator::new(graph)
            .with_config(config)
            .with_named_terminals();
        orchestrator.run()?;
        let layout = orchestrator
            .layout()
            .ok_or_else(|| TectonError::layout("no computed layout"))?;
        let routes = orchestrator
            .routes()
            .ok_or_else(|| TectonError::routing("no computed routes"))?;
        for (name, id) in &bindings.nodes {
            if layout.find_node(*id).is_none() {
                return Err(TectonError::layout(format!(
                    "named node {name:?} was not laid out"
                )));
            }
        }
        for (name, edge) in &bindings.edges {
            let route = routes.get_route(edge.edge_id).ok_or_else(|| {
                TectonError::routing(format!("named edge {name:?} is missing its route"))
            })?;
            if route.segments.len() < 2
                || route.source_port != edge.source_port
                || route.target_port != edge.target_port
            {
                return Err(TectonError::routing(format!(
                    "named edge {name:?} has incomplete route or changed relation"
                )));
            }
        }
        Ok(Self {
            orchestrator,
            bindings,
        })
    }

    pub fn bindings(&self) -> &DiagramBindings {
        &self.bindings
    }

    pub fn layout(&self) -> &LayoutResult {
        // compute() checked this, and no mutation of orchestrator is exposed.
        self.orchestrator
            .layout()
            .expect("verified named automatic layout")
    }

    pub fn node(&self, author_id: &str) -> Option<&LayoutNode> {
        self.layout().find_node(self.bindings.node_id(author_id)?)
    }

    pub fn route(&self, author_id: &str) -> Option<&Route> {
        self.orchestrator
            .routes()?
            .get_route(self.bindings.edge(author_id)?.edge_id)
    }

    pub fn render(&self) -> Result<String> {
        self.orchestrator.render()
    }
}
