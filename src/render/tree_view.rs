//! Radial tree view renderer — PoE-style skill tree visualization.
//!
//! Used for both the player skill tree and per-item crucible trees.
//! Supports pan (drag), zoom (scroll), hover tooltips, and click-to-allocate.

use web_sys::CanvasRenderingContext2d;
use crate::game::TreeCamera;

/// Node state for rendering purposes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NodeVisual {
    /// Already allocated / unlocked
    Allocated,
    /// Can be allocated right now (adjacent + has points/xp)
    Available,
    /// Locked (not adjacent or no points)
    Locked,
}

/// A single node to render in the tree view.
pub struct TreeNodeView {
    /// World-space position
    pub wx: f64,
    pub wy: f64,
    /// Display name
    pub name: String,
    /// Description / effect text
    pub description: String,
    /// Visual state
    pub visual: NodeVisual,
    /// Whether this is a notable / keystone node
    pub is_notable: bool,
    /// Color for the node ring (cluster color, rarity color, etc.)
    pub color: String,
}

/// Data required to render a full tree view.
pub struct TreeViewData {
    pub nodes: Vec<TreeNodeView>,
    pub edges: Vec<(usize, usize)>,
    pub title: String,
    pub subtitle: String,
    /// Index of hovered node, if any
    pub hover: Option<usize>,
}

// ── Rendering constants ──────────────────────────────────────────────────────

const NODE_RADIUS: f64 = 14.0;
const NOTABLE_RADIUS: f64 = 20.0;
const EDGE_COLOR: &str = "rgba(100, 120, 160, 0.4)";
const EDGE_ACTIVE_COLOR: &str = "rgba(140, 200, 255, 0.7)";
const BG_COLOR: &str = "rgba(6, 5, 18, 0.95)";

const ALLOCATED_FILL: &str = "#22aa44";
const AVAILABLE_FILL: &str = "#334466";
const LOCKED_FILL: &str = "#1a1a2a";
const AVAILABLE_GLOW: &str = "rgba(100, 180, 255, 0.5)";
const HOVER_GLOW: &str = "rgba(255, 220, 100, 0.8)";

/// Draw a complete tree view onto the canvas.
pub fn draw_tree_view(
    ctx: &CanvasRenderingContext2d,
    data: &TreeViewData,
    camera: &TreeCamera,
    canvas_w: f64,
    canvas_h: f64,
    spacing: f64,
) {
    // Background
    ctx.set_fill_style_str(BG_COLOR);
    ctx.fill_rect(0.0, 0.0, canvas_w, canvas_h);

    // Title
    ctx.set_fill_style_str("#ffcc33");
    ctx.set_font("bold 18px monospace");
    ctx.set_text_align("center");
    ctx.fill_text(&data.title, canvas_w / 2.0, 28.0).ok();

    // Subtitle
    ctx.set_fill_style_str("#aaccff");
    ctx.set_font("13px monospace");
    ctx.fill_text(&data.subtitle, canvas_w / 2.0, 48.0).ok();

    // Draw edges first (behind nodes)
    ctx.set_line_width(2.0 * camera.zoom.max(0.5));
    for &(a, b) in &data.edges {
        if a >= data.nodes.len() || b >= data.nodes.len() {
            continue;
        }
        let na = &data.nodes[a];
        let nb = &data.nodes[b];
        let (sx_a, sy_a) = camera.world_to_screen(na.wx * spacing, na.wy * spacing, canvas_w, canvas_h);
        let (sx_b, sy_b) = camera.world_to_screen(nb.wx * spacing, nb.wy * spacing, canvas_w, canvas_h);

        let both_allocated = na.visual == NodeVisual::Allocated && nb.visual == NodeVisual::Allocated;
        ctx.set_stroke_style_str(if both_allocated {
            EDGE_ACTIVE_COLOR
        } else {
            EDGE_COLOR
        });
        ctx.begin_path();
        ctx.move_to(sx_a, sy_a);
        ctx.line_to(sx_b, sy_b);
        ctx.stroke();
    }

    // Draw nodes
    for (i, node) in data.nodes.iter().enumerate() {
        let (sx, sy) = camera.world_to_screen(node.wx * spacing, node.wy * spacing, canvas_w, canvas_h);

        // Skip off-screen nodes
        let margin = 60.0;
        if sx < -margin || sx > canvas_w + margin || sy < -margin || sy > canvas_h + margin {
            continue;
        }

        let base_r = if node.is_notable { NOTABLE_RADIUS } else { NODE_RADIUS };
        let r = base_r * camera.zoom;
        let is_hovered = data.hover == Some(i);

        // Glow for available / hovered nodes
        if is_hovered {
            ctx.set_shadow_color(HOVER_GLOW);
            ctx.set_shadow_blur(16.0 * camera.zoom);
        } else if node.visual == NodeVisual::Available {
            ctx.set_shadow_color(AVAILABLE_GLOW);
            ctx.set_shadow_blur(10.0 * camera.zoom);
        } else if node.visual == NodeVisual::Allocated {
            ctx.set_shadow_color(&node.color);
            ctx.set_shadow_blur(6.0 * camera.zoom);
        }

        // Fill
        let fill = match node.visual {
            NodeVisual::Allocated => ALLOCATED_FILL,
            NodeVisual::Available => AVAILABLE_FILL,
            NodeVisual::Locked => LOCKED_FILL,
        };
        ctx.set_fill_style_str(fill);
        ctx.begin_path();
        ctx.arc(sx, sy, r, 0.0, std::f64::consts::TAU).ok();
        ctx.fill();

        // Reset shadow
        ctx.set_shadow_blur(0.0);

        // Ring (border) in cluster/node color
        let ring_color = if node.visual == NodeVisual::Locked {
            "rgba(80, 80, 100, 0.5)"
        } else {
            &node.color
        };
        ctx.set_stroke_style_str(ring_color);
        ctx.set_line_width(if node.is_notable { 3.0 } else { 2.0 } * camera.zoom.max(0.5));
        ctx.begin_path();
        ctx.arc(sx, sy, r, 0.0, std::f64::consts::TAU).ok();
        ctx.stroke();

        // Node icon/label (short text inside node at higher zoom)
        if camera.zoom > 0.6 {
            let font_size = (10.0 * camera.zoom).clamp(8.0, 14.0);
            ctx.set_font(&format!("{:.0}px monospace", font_size));
            ctx.set_text_align("center");
            ctx.set_fill_style_str(match node.visual {
                NodeVisual::Allocated => "#ffffff",
                NodeVisual::Available => "#ccddff",
                NodeVisual::Locked => "#666677",
            });

            // For notables, show a star; for regular nodes, show first char of name
            if node.is_notable {
                ctx.fill_text("★", sx, sy + font_size * 0.35).ok();
            } else if !node.name.is_empty() {
                let ch: String = node.name.chars().take(2).collect();
                ctx.fill_text(&ch, sx, sy + font_size * 0.35).ok();
            }
        }

        // Name label below node at higher zoom
        if camera.zoom > 0.8 {
            let font_size = (9.0 * camera.zoom).clamp(7.0, 11.0);
            ctx.set_font(&format!("{:.0}px monospace", font_size));
            ctx.set_fill_style_str(match node.visual {
                NodeVisual::Allocated => "#88cc88",
                NodeVisual::Available => "#8899bb",
                NodeVisual::Locked => "#444455",
            });
            ctx.fill_text(&node.name, sx, sy + r + font_size + 2.0).ok();
        }
    }

    // Hover tooltip
    if let Some(hi) = data.hover {
        if hi < data.nodes.len() {
            let node = &data.nodes[hi];
            let (sx, sy) = camera.world_to_screen(node.wx * spacing, node.wy * spacing, canvas_w, canvas_h);
            let base_r = if node.is_notable { NOTABLE_RADIUS } else { NODE_RADIUS };
            let r = base_r * camera.zoom;

            draw_tooltip(ctx, &node.name, &node.description, &node.color, node.visual, sx, sy - r - 8.0, canvas_w);
        }
    }

    // Footer
    ctx.set_fill_style_str("#555577");
    ctx.set_font("11px monospace");
    ctx.set_text_align("center");
    ctx.fill_text(
        "Scroll: Zoom • Drag: Pan • Click: Allocate • Esc: Close",
        canvas_w / 2.0,
        canvas_h - 10.0,
    ).ok();
}

/// Draw a tooltip box above a node.
fn draw_tooltip(
    ctx: &CanvasRenderingContext2d,
    name: &str,
    description: &str,
    color: &str,
    visual: NodeVisual,
    cx: f64,
    cy: f64,
    canvas_w: f64,
) {
    let padding = 8.0;
    let line_h = 16.0;

    // Estimate text widths (monospace: ~7.2px per char at 12px, ~6.6px at 11px)
    let name_w = name.len() as f64 * 7.2;
    let desc_w = description.len() as f64 * 6.6;

    let status_text = match visual {
        NodeVisual::Allocated => "[Allocated]",
        NodeVisual::Available => "[Click to allocate]",
        NodeVisual::Locked => "[Locked]",
    };
    let status_w = status_text.len() as f64 * 6.6;

    let box_w = name_w.max(desc_w).max(status_w) + padding * 2.0;
    let box_h = line_h * 3.0 + padding * 2.0;

    // Position: center above node, clamp to screen
    let bx = (cx - box_w / 2.0).clamp(4.0, canvas_w - box_w - 4.0);
    let by = (cy - box_h).max(4.0);

    // Background
    ctx.set_fill_style_str("rgba(10, 8, 24, 0.95)");
    ctx.fill_rect(bx, by, box_w, box_h);

    // Border
    ctx.set_stroke_style_str(color);
    ctx.set_line_width(1.5);
    ctx.stroke_rect(bx, by, box_w, box_h);

    // Name
    ctx.set_fill_style_str("#ffcc33");
    ctx.set_font("bold 12px monospace");
    ctx.set_text_align("left");
    ctx.fill_text(name, bx + padding, by + padding + 12.0).ok();

    // Description
    ctx.set_fill_style_str("#ccccdd");
    ctx.set_font("11px monospace");
    ctx.fill_text(description, bx + padding, by + padding + 12.0 + line_h).ok();

    // Status
    let status_color = match visual {
        NodeVisual::Allocated => "#44ff44",
        NodeVisual::Available => "#88ccff",
        NodeVisual::Locked => "#666666",
    };
    ctx.set_fill_style_str(status_color);
    ctx.fill_text(status_text, bx + padding, by + padding + 12.0 + line_h * 2.0).ok();
}
