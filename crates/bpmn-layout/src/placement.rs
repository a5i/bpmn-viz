//! Node placement: rank columns, semantic band rows, within-band stacking,
//! disconnected component packing and margin normalization (TASK-018 §5.3).
//!
//! Writes node bounds only. All arithmetic is integer-based; the semantics of
//! the layout mirror the pinned reference: each rank becomes an x position
//! (width of its widest node, nodes centered in the column), each semantic
//! band becomes a y row (`node_gap + band height` pitch, nodes centered on
//! the row), components are stacked vertically in declaration order, and the
//! whole diagram is normalized to the outer margin.

use crate::api::LayoutOptions;
use crate::geometry::IRect;
use crate::graph::ScopeGraph;
use crate::semantics::Analysis;
use crate::validation::round_option;

/// Outer margin kept around the final diagram (reference: 80).
pub(crate) const OUTER_MARGIN: i64 = 80;
/// Height of one semantic band (reference: `SEMANTIC_BAND_HEIGHT` = 80).
pub(crate) const BAND_HEIGHT: i64 = 80;

/// The placement result: one rectangle per node, indexed by node index.
#[derive(Debug, Clone)]
pub(crate) struct Placement {
    /// Final node rectangles (translation already applied).
    pub rects: Vec<IRect>,
    /// Rounded routing clearance.
    pub clearance: i64,
}

/// Assigns final rectangles to every node of the graph.
pub(crate) fn place(graph: &ScopeGraph, analysis: &Analysis, options: &LayoutOptions) -> Placement {
    let rank_gap = round_option(options.rank_gap).max(1);
    let node_gap = round_option(options.node_gap).max(0);
    let clearance = round_option(options.routing_clearance).max(0);
    let pitch = node_gap + BAND_HEIGHT;

    let mut rects: Vec<IRect> = vec![IRect::new(0, 0, 0, 0); graph.nodes.len()];

    // Per component: rank columns, band rows.
    for component in &analysis.components {
        // Rank widths: widest node per rank.
        let mut rank_count = 0usize;
        for &node in &component.nodes {
            rank_count = rank_count.max(graph.nodes[node].rank + 1);
        }
        let mut rank_width = vec![0i64; rank_count];
        for &node in &component.nodes {
            let record = &graph.nodes[node];
            rank_width[record.rank] = rank_width[record.rank].max(record.width);
        }
        // Rank x offsets (cumulative).
        let mut rank_x = vec![0i64; rank_count];
        for rank in 1..rank_count {
            rank_x[rank] = rank_x[rank - 1] + rank_width[rank - 1] + rank_gap;
        }

        // Group nodes by (rank, band); members keep declaration order (the
        // map merges same-key groups that are non-adjacent in declaration
        // order, and iteration is deterministic by key).
        let mut groups = std::collections::BTreeMap::new();
        for &node in &component.nodes {
            let key = (graph.nodes[node].rank, graph.nodes[node].band);
            groups.entry(key).or_insert_with(Vec::new).push(node);
        }
        let groups: Vec<((usize, i64), Vec<usize>)> = groups.into_iter().collect();

        // Place each group centered on its rank column and band row; groups
        // with multiple members stack vertically around the band center.
        for ((rank, band), members) in &groups {
            let band_center = band * pitch;
            let total_height: i64 = members
                .iter()
                .map(|&node| graph.nodes[node].height)
                .sum::<i64>()
                + node_gap * (members.len() as i64 - 1);
            let mut y = band_center - div2_floor(total_height);
            for &node in members {
                let record = &graph.nodes[node];
                let centered = (rank_width[*rank] - record.width + 1) / 2;
                rects[node] = IRect::new(rank_x[*rank] + centered, y, record.width, record.height);
                y += record.height + node_gap;
            }
        }
    }

    // Stack components vertically in declaration order (component ids are
    // declaration indexes, and analysis.components is sorted by them).
    let mut offset = 0i64;
    for (index, component) in analysis.components.iter().enumerate() {
        if index > 0 {
            offset += node_gap;
        }
        if offset > 0 {
            for &node in &component.nodes {
                rects[node].y += offset;
            }
        }
        // Next component starts below this one.
        let bottom = component
            .nodes
            .iter()
            .map(|&node| rects[node].bottom())
            .max()
            .unwrap_or(offset);
        offset = bottom;
    }

    // Normalize: translate so the diagram starts at the outer margin.
    if !graph.nodes.is_empty() {
        let min_x = rects.iter().map(|rect| rect.x).min().unwrap_or(0);
        let min_y = rects.iter().map(|rect| rect.y).min().unwrap_or(0);
        let dx = OUTER_MARGIN - min_x;
        let dy = OUTER_MARGIN - min_y;
        for rect in &mut rects {
            rect.x += dx;
            rect.y += dy;
        }
    }

    Placement { rects, clearance }
}

/// Floor division by two (deterministic for negative band centers).
fn div2_floor(value: i64) -> i64 {
    value.div_euclid(2)
}
