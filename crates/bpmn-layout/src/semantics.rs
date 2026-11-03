//! Semantic analysis: weakly connected components, start selection, primary
//! spine, cycle/back-edge classification, ranks and semantic bands (TASK-018
//! §5.2). No geometry is written here.
//!
//! Deterministic policy (mirroring the pinned reference behavior contract):
//!
//! - **Components** are weakly connected groups, ordered by the earliest
//!   declared node they contain; each is laid out independently and stacked
//!   vertically in that order.
//! - **Component start:** the earliest declared start event; otherwise the
//!   earliest node without an incoming sequence flow; otherwise the earliest
//!   declared node.
//! - **Primary spine (band 0):** walked one edge at a time, preferring an
//!   edge whose target can reach an end event, then the BPMN default flow,
//!   then declaration order. This keeps the primary narrative continuous so a
//!   dead-end alternative declared first cannot hijack it.
//! - **Cycles:** a deterministic DFS (from the component start, continuing
//!   from the earliest unvisited node until all nodes are visited) marks back
//!   edges; they are excluded from ranks and routed as feedback.
//! - **Ranks:** longest path over the remaining forward DAG.
//! - **Bands:** spine nodes sit at band 0. Alternatives fan per source: with
//!   a default flow present, non-default alternatives all fan below
//!   (+1, +2, …); without one they alternate below/above (+1, −1, +2, −2, …)
//!   in declaration order. A node's band comes from its first (lowest-decl)
//!   assigned incoming chain; unassigned stragglers fall back to band 0.

use std::collections::VecDeque;

use crate::api::LayoutError;
use crate::graph::ScopeGraph;

/// Per-component narrative grouping.
#[derive(Debug, Clone)]
pub(crate) struct Component {
    /// Node indexes in declaration order.
    pub nodes: Vec<usize>,
}

/// Result of the semantic analysis pass. `rank`/`band`/`component` are also
/// written back into the graph's node records; flow `back_edge`/`spine` flags
/// are updated in place.
#[derive(Debug, Clone)]
pub(crate) struct Analysis {
    /// Components ordered by their earliest declared node.
    pub components: Vec<Component>,
}

/// Runs the full semantic analysis, mutating the graph's annotation fields.
pub(crate) fn analyze(graph: &mut ScopeGraph) -> Result<Analysis, LayoutError> {
    assign_components(graph);
    mark_back_edges(graph);

    let mut components = Vec::new();
    for &component_id in component_ids(graph).iter() {
        let nodes: Vec<usize> = graph
            .nodes
            .iter()
            .filter(|node| node.component == component_id)
            .map(|node| node.decl)
            .collect();
        let start = select_start(graph, &nodes);
        mark_spine(graph, start);
        let order =
            topological_order(graph, &nodes).ok_or_else(|| LayoutError::CycleNotBroken {
                scope: graph.scope_id.clone(),
            })?;
        assign_ranks(graph, &order);
        assign_bands(graph, &nodes, start);
        components.push(Component { nodes });
    }
    Ok(Analysis { components })
}

/// Weakly connected components via union-find; node `component` fields are
/// set to the smallest declaration index of their component.
fn assign_components(graph: &mut ScopeGraph) {
    let mut parent: Vec<usize> = (0..graph.nodes.len()).collect();
    for flow in &graph.flows {
        let (a, b) = (flow.source, flow.target);
        let (ra, rb) = (find(&parent, a), find(&parent, b));
        if ra != rb {
            let (keep, drop) = if graph.nodes[ra].decl <= graph.nodes[rb].decl {
                (ra, rb)
            } else {
                (rb, ra)
            };
            parent[drop] = keep;
        }
    }
    // Normalize every member to the component root with the smallest decl.
    for node in 0..graph.nodes.len() {
        let root = find(&parent, node);
        graph.nodes[node].component = graph.nodes[root].decl;
    }
}

fn find(parent: &[usize], mut node: usize) -> usize {
    while parent[node] != node {
        node = parent[node];
    }
    node
}

/// Sorted unique component ids (they are declaration indexes, so this is
/// declaration order of the components).
fn component_ids(graph: &ScopeGraph) -> Vec<usize> {
    let mut ids: Vec<usize> = graph.nodes.iter().map(|node| node.component).collect();
    ids.sort_unstable();
    ids.dedup();
    ids
}

/// Deterministic DFS back-edge marking. Iterative (untrusted input must not
/// overflow the stack). After the component start, traversal continues from
/// the earliest unvisited node so cycles disconnected from the start are
/// still classified.
fn mark_back_edges(graph: &mut ScopeGraph) {
    // Node indexes equal declaration order, so iterating 0..n gives
    // deterministic DFS roots.
    let n = graph.nodes.len();
    let mut state: Vec<u8> = vec![0; n]; // 0 = unvisited, 1 = on stack, 2 = done

    for start in 0..n {
        if state[start] != 0 {
            continue;
        }
        // Explicit stack of (node, next child slot).
        let mut stack: Vec<(usize, usize)> = vec![(start, 0)];
        state[start] = 1;
        while let Some(&mut (node, ref mut child)) = stack.last_mut() {
            let outgoing = graph.nodes[node].outgoing.clone();
            if *child < outgoing.len() {
                let flow = outgoing[*child];
                *child += 1;
                let target = graph.flows[flow].target;
                match state[target] {
                    0 => {
                        state[target] = 1;
                        stack.push((target, 0));
                    }
                    1 => graph.flows[flow].back_edge = true,
                    _ => {}
                }
            } else {
                state[node] = 2;
                stack.pop();
            }
        }
    }
}

/// Start selection: earliest declared start event, else earliest node without
/// incoming flow, else earliest declared node.
fn select_start(graph: &ScopeGraph, nodes: &[usize]) -> usize {
    let mut candidates = nodes.to_vec();
    candidates.sort_by_key(|&node| graph.nodes[node].decl);
    for node in &candidates {
        if graph.nodes[*node].is_start_event {
            return *node;
        }
    }
    for node in &candidates {
        if graph.nodes[*node].incoming.is_empty() {
            return *node;
        }
    }
    candidates[0]
}

/// Nodes that can reach an end event over forward (non-back) edges.
fn reaches_end(graph: &ScopeGraph) -> Vec<bool> {
    let n = graph.nodes.len();
    let mut reverse: Vec<Vec<usize>> = vec![Vec::new(); n];
    for flow in &graph.flows {
        if !flow.back_edge {
            reverse[flow.target].push(flow.source);
        }
    }
    let mut reached = vec![false; n];
    let mut queue: VecDeque<usize> = VecDeque::new();
    for (node, record) in graph.nodes.iter().enumerate() {
        if record.is_end {
            reached[node] = true;
            queue.push_back(node);
        }
    }
    while let Some(node) = queue.pop_front() {
        for &source in &reverse[node] {
            if !reached[source] {
                reached[source] = true;
                queue.push_back(source);
            }
        }
    }
    reached
}

/// Walks the primary spine from `start`, marking `spine` flows. Preference:
/// target reaches an end event → default flow → declaration order.
fn mark_spine(graph: &mut ScopeGraph, start: usize) {
    let reached = reaches_end(graph);
    let mut visited = vec![false; graph.nodes.len()];
    let mut current = Some(start);
    visited[start] = true;
    while let Some(node) = current {
        let outgoing = graph.nodes[node].outgoing.clone();
        let mut chosen: Option<usize> = None;
        // 1. forward flow whose target reaches an end event and is unvisited
        for &flow in outgoing.iter() {
            if graph.flows[flow].back_edge {
                continue;
            }
            let target = graph.flows[flow].target;
            if !visited[target] && reached[target] {
                chosen = Some(flow);
                break;
            }
        }
        // 2. the default flow, if it qualifies
        if chosen.is_none() {
            for &flow in outgoing.iter() {
                if graph.flows[flow].back_edge || !graph.flows[flow].is_default {
                    continue;
                }
                let target = graph.flows[flow].target;
                if !visited[target] {
                    chosen = Some(flow);
                    break;
                }
            }
        }
        // 3. first declared unvisited forward flow
        if chosen.is_none() {
            for &flow in outgoing.iter() {
                if graph.flows[flow].back_edge {
                    continue;
                }
                let target = graph.flows[flow].target;
                if !visited[target] {
                    chosen = Some(flow);
                    break;
                }
            }
        }
        match chosen {
            Some(flow) => {
                let target = graph.flows[flow].target;
                graph.flows[flow].spine = true;
                visited[target] = true;
                current = Some(target);
            }
            None => current = None,
        }
    }
}

/// Deterministic topological order (Kahn) over forward edges; `None` when a
/// cycle survives back-edge removal (engine defect).
fn topological_order(graph: &ScopeGraph, nodes: &[usize]) -> Option<Vec<usize>> {
    let mut indegree: Vec<usize> = vec![0; graph.nodes.len()];
    for node in nodes {
        indegree[*node] = graph.nodes[*node]
            .incoming
            .iter()
            .filter(|&&flow| !graph.flows[flow].back_edge)
            .count();
    }
    let mut ready: Vec<usize> = nodes
        .iter()
        .copied()
        .filter(|&node| indegree[node] == 0)
        .collect();
    ready.sort_by_key(|&node| graph.nodes[node].decl);
    let mut order = Vec::with_capacity(nodes.len());
    while let Some((position, node)) = ready
        .iter()
        .copied()
        .enumerate()
        .min_by_key(|&(_, candidate)| graph.nodes[candidate].decl)
    {
        ready.remove(position);
        order.push(node);
        let outgoing = graph.nodes[node].outgoing.clone();
        for flow in outgoing {
            if graph.flows[flow].back_edge {
                continue;
            }
            let target = graph.flows[flow].target;
            indegree[target] -= 1;
            if indegree[target] == 0 {
                ready.push(target);
            }
        }
    }
    if order.len() == nodes.len() {
        Some(order)
    } else {
        None
    }
}

/// Longest-path ranks over the forward DAG, in topological order.
fn assign_ranks(graph: &mut ScopeGraph, order: &[usize]) {
    for &node in order {
        let mut rank = 0usize;
        let incoming = graph.nodes[node].incoming.clone();
        for flow in incoming {
            if graph.flows[flow].back_edge {
                continue;
            }
            let source = graph.flows[flow].source;
            rank = rank.max(graph.nodes[source].rank + 1);
        }
        graph.nodes[node].rank = rank;
    }
}

/// Semantic bands. Spine nodes are 0; alternatives fan per source policy.
fn assign_bands(graph: &mut ScopeGraph, nodes: &[usize], start: usize) {
    let mut assigned = vec![false; graph.nodes.len()];
    let mut queue: VecDeque<usize> = VecDeque::new();

    // Spine nodes: band 0. Seed the BFS in spine chain order from the start,
    // marking every walked node as assigned so branch chains cannot reassign
    // them (the join stays on the spine row).
    let mut cursor = Some(start);
    while let Some(node) = cursor {
        assigned[node] = true;
        graph.nodes[node].band = 0;
        queue.push_back(node);
        let outgoing = graph.nodes[node].outgoing.clone();
        cursor = outgoing
            .iter()
            .find(|&&flow| graph.flows[flow].spine)
            .map(|&flow| graph.flows[flow].target);
    }

    // Per-source alternative deltas, applied in BFS order.
    while let Some(source) = queue.pop_front() {
        let outgoing = graph.nodes[source].outgoing.clone();
        let has_default = outgoing.iter().any(|&f| graph.flows[f].is_default);
        let mut alternative_index = 0usize;
        for flow in outgoing {
            let record = &graph.flows[flow];
            if record.back_edge || record.spine {
                continue;
            }
            let target = record.target;
            let delta: i64 = if has_default {
                1 + alternative_index as i64
            } else {
                let step = (alternative_index / 2 + 1) as i64;
                if alternative_index.is_multiple_of(2) {
                    step
                } else {
                    -step
                }
            };
            alternative_index += 1;
            if !assigned[target] {
                assigned[target] = true;
                graph.nodes[target].band = graph.nodes[source].band + delta;
                queue.push_back(target);
            }
        }
    }

    // Deterministic fallback for nodes unreachable from any banded node.
    for &node in nodes {
        if !assigned[node] {
            graph.nodes[node].band = 0;
        }
    }
}
