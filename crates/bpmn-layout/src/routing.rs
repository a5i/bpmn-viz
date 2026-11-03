//! Orthogonal sequence-flow routing: deterministic port selection, Manhattan
//! waypoints, obstacle avoidance and channel escalation (TASK-018 §5.4).
//!
//! Routing runs after all node rectangles are final. Escalation ladder per
//! flow (mirroring the reference's documented strategy, simplified to the M1
//! subset):
//!
//! 1. **Straight segment** — same row, adjacent or unobstructed span.
//! 2. **Bend template** — right-exit/left-enter elbow through a mid channel
//!    scanned deterministically from the midpoint outward.
//! 3. **Channel detour** — route through a clear horizontal lane below (then
//!    above) the involved shapes, for long same-band spans and same-rank
//!    bypasses.
//! 4. **Feedback channel** — back edges and upward cross edges leave the
//!    source bottom (top), run in an outer lane and dock on the target
//!    bottom (top).
//!
//! If no legal orthogonal route exists under the options, the flow fails with
//! a structured [`LayoutError::RoutingFailed`] instead of drawing through
//! unrelated shapes.
//!
//! Ports follow the row semantics: same-band flows exit right and enter
//! left; cross-band flows connect through elbows on the same ports; feedback
//! uses bottom/top docks. Obstacles are shape rectangles inset by 1 px, so
//! docking on a boundary or running along it is legal while interior entry
//! is not. Every candidate is checked against all unrelated shapes; ties are
//! resolved by the first candidate in the deterministic scan order.

use crate::api::{LayoutError, LayoutOptions};
use crate::geometry::{IRect, horizontal_segment_hits, simplify_waypoints, vertical_segment_hits};
use crate::graph::ScopeGraph;
use crate::placement::Placement;

/// Waypoint scan step for channel searches.
const SCAN_STEP: i64 = 8;
/// Maximum escalation attempts per ladder stage.
const MAX_SCAN_ATTEMPTS: i64 = 32;

/// Routed waypoints per flow (indexed by flow index).
#[derive(Debug, Clone)]
pub(crate) struct Routes {
    /// Waypoints per flow index; always at least two points.
    pub waypoints: Vec<Vec<(i64, i64)>>,
}

/// Routes all flows of the graph.
pub(crate) fn route(
    graph: &ScopeGraph,
    placement: &Placement,
    _options: &LayoutOptions,
) -> Result<Routes, LayoutError> {
    // Obstacle set: every shape rectangle inset by 1, tagged with its node.
    let obstacles: Vec<(usize, IRect)> = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(index, _node)| (index, placement.rects[index].inset(1)))
        .collect();

    let mut waypoints = Vec::with_capacity(graph.flows.len());
    for index in 0..graph.flows.len() {
        let flow = &graph.flows[index];
        let source = &graph.nodes[flow.source];
        let target = &graph.nodes[flow.target];
        let source_rect = placement.rects[flow.source];
        let target_rect = placement.rects[flow.target];
        let clearance = placement.clearance;

        // Route by classification; failures are attributed to the flow id so
        // errors always name the element (TASK-018 §4).
        let routed = route_one(
            flow,
            source_rect,
            target_rect,
            source.rank,
            target.rank,
            clearance,
            &obstacles,
        )
        .map_err(|_| LayoutError::RoutingFailed {
            flow: flow.id.clone(),
        })?;
        waypoints.push(simplify_waypoints(routed));
    }

    Ok(Routes { waypoints })
}

/// Dispatches one flow to its routing template by classification.
#[allow(clippy::too_many_arguments)]
fn route_one(
    flow: &crate::graph::FlowRec,
    source_rect: IRect,
    target_rect: IRect,
    source_rank: usize,
    target_rank: usize,
    clearance: i64,
    obstacles: &[(usize, IRect)],
) -> Result<Vec<(i64, i64)>, LayoutError> {
    if flow.source == flow.target {
        route_self_loop(source_rect, clearance, obstacles, flow.source)
    } else if flow.back_edge || target_rank < source_rank {
        route_feedback(
            source_rect,
            target_rect,
            clearance,
            obstacles,
            flow.source,
            flow.target,
        )
    } else if target_rank == source_rank {
        route_side_bypass(
            source_rect,
            target_rect,
            clearance,
            obstacles,
            flow.source,
            flow.target,
        )
    } else {
        route_forward(
            source_rect,
            target_rect,
            obstacles,
            flow.source,
            flow.target,
            clearance,
        )
    }
}

/// True when the horizontal segment does not penetrate any unrelated shape.
fn span_clear(
    x1: i64,
    x2: i64,
    y: i64,
    obstacles: &[(usize, IRect)],
    source: usize,
    target: usize,
) -> bool {
    obstacles.iter().all(|&(node, rect)| {
        node == source || node == target || !horizontal_segment_hits(x1, x2, y, &rect)
    })
}

/// True when the vertical segment does not penetrate any unrelated shape.
fn leg_clear(
    x: i64,
    y1: i64,
    y2: i64,
    obstacles: &[(usize, IRect)],
    source: usize,
    target: usize,
) -> bool {
    obstacles.iter().all(|&(node, rect)| {
        node == source || node == target || !vertical_segment_hits(x, y1, y2, &rect)
    })
}

/// Self loop: a deterministic right-side U around the source.
fn route_self_loop(
    rect: IRect,
    clearance: i64,
    obstacles: &[(usize, IRect)],
    source: usize,
) -> Result<Vec<(i64, i64)>, LayoutError> {
    let top_y = rect.center_y() - rect.h / 4;
    let bottom_y = rect.center_y() + rect.h / 4;
    let base = clearance + 20;
    for attempt in 0..MAX_SCAN_ATTEMPTS {
        let offset = base + attempt * SCAN_STEP;
        let x = rect.right() + offset;
        if leg_clear(x, top_y, bottom_y, obstacles, source, source) {
            return Ok(vec![
                (rect.right(), top_y),
                (x, top_y),
                (x, bottom_y),
                (rect.right(), bottom_y),
            ]);
        }
    }
    Err(LayoutError::RoutingFailed {
        flow: String::new(),
    })
}

/// Forward edge to a later rank: straight, elbow, or channel detour.
#[allow(clippy::too_many_arguments)]
fn route_forward(
    source: IRect,
    target: IRect,
    obstacles: &[(usize, IRect)],
    source_node: usize,
    target_node: usize,
    clearance: i64,
) -> Result<Vec<(i64, i64)>, LayoutError> {
    let sx = source.right();
    let sy = source.center_y();
    let tx = target.x;
    let ty = target.center_y();

    // 1. Straight when the row is unobstructed across the span.
    if sy == ty && span_clear(sx, tx, sy, obstacles, source_node, target_node) {
        return Ok(vec![(sx, sy), (tx, ty)]);
    }

    // 2. Elbow through a mid channel, scanning outward from the midpoint.
    //    Only meaningful for different rows: on one row the template
    //    degenerates into the (already rejected) straight line.
    if sy != ty {
        let mid = (sx + tx).div_euclid(2);
        let mut candidates = vec![mid];
        for step in 1..=16 {
            candidates.push(mid + step * SCAN_STEP);
            candidates.push(mid - step * SCAN_STEP);
        }
        let (low_y, high_y) = (sy.min(ty), ty.max(sy));
        for &mx in &candidates {
            if mx <= sx || mx >= tx {
                continue;
            }
            // Vertical leg plus both horizontal halves must be legal.
            if leg_clear(mx, low_y, high_y, obstacles, source_node, target_node)
                && span_clear(sx, mx, sy, obstacles, source_node, target_node)
                && span_clear(mx, tx, ty, obstacles, source_node, target_node)
            {
                return Ok(vec![(sx, sy), (mx, sy), (mx, ty), (tx, ty)]);
            }
        }
    }

    // 3. Channel detour below, then above.
    channel_detour(
        source,
        target,
        obstacles,
        source_node,
        target_node,
        clearance,
    )
}

/// Same-rank bypass around the right side; falls back to the channel ladder.
#[allow(clippy::too_many_arguments)]
fn route_side_bypass(
    source: IRect,
    target: IRect,
    clearance: i64,
    obstacles: &[(usize, IRect)],
    source_node: usize,
    target_node: usize,
) -> Result<Vec<(i64, i64)>, LayoutError> {
    let sy = source.center_y();
    let ty = target.center_y();
    let base = clearance + 20;
    for attempt in 0..MAX_SCAN_ATTEMPTS {
        let x = source.right() + base + attempt * SCAN_STEP;
        if leg_clear(
            x,
            sy.min(ty),
            sy.max(ty),
            obstacles,
            source_node,
            target_node,
        ) && span_clear(target.x, x, ty, obstacles, source_node, target_node)
        {
            return Ok(vec![(source.right(), sy), (x, sy), (x, ty), (target.x, ty)]);
        }
    }
    channel_detour(
        source,
        target,
        obstacles,
        source_node,
        target_node,
        clearance,
    )
}

/// Back edges and upward cross edges: outer feedback channel below (preferred)
/// or above, docking on the target's bottom/top center.
#[allow(clippy::too_many_arguments)]
fn route_feedback(
    source: IRect,
    target: IRect,
    clearance: i64,
    obstacles: &[(usize, IRect)],
    source_node: usize,
    target_node: usize,
) -> Result<Vec<(i64, i64)>, LayoutError> {
    // Below first: exit source bottom, run in an outer lane, enter target
    // bottom.
    let start_y = source.bottom().max(target.bottom()) + clearance;
    for attempt in 0..MAX_SCAN_ATTEMPTS {
        let y = start_y + attempt * SCAN_STEP;
        let (left_x, right_x) = if source.center_x() <= target.center_x() {
            (source.center_x(), target.center_x())
        } else {
            (target.center_x(), source.center_x())
        };
        if span_clear(left_x, right_x, y, obstacles, source_node, target_node)
            && leg_clear(
                source.center_x(),
                source.bottom(),
                y,
                obstacles,
                source_node,
                target_node,
            )
            && leg_clear(
                target.center_x(),
                target.bottom(),
                y,
                obstacles,
                source_node,
                target_node,
            )
        {
            return Ok(vec![
                (source.center_x(), source.bottom()),
                (source.center_x(), y),
                (target.center_x(), y),
                (target.center_x(), target.bottom()),
            ]);
        }
    }
    // Above: exit source top, run in an outer lane, enter target top.
    let start_y = source.y.min(target.y) - clearance;
    for attempt in 0..MAX_SCAN_ATTEMPTS {
        let y = start_y - attempt * SCAN_STEP;
        let (left_x, right_x) = if source.center_x() <= target.center_x() {
            (source.center_x(), target.center_x())
        } else {
            (target.center_x(), source.center_x())
        };
        if span_clear(left_x, right_x, y, obstacles, source_node, target_node)
            && leg_clear(
                source.center_x(),
                source.y,
                y,
                obstacles,
                source_node,
                target_node,
            )
            && leg_clear(
                target.center_x(),
                target.y,
                y,
                obstacles,
                source_node,
                target_node,
            )
        {
            return Ok(vec![
                (source.center_x(), source.y),
                (source.center_x(), y),
                (target.center_x(), y),
                (target.center_x(), target.y),
            ]);
        }
    }
    Err(LayoutError::RoutingFailed {
        flow: String::new(),
    })
}

/// Channel detour used by forward long edges and side bypasses: the route
/// leaves through the source's bottom-center, runs in a clear horizontal lane
/// below (preferred) or above the involved shapes, and docks on the target's
/// bottom-center (top-center above). Docking on the row-facing boundary keeps
/// the arrowhead direction consistent with the final approach segment
/// (regression: docking left while approaching vertically pointed the arrow
/// the wrong way).
fn channel_detour(
    source: IRect,
    target: IRect,
    obstacles: &[(usize, IRect)],
    source_node: usize,
    target_node: usize,
    clearance: i64,
) -> Result<Vec<(i64, i64)>, LayoutError> {
    // Below the involved shapes: bottom-center exit and entry.
    let start_y = source.bottom().max(target.bottom()) + clearance;
    for attempt in 0..MAX_SCAN_ATTEMPTS {
        let y = start_y + attempt * SCAN_STEP;
        let (left_x, right_x) = if source.center_x() <= target.center_x() {
            (source.center_x(), target.center_x())
        } else {
            (target.center_x(), source.center_x())
        };
        if span_clear(left_x, right_x, y, obstacles, source_node, target_node)
            && leg_clear(
                source.center_x(),
                source.bottom(),
                y,
                obstacles,
                source_node,
                target_node,
            )
            && leg_clear(
                target.center_x(),
                target.bottom(),
                y,
                obstacles,
                source_node,
                target_node,
            )
        {
            return Ok(vec![
                (source.center_x(), source.bottom()),
                (source.center_x(), y),
                (target.center_x(), y),
                (target.center_x(), target.bottom()),
            ]);
        }
    }
    // Above the involved shapes: top-center exit and entry.
    let start_y = source.y.min(target.y) - clearance;
    for attempt in 0..MAX_SCAN_ATTEMPTS {
        let y = start_y - attempt * SCAN_STEP;
        let (left_x, right_x) = if source.center_x() <= target.center_x() {
            (source.center_x(), target.center_x())
        } else {
            (target.center_x(), source.center_x())
        };
        if span_clear(left_x, right_x, y, obstacles, source_node, target_node)
            && leg_clear(
                source.center_x(),
                source.y,
                y,
                obstacles,
                source_node,
                target_node,
            )
            && leg_clear(
                target.center_x(),
                target.y,
                y,
                obstacles,
                source_node,
                target_node,
            )
        {
            return Ok(vec![
                (source.center_x(), source.y),
                (source.center_x(), y),
                (target.center_x(), y),
                (target.center_x(), target.y),
            ]);
        }
    }
    Err(LayoutError::RoutingFailed {
        flow: String::new(),
    })
}
