//! Moving panes from one tab's tree to another's.
//!
//! Apart from [`crate::session_tree`], which rearranges one tree: these take
//! two, and neither touches a process — a leaf names its tmux window, not the
//! tab it is drawn in, so moving it between tabs is only a change of shape.

use crate::session::{LayoutNode, SplitDirection};

impl LayoutNode {
    /// This tree with `other` beside it, as the two sides of one new split.
    ///
    /// What was here stays first, so joining a tab into this one leaves what
    /// the person was looking at where it was.
    pub fn joined(
        &self,
        other: LayoutNode,
        direction: SplitDirection,
        split_id: impl Into<String>,
    ) -> LayoutNode {
        Self::Split {
            id: split_id.into(),
            direction,
            ratio: 0.5,
            first: Box::new(self.clone()),
            second: Box::new(other),
        }
    }

    /// Takes `leaf_id` out: what is left, and the leaf on its own.
    ///
    /// `None` when the leaf is not here, and for the last leaf, for the same
    /// reason [`LayoutNode::close_leaf`] refuses it: a tab with no pane is not
    /// a layout.
    pub fn detached(&self, leaf_id: &str) -> Option<(LayoutNode, LayoutNode)> {
        let leaf = self.leaf_node(leaf_id)?.clone();
        let rest = self.close_leaf(leaf_id)?;
        Some((rest, leaf))
    }

    fn leaf_node(&self, leaf_id: &str) -> Option<&LayoutNode> {
        match self {
            Self::Leaf { id, .. } => (id == leaf_id).then_some(self),
            Self::Split { first, second, .. } => first
                .leaf_node(leaf_id)
                .or_else(|| second.leaf_node(leaf_id)),
        }
    }
}

#[cfg(test)]
#[path = "session_regroup_tests.rs"]
mod tests;
