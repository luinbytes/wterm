//! Layout management for split panes
//!
//! Provides a tree-based layout system for managing multiple terminal panes.

use crate::terminal::grid::TerminalGrid;
use crate::terminal::parser::TerminalParser;
use crate::terminal::pty::PtySession;
use std::sync::{Arc, Mutex, OnceLock};
use uuid::Uuid;

/// Rectangle representing a pane's bounds
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    /// Create a new rectangle
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Check if a point is inside this rectangle
    pub fn contains(&self, x: u32, y: u32) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }
}

/// Direction for splitting panes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SplitDirection {
    /// Split horizontally (side by side)
    Horizontal,
    /// Split vertically (stacked)
    Vertical,
}

/// A single terminal pane
pub struct Pane {
    /// Unique identifier for this pane
    pub id: Uuid,
    /// Pane title (can be set dynamically from shell)
    #[allow(dead_code)]
    pub title: String,
    /// PTY session for this pane
    pub pty: Arc<Mutex<PtySession>>,
    /// Terminal grid (screen buffer)
    pub grid: TerminalGrid,
    /// Terminal parser
    pub parser: TerminalParser,
    /// Calculated bounds during layout
    pub bounds: Rect,
}

impl Pane {
    /// Create a new pane with the given PTY and dimensions
    pub fn new(pty: PtySession, cols: usize, rows: usize, bounds: Rect) -> Self {
        let id = Uuid::new_v4();
        let title = "Terminal".to_string();

        Self {
            id,
            title,
            pty: Arc::new(Mutex::new(pty)),
            grid: TerminalGrid::with_size(cols, rows),
            parser: TerminalParser::new(),
            bounds,
        }
    }

    /// Get the terminal dimensions from the bounds
    pub fn terminal_size(&self, cell_width: u32, cell_height: u32) -> (usize, usize) {
        let cols = (self.bounds.width / cell_width) as usize;
        let rows = (self.bounds.height / cell_height) as usize;
        (cols.max(1), rows.max(1))
    }
}

/// A node in the layout tree
#[allow(clippy::vec_box)]
pub enum LayoutNode {
    /// Leaf node containing a single pane
    Pane(Box<Pane>),
    /// Horizontal split (panes side by side)
    HorizontalSplit {
        children: Vec<Box<LayoutNode>>,
        ratios: Vec<f32>,
    },
    /// Vertical split (panes stacked)
    VerticalSplit {
        children: Vec<Box<LayoutNode>>,
        ratios: Vec<f32>,
    },
}

#[allow(dead_code)]
impl LayoutNode {
    /// Check if this node is a pane
    pub fn is_pane(&self) -> bool {
        matches!(self, LayoutNode::Pane(_))
    }

    /// Check if this node is a split
    pub fn is_split(&self) -> bool {
        !self.is_pane()
    }

    /// Get the pane ID if this is a pane node
    pub fn pane_id(&self) -> Option<Uuid> {
        match self {
            LayoutNode::Pane(pane) => Some(pane.id),
            _ => None,
        }
    }

    /// Count the total number of panes in this subtree
    pub fn pane_count(&self) -> usize {
        match self {
            LayoutNode::Pane(_) => 1,
            LayoutNode::HorizontalSplit { children, .. } => {
                children.iter().map(|c| c.pane_count()).sum()
            }
            LayoutNode::VerticalSplit { children, .. } => {
                children.iter().map(|c| c.pane_count()).sum()
            }
        }
    }

    /// Find a pane by ID in this subtree
    pub fn find_pane(&self, id: Uuid) -> Option<&Pane> {
        match self {
            LayoutNode::Pane(pane) if pane.id == id => Some(pane),
            LayoutNode::Pane(_) => None,
            LayoutNode::HorizontalSplit { children, .. } => {
                children.iter().find_map(|c| c.find_pane(id))
            }
            LayoutNode::VerticalSplit { children, .. } => {
                children.iter().find_map(|c| c.find_pane(id))
            }
        }
    }

    /// Find a pane by ID (mutable) in this subtree
    pub fn find_pane_mut(&mut self, id: Uuid) -> Option<&mut Pane> {
        match self {
            LayoutNode::Pane(pane) if pane.id == id => Some(pane),
            LayoutNode::Pane(_) => None,
            LayoutNode::HorizontalSplit { children, .. } => {
                children.iter_mut().find_map(|c| c.find_pane_mut(id))
            }
            LayoutNode::VerticalSplit { children, .. } => {
                children.iter_mut().find_map(|c| c.find_pane_mut(id))
            }
        }
    }

    /// Collect all pane IDs in this subtree
    pub fn collect_pane_ids(&self) -> Vec<Uuid> {
        match self {
            LayoutNode::Pane(pane) => vec![pane.id],
            LayoutNode::HorizontalSplit { children, .. } => {
                children.iter().flat_map(|c| c.collect_pane_ids()).collect()
            }
            LayoutNode::VerticalSplit { children, .. } => {
                children.iter().flat_map(|c| c.collect_pane_ids()).collect()
            }
        }
    }
}

/// Layout tree managing all panes
pub struct LayoutTree {
    /// Root node of the layout tree
    root: LayoutNode,
    /// Currently focused pane ID
    focused_pane: Uuid,
}

impl LayoutTree {
    /// Create a new layout tree with a single pane
    pub fn new(initial_pane: Pane) -> Self {
        let focused_pane = initial_pane.id;
        let root = LayoutNode::Pane(Box::new(initial_pane));

        Self { root, focused_pane }
    }

    /// Get the focused pane ID
    pub fn focused_pane_id(&self) -> Uuid {
        self.focused_pane
    }

    /// Set focus to a specific pane
    pub fn set_focus(&mut self, pane_id: Uuid) -> bool {
        if self.root.find_pane(pane_id).is_some() {
            self.focused_pane = pane_id;
            true
        } else {
            false
        }
    }

    /// Get the currently focused pane
    pub fn focused_pane(&self) -> Option<&Pane> {
        self.root.find_pane(self.focused_pane)
    }

    /// Get the currently focused pane (mutable)
    pub fn focused_pane_mut(&mut self) -> Option<&mut Pane> {
        self.root.find_pane_mut(self.focused_pane)
    }

    /// Get a pane by ID
    pub fn get_pane(&self, id: Uuid) -> Option<&Pane> {
        self.root.find_pane(id)
    }

    /// Get a pane by ID (mutable)
    pub fn get_pane_mut(&mut self, id: Uuid) -> Option<&mut Pane> {
        self.root.find_pane_mut(id)
    }

    /// Count total panes
    #[allow(dead_code)]
    pub fn pane_count(&self) -> usize {
        self.root.pane_count()
    }

    /// Get all pane IDs
    pub fn all_pane_ids(&self) -> Vec<Uuid> {
        self.root.collect_pane_ids()
    }

    /// Split the focused pane in the given direction
    pub fn split_focused(
        &mut self,
        direction: SplitDirection,
        new_pane: Pane,
    ) -> Result<(), String> {
        if self.root.pane_count() >= 8 {
            return Err("Maximum pane limit (8) reached".to_string());
        }

        let focused_id = self.focused_pane;
        let new_pane_id = new_pane.id;

        // Replace the focused pane with a split containing the old and new panes
        if let Some(_pane) = self.root.find_pane(focused_id) {
            // Extract the old pane by replacing root with a placeholder
            // This is a workaround for not being able to clone PtySession
            let old_root = std::mem::replace(
                &mut self.root,
                LayoutNode::Pane(Box::new(create_placeholder_pane())),
            );

            // Try to split the old root
            match self.try_split_node(old_root, focused_id, direction, new_pane) {
                Ok(new_root) => {
                    self.root = new_root;
                    self.focused_pane = new_pane_id;
                    Ok(())
                }
                Err((old_root, err)) => {
                    self.root = old_root;
                    Err(err)
                }
            }
        } else {
            Err("Focused pane not found".to_string())
        }
    }

    /// Try to split a node, returning the node on error
    fn try_split_node(
        &self,
        node: LayoutNode,
        target_id: Uuid,
        direction: SplitDirection,
        new_pane: Pane,
    ) -> Result<LayoutNode, (LayoutNode, String)> {
        match node {
            LayoutNode::Pane(pane) if pane.id == target_id => {
                // Found the pane to split
                let old_pane = pane;
                let split = match direction {
                    SplitDirection::Horizontal => LayoutNode::HorizontalSplit {
                        children: vec![
                            Box::new(LayoutNode::Pane(old_pane)),
                            Box::new(LayoutNode::Pane(Box::new(new_pane))),
                        ],
                        ratios: vec![0.5, 0.5],
                    },
                    SplitDirection::Vertical => LayoutNode::VerticalSplit {
                        children: vec![
                            Box::new(LayoutNode::Pane(old_pane)),
                            Box::new(LayoutNode::Pane(Box::new(new_pane))),
                        ],
                        ratios: vec![0.5, 0.5],
                    },
                };
                Ok(split)
            }
            LayoutNode::Pane(pane) => {
                // Not the target pane, return unchanged
                Ok(LayoutNode::Pane(pane))
            }
            LayoutNode::HorizontalSplit {
                mut children,
                ratios,
            } => {
                // Recurse into children
                for child in &mut children {
                    if child.find_pane(target_id).is_some() {
                        // Take ownership of child
                        let old_child = std::mem::replace(
                            child,
                            Box::new(LayoutNode::Pane(Box::new(create_placeholder_pane()))),
                        );
                        match self.try_split_node(*old_child, target_id, direction, new_pane) {
                            Ok(new_child) => {
                                **child = new_child;
                                break;
                            }
                            Err((old_child, err)) => {
                                **child = old_child;
                                return Err((
                                    LayoutNode::HorizontalSplit { children, ratios },
                                    err,
                                ));
                            }
                        }
                    }
                }
                Ok(LayoutNode::HorizontalSplit { children, ratios })
            }
            LayoutNode::VerticalSplit {
                mut children,
                ratios,
            } => {
                // Recurse into children
                for child in &mut children {
                    if child.find_pane(target_id).is_some() {
                        // Take ownership of child
                        let old_child = std::mem::replace(
                            child,
                            Box::new(LayoutNode::Pane(Box::new(create_placeholder_pane()))),
                        );
                        match self.try_split_node(*old_child, target_id, direction, new_pane) {
                            Ok(new_child) => {
                                **child = new_child;
                                break;
                            }
                            Err((old_child, err)) => {
                                **child = old_child;
                                return Err((LayoutNode::VerticalSplit { children, ratios }, err));
                            }
                        }
                    }
                }
                Ok(LayoutNode::VerticalSplit { children, ratios })
            }
        }
    }

    /// Calculate layout bounds for all panes
    pub fn calculate_layout(&mut self, total_bounds: Rect) {
        let root = std::mem::replace(
            &mut self.root,
            LayoutNode::Pane(Box::new(create_placeholder_pane())),
        );
        let new_root = self.calculate_node_layout_owned(root, total_bounds);
        self.root = new_root;
    }

    /// Recursively calculate bounds for a node (takes ownership to avoid borrow issues)
    fn calculate_node_layout_owned(&self, node: LayoutNode, bounds: Rect) -> LayoutNode {
        match node {
            LayoutNode::Pane(mut pane) => {
                pane.bounds = bounds;
                LayoutNode::Pane(pane)
            }
            LayoutNode::HorizontalSplit { children, ratios } => {
                let children_len = children.len();
                let mut x = bounds.x;
                let mut new_children = Vec::with_capacity(children_len);
                for (i, child) in children.into_iter().enumerate() {
                    let ratio = ratios.get(i).copied().unwrap_or(1.0 / children_len as f32);
                    let width = (bounds.width as f32 * ratio) as u32;
                    let child_bounds = Rect::new(x, bounds.y, width, bounds.height);
                    let new_child = self.calculate_node_layout_owned(*child, child_bounds);
                    new_children.push(Box::new(new_child));
                    x += width;
                }
                LayoutNode::HorizontalSplit {
                    children: new_children,
                    ratios,
                }
            }
            LayoutNode::VerticalSplit { children, ratios } => {
                let children_len = children.len();
                let mut y = bounds.y;
                let mut new_children = Vec::with_capacity(children_len);
                for (i, child) in children.into_iter().enumerate() {
                    let ratio = ratios.get(i).copied().unwrap_or(1.0 / children_len as f32);
                    let height = (bounds.height as f32 * ratio) as u32;
                    let child_bounds = Rect::new(bounds.x, y, bounds.width, height);
                    let new_child = self.calculate_node_layout_owned(*child, child_bounds);
                    new_children.push(Box::new(new_child));
                    y += height;
                }
                LayoutNode::VerticalSplit {
                    children: new_children,
                    ratios,
                }
            }
        }
    }

    /// Focus the next pane (circular order)
    pub fn focus_next(&mut self) {
        let pane_ids = self.all_pane_ids();
        if pane_ids.len() <= 1 {
            return;
        }

        if let Some(current_idx) = pane_ids.iter().position(|&id| id == self.focused_pane) {
            let next_idx = (current_idx + 1) % pane_ids.len();
            self.focused_pane = pane_ids[next_idx];
        }
    }

    /// Focus the previous pane (circular order)
    pub fn focus_prev(&mut self) {
        let pane_ids = self.all_pane_ids();
        if pane_ids.len() <= 1 {
            return;
        }

        if let Some(current_idx) = pane_ids.iter().position(|&id| id == self.focused_pane) {
            let prev_idx = if current_idx == 0 {
                pane_ids.len() - 1
            } else {
                current_idx - 1
            };
            self.focused_pane = pane_ids[prev_idx];
        }
    }

    /// Get the root node (for rendering)
    pub fn root(&self) -> &LayoutNode {
        &self.root
    }

    /// Get the root node (mutable, for rendering)
    #[allow(dead_code)]
    pub fn root_mut(&mut self) -> &mut LayoutNode {
        &mut self.root
    }

    /// Close the focused pane
    ///
    /// Returns Ok(()) if pane was closed, Err if this is the last pane or pane not found.
    ///
    /// This method is kept for potential future use with pane management keybindings.
    #[allow(dead_code)]
    pub fn close_focused(&mut self) -> Result<(), String> {
        let pane_count = self.pane_count();
        if pane_count <= 1 {
            return Err("Cannot close the last pane".to_string());
        }

        let focused_id = self.focused_pane;

        // Take the old root and replace with placeholder
        let old_root = std::mem::replace(
            &mut self.root,
            LayoutNode::Pane(Box::new(create_placeholder_pane())),
        );

        match close_pane_in_node(old_root, focused_id) {
            Ok(new_root) => {
                self.root = new_root;

                // Update focus to first available pane
                let pane_ids = self.all_pane_ids();
                if !pane_ids.is_empty() {
                    self.focused_pane = pane_ids[0];
                }

                Ok(())
            }
            Err(e) => {
                // On error, just set focus to first pane
                let pane_ids = self.all_pane_ids();
                if !pane_ids.is_empty() {
                    self.focused_pane = pane_ids[0];
                }
                Err(e)
            }
        }
    }

    /// Resize the focused pane
    ///
    /// # Arguments
    /// * `direction` - Direction to resize (Horizontal for left/right, Vertical for up/down)
    /// * `delta` - Amount to resize (positive to grow, negative to shrink)
    ///
    /// # Returns
    /// Ok(()) if resize was successful, Err if resize not possible
    #[allow(dead_code)]
    pub fn resize_focused(&mut self, direction: SplitDirection, delta: f32) -> Result<(), String> {
        let focused_id = self.focused_pane;

        // Take the old root and replace with placeholder
        let old_root = std::mem::replace(
            &mut self.root,
            LayoutNode::Pane(Box::new(create_placeholder_pane())),
        );

        match resize_pane_in_node(old_root, focused_id, direction, delta) {
            Ok(new_root) => {
                self.root = new_root;
                Ok(())
            }
            Err((old_root, err)) => {
                self.root = old_root;
                Err(err)
            }
        }
    }
}

/// Recursively close a pane in a node (standalone function to avoid borrow issues).
///
/// This function is used internally by [`LayoutTree::close_focused`].
#[allow(dead_code)]
fn close_pane_in_node(node: LayoutNode, pane_id: Uuid) -> Result<LayoutNode, String> {
    match node {
        LayoutNode::Pane(pane) if pane.id == pane_id => {
            // This pane should be removed - caller will handle it
            Err(format!("Pane {} found", pane_id))
        }
        LayoutNode::Pane(pane) => {
            // Not the target pane
            Ok(LayoutNode::Pane(pane))
        }
        LayoutNode::HorizontalSplit {
            mut children,
            mut ratios,
        } => {
            // Try to remove the pane from children
            let mut found_idx = None;
            for (i, child) in children.iter().enumerate() {
                if child.find_pane(pane_id).is_some() {
                    found_idx = Some(i);
                    break;
                }
            }

            if let Some(idx) = found_idx {
                // Remove the child
                children.remove(idx);
                ratios.remove(idx);

                // Normalize ratios
                let total: f32 = ratios.iter().sum();
                if total > 0.0 {
                    for ratio in &mut ratios {
                        *ratio /= total;
                    }
                }

                // If only one child left, collapse the split
                if children.len() == 1 {
                    return Ok(*children.remove(0));
                }

                Ok(LayoutNode::HorizontalSplit { children, ratios })
            } else {
                Ok(LayoutNode::HorizontalSplit { children, ratios })
            }
        }
        LayoutNode::VerticalSplit {
            mut children,
            mut ratios,
        } => {
            // Try to remove the pane from children
            let mut found_idx = None;
            for (i, child) in children.iter().enumerate() {
                if child.find_pane(pane_id).is_some() {
                    found_idx = Some(i);
                    break;
                }
            }

            if let Some(idx) = found_idx {
                // Remove the child
                children.remove(idx);
                ratios.remove(idx);

                // Normalize ratios
                let total: f32 = ratios.iter().sum();
                if total > 0.0 {
                    for ratio in &mut ratios {
                        *ratio /= total;
                    }
                }

                // If only one child left, collapse the split
                if children.len() == 1 {
                    return Ok(*children.remove(0));
                }

                Ok(LayoutNode::VerticalSplit { children, ratios })
            } else {
                Ok(LayoutNode::VerticalSplit { children, ratios })
            }
        }
    }
}

/// Recursively resize a pane in a node (standalone function to avoid borrow issues)
#[allow(dead_code)]
fn resize_pane_in_node(
    node: LayoutNode,
    pane_id: Uuid,
    direction: SplitDirection,
    delta: f32,
) -> Result<LayoutNode, (LayoutNode, String)> {
    match node {
        LayoutNode::Pane(pane) => {
            // Single pane cannot be resized
            Err((
                LayoutNode::Pane(pane),
                "Cannot resize: no adjacent pane".to_string(),
            ))
        }
        LayoutNode::HorizontalSplit {
            children,
            mut ratios,
        } => {
            // Only resize if direction matches
            if direction == SplitDirection::Horizontal {
                // Find which child contains the pane
                for i in 0..children.len() {
                    if children[i].find_pane(pane_id).is_some() {
                        // Found the pane, adjust ratios
                        // If pane is in child i, we adjust ratios[i] and ratios[i+1] or ratios[i-1]
                        // For simplicity, we adjust the pane's ratio up/down

                        if i < ratios.len() {
                            let new_ratio = (ratios[i] + delta).clamp(0.1, 0.9);
                            let diff = new_ratio - ratios[i];
                            ratios[i] = new_ratio;

                            // Adjust adjacent pane(s) to maintain total of 1.0
                            if i + 1 < ratios.len() {
                                ratios[i + 1] -= diff;
                                ratios[i + 1] = ratios[i + 1].clamp(0.1, 0.9);
                            } else if i > 0 {
                                ratios[i - 1] -= diff;
                                ratios[i - 1] = ratios[i - 1].clamp(0.1, 0.9);
                            }

                            // Normalize to ensure total is 1.0
                            let total: f32 = ratios.iter().sum();
                            if total > 0.0 {
                                for ratio in &mut ratios {
                                    *ratio /= total;
                                }
                            }
                        }

                        // Recurse into children to find the actual pane
                        let mut new_children = Vec::with_capacity(children.len());
                        for child in children.into_iter() {
                            let new_child = resize_pane_in_node(*child, pane_id, direction, delta);
                            match new_child {
                                Ok(c) => new_children.push(Box::new(c)),
                                Err((c, _)) => new_children.push(Box::new(c)),
                            }
                        }

                        return Ok(LayoutNode::HorizontalSplit {
                            children: new_children,
                            ratios,
                        });
                    }
                }
            }

            // Direction doesn't match or pane not found, just recurse
            let mut new_children = Vec::with_capacity(children.len());
            for child in children.into_iter() {
                let new_child = resize_pane_in_node(*child, pane_id, direction, delta);
                match new_child {
                    Ok(c) => new_children.push(Box::new(c)),
                    Err((c, _)) => new_children.push(Box::new(c)),
                }
            }

            Ok(LayoutNode::HorizontalSplit {
                children: new_children,
                ratios,
            })
        }
        LayoutNode::VerticalSplit {
            children,
            mut ratios,
        } => {
            // Only resize if direction matches
            if direction == SplitDirection::Vertical {
                // Find which child contains the pane
                for i in 0..children.len() {
                    if children[i].find_pane(pane_id).is_some() {
                        // Found the pane, adjust ratios
                        if i < ratios.len() {
                            let new_ratio = (ratios[i] + delta).clamp(0.1, 0.9);
                            let diff = new_ratio - ratios[i];
                            ratios[i] = new_ratio;

                            // Adjust adjacent pane(s)
                            if i + 1 < ratios.len() {
                                ratios[i + 1] -= diff;
                                ratios[i + 1] = ratios[i + 1].clamp(0.1, 0.9);
                            } else if i > 0 {
                                ratios[i - 1] -= diff;
                                ratios[i - 1] = ratios[i - 1].clamp(0.1, 0.9);
                            }

                            // Normalize
                            let total: f32 = ratios.iter().sum();
                            if total > 0.0 {
                                for ratio in &mut ratios {
                                    *ratio /= total;
                                }
                            }
                        }

                        // Recurse into children
                        let mut new_children = Vec::with_capacity(children.len());
                        for child in children.into_iter() {
                            let new_child = resize_pane_in_node(*child, pane_id, direction, delta);
                            match new_child {
                                Ok(c) => new_children.push(Box::new(c)),
                                Err((c, _)) => new_children.push(Box::new(c)),
                            }
                        }

                        return Ok(LayoutNode::VerticalSplit {
                            children: new_children,
                            ratios,
                        });
                    }
                }
            }

            // Direction doesn't match or pane not found, just recurse
            let mut new_children = Vec::with_capacity(children.len());
            for child in children.into_iter() {
                let new_child = resize_pane_in_node(*child, pane_id, direction, delta);
                match new_child {
                    Ok(c) => new_children.push(Box::new(c)),
                    Err((c, _)) => new_children.push(Box::new(c)),
                }
            }

            Ok(LayoutNode::VerticalSplit {
                children: new_children,
                ratios,
            })
        }
    }
}

/// Cached PTY for placeholder panes (avoids spawning multiple times on Windows)
static PLACEHOLDER_PTY: OnceLock<Arc<Mutex<PtySession>>> = OnceLock::new();

/// Create a placeholder pane (used internally for tree manipulation).
///
/// Uses a cached PTY to avoid stack overflow on Windows from spawning
/// multiple PTY sessions during tree restructuring.
///
/// # Panics
///
/// Panics if PTY spawning fails. This is acceptable because:
/// 1. The placeholder pane is only used during internal tree operations
/// 2. A PTY failure at this point indicates a fundamental system problem
/// 3. The PTY is cached and only spawned once via `OnceLock`
fn create_placeholder_pane() -> Pane {
    use std::sync::Arc;
    let pty = PLACEHOLDER_PTY.get_or_init(|| {
        use crate::terminal::pty::PtyConfig;
        // SAFETY: This only runs once due to OnceLock. Panicking here is
        // appropriate as PTY spawning failure indicates a system-level issue.
        let pty_session = PtySession::spawn(PtyConfig::default())
            .expect("Failed to spawn placeholder PTY - system may not support PTY operations");
        Arc::new(Mutex::new(pty_session))
    });

    Pane {
        id: uuid::Uuid::new_v4(),
        title: "Placeholder".to_string(),
        pty: Arc::clone(pty),
        grid: TerminalGrid::with_size(1, 1),
        parser: TerminalParser::new(),
        bounds: Rect::new(0, 0, 1, 1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::pty::PtyConfig;

    fn create_test_pane() -> Pane {
        let pty = PtySession::spawn(PtyConfig::default()).unwrap();
        Pane::new(pty, 80, 24, Rect::new(0, 0, 800, 600))
    }

    #[test]
    fn test_rect_creation() {
        let rect = Rect::new(10, 20, 100, 200);
        assert_eq!(rect.x, 10);
        assert_eq!(rect.y, 20);
        assert_eq!(rect.width, 100);
        assert_eq!(rect.height, 200);
    }

    #[test]
    fn test_rect_contains() {
        let rect = Rect::new(10, 20, 100, 200);
        assert!(rect.contains(50, 100)); // Inside
        assert!(rect.contains(10, 20)); // On edge
        assert!(!rect.contains(200, 100)); // Outside X
        assert!(!rect.contains(50, 300)); // Outside Y
    }

    #[test]
    fn test_pane_creation() {
        let pane = create_test_pane();
        assert!(!pane.id.is_nil());
        assert_eq!(pane.title, "Terminal");
    }

    #[test]
    fn test_pane_terminal_size() {
        let pane = create_test_pane();
        let (cols, rows) = pane.terminal_size(10, 20);
        assert_eq!(cols, 80);
        assert_eq!(rows, 30);
    }

    #[test]
    fn test_layout_tree_creation() {
        let pane = create_test_pane();
        let pane_id = pane.id;
        let tree = LayoutTree::new(pane);

        assert_eq!(tree.focused_pane_id(), pane_id);
        assert_eq!(tree.pane_count(), 1);
    }

    #[test]
    fn test_layout_tree_focused_pane() {
        let pane = create_test_pane();
        let pane_id = pane.id;
        let tree = LayoutTree::new(pane);

        let focused = tree.focused_pane();
        assert!(focused.is_some());
        assert_eq!(focused.unwrap().id, pane_id);
    }

    #[test]
    fn test_layout_node_pane_count() {
        let pane1 = create_test_pane();
        let pane2 = create_test_pane();

        let node = LayoutNode::HorizontalSplit {
            children: vec![
                Box::new(LayoutNode::Pane(Box::new(pane1))),
                Box::new(LayoutNode::Pane(Box::new(pane2))),
            ],
            ratios: vec![0.5, 0.5],
        };

        assert_eq!(node.pane_count(), 2);
    }

    #[test]
    fn test_layout_tree_split_horizontal() {
        let pane1 = create_test_pane();
        let pane1_id = pane1.id;
        let pane2 = create_test_pane();
        let pane2_id = pane2.id;

        let mut tree = LayoutTree::new(pane1);
        tree.split_focused(SplitDirection::Horizontal, pane2)
            .unwrap();

        assert_eq!(tree.pane_count(), 2);
        assert_eq!(tree.focused_pane_id(), pane2_id);
        assert!(tree.get_pane(pane1_id).is_some());
        assert!(tree.get_pane(pane2_id).is_some());
    }

    #[test]
    fn test_layout_tree_split_vertical() {
        let pane1 = create_test_pane();
        let pane2 = create_test_pane();

        let mut tree = LayoutTree::new(pane1);
        tree.split_focused(SplitDirection::Vertical, pane2).unwrap();

        assert_eq!(tree.pane_count(), 2);
        assert_eq!(tree.root().pane_count(), 2);
    }

    #[test]
    fn test_layout_tree_focus_navigation() {
        let pane1 = create_test_pane();
        let pane1_id = pane1.id;
        let pane2 = create_test_pane();
        let pane2_id = pane2.id;
        let pane3 = create_test_pane();
        let pane3_id = pane3.id;

        let mut tree = LayoutTree::new(pane1);
        tree.split_focused(SplitDirection::Horizontal, pane2)
            .unwrap();
        tree.set_focus(pane1_id);
        tree.split_focused(SplitDirection::Vertical, pane3).unwrap();

        // Test focus_next
        tree.set_focus(pane1_id);
        tree.focus_next();
        assert_eq!(tree.focused_pane_id(), pane3_id);

        tree.focus_next();
        assert_eq!(tree.focused_pane_id(), pane2_id);

        tree.focus_next();
        assert_eq!(tree.focused_pane_id(), pane1_id);

        // Test focus_prev
        tree.focus_prev();
        assert_eq!(tree.focused_pane_id(), pane2_id);
    }

    #[test]
    fn test_layout_tree_calculate_layout() {
        let pane1 = create_test_pane();
        let pane2 = create_test_pane();

        let mut tree = LayoutTree::new(pane1);
        tree.split_focused(SplitDirection::Horizontal, pane2)
            .unwrap();

        let total_bounds = Rect::new(0, 0, 1000, 800);
        tree.calculate_layout(total_bounds);

        // Check that panes have bounds set
        let pane_ids = tree.all_pane_ids();
        for id in pane_ids {
            let pane = tree.get_pane(id).unwrap();
            assert!(pane.bounds.width > 0);
            assert!(pane.bounds.height > 0);
        }
    }

    #[test]
    fn test_layout_tree_max_panes() {
        let pane1 = create_test_pane();
        let mut tree = LayoutTree::new(pane1);

        // Add 7 more panes (total 8)
        for _ in 0..7 {
            let new_pane = create_test_pane();
            tree.split_focused(SplitDirection::Horizontal, new_pane)
                .unwrap();
        }

        assert_eq!(tree.pane_count(), 8);

        // Try to add 9th pane (should fail)
        let pane9 = create_test_pane();
        let result = tree.split_focused(SplitDirection::Horizontal, pane9);
        assert!(result.is_err());
        assert_eq!(tree.pane_count(), 8);
    }

    #[test]
    fn test_layout_node_find_pane() {
        let pane1 = create_test_pane();
        let pane1_id = pane1.id;
        let pane2 = create_test_pane();
        let pane2_id = pane2.id;

        let node = LayoutNode::HorizontalSplit {
            children: vec![
                Box::new(LayoutNode::Pane(Box::new(pane1))),
                Box::new(LayoutNode::Pane(Box::new(pane2))),
            ],
            ratios: vec![0.5, 0.5],
        };

        assert!(node.find_pane(pane1_id).is_some());
        assert!(node.find_pane(pane2_id).is_some());
        assert!(node.find_pane(Uuid::new_v4()).is_none());
    }

    #[test]
    fn test_layout_node_collect_pane_ids() {
        let pane1 = create_test_pane();
        let pane1_id = pane1.id;
        let pane2 = create_test_pane();
        let pane2_id = pane2.id;

        let node = LayoutNode::HorizontalSplit {
            children: vec![
                Box::new(LayoutNode::Pane(Box::new(pane1))),
                Box::new(LayoutNode::Pane(Box::new(pane2))),
            ],
            ratios: vec![0.5, 0.5],
        };

        let ids = node.collect_pane_ids();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&pane1_id));
        assert!(ids.contains(&pane2_id));
    }
}
