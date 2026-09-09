//! What a drag does to the pane tree.
//!
//! Every operation returns a **new** tree rather than mutating one. A split
//! that half-applied would leave a layout nothing can draw, and the caller
//! persists the result in one write — so either the whole move landed or none
//! of it did.

use crate::session::{AgentPresence, LayoutNode, PaneKind, SplitDirection};

/// How far a boundary may be dragged.
///
/// A pane at ratio 0 is a pane that exists, holds a running process, and
/// cannot be seen or grabbed to bring back. The clamp is what stops a drag
/// from making a process unreachable.
const NARROWEST: f64 = 0.05;
const WIDEST: f64 = 0.95;

impl LayoutNode {
    pub fn leaf(id: impl Into<String>, tmux_target: impl Into<String>) -> Self {
        Self::Leaf {
            id: id.into(),
            tmux_target: tmux_target.into(),
            kind: PaneKind::Terminal,
            agent: AgentPresence::None,
            title: String::new(),
        }
    }

    pub fn leaves(&self) -> Vec<(&str, &str)> {
        let mut out = Vec::new();
        self.collect_leaves(&mut out);
        out
    }

    fn collect_leaves<'a>(&'a self, out: &mut Vec<(&'a str, &'a str)>) {
        match self {
            Self::Leaf {
                id, tmux_target, ..
            } => out.push((id, tmux_target)),
            Self::Split { first, second, .. } => {
                first.collect_leaves(out);
                second.collect_leaves(out);
            }
        }
    }

    pub fn first_leaf_id(&self) -> &str {
        match self {
            Self::Leaf { id, .. } => id,
            Self::Split { first, .. } => first.first_leaf_id(),
        }
    }

    pub fn contains_leaf(&self, leaf_id: &str) -> bool {
        self.leaves().iter().any(|(id, _)| *id == leaf_id)
    }

    /// Replaces `leaf_id` with a split: existing leaf on `first`, `new_leaf` on `second`.
    pub fn split_leaf(
        &self,
        leaf_id: &str,
        direction: SplitDirection,
        new_leaf: LayoutNode,
    ) -> Option<LayoutNode> {
        match self {
            Self::Leaf { id, .. } if id == leaf_id => Some(Self::Split {
                id: String::new(),
                direction,
                ratio: 0.5,
                first: Box::new(self.clone()),
                second: Box::new(new_leaf),
            }),
            Self::Leaf { .. } => None,
            Self::Split {
                id,
                direction: dir,
                ratio,
                first,
                second,
            } => {
                if let Some(next) = first.split_leaf(leaf_id, direction, new_leaf.clone()) {
                    return Some(Self::Split {
                        id: id.clone(),
                        direction: *dir,
                        ratio: *ratio,
                        first: Box::new(next),
                        second: second.clone(),
                    });
                }
                let next = second.split_leaf(leaf_id, direction, new_leaf)?;
                Some(Self::Split {
                    id: id.clone(),
                    direction: *dir,
                    ratio: *ratio,
                    first: first.clone(),
                    second: Box::new(next),
                })
            }
        }
    }

    /// Removes `leaf_id`, and its split along with it.
    ///
    /// Closing one side of a split leaves the other side where the split was,
    /// rather than a split with an empty half. `None` when the leaf is not
    /// here, and `None` for the last leaf of the tree: a session with no pane
    /// is not a layout, and the caller says so in words the screen can show.
    pub fn close_leaf(&self, leaf_id: &str) -> Option<LayoutNode> {
        match self {
            // The whole tree is this leaf. There is nothing to be left with.
            Self::Leaf { .. } => None,
            Self::Split { first, second, .. } => {
                if first.is_leaf(leaf_id) {
                    return Some((**second).clone());
                }
                if second.is_leaf(leaf_id) {
                    return Some((**first).clone());
                }
                self.rebuilt_with(
                    first.close_leaf(leaf_id).map(Box::new),
                    second.close_leaf(leaf_id).map(Box::new),
                )
            }
        }
    }

    /// The name the person gave this pane. Empty clears it back to the
    /// program's own title.
    pub fn rename_leaf(&self, leaf_id: &str, name: &str) -> Option<LayoutNode> {
        match self {
            Self::Leaf {
                id,
                tmux_target,
                kind,
                agent,
                ..
            } if id == leaf_id => Some(Self::Leaf {
                id: id.clone(),
                tmux_target: tmux_target.clone(),
                kind: *kind,
                agent: *agent,
                title: name.trim().to_owned(),
            }),
            Self::Leaf { .. } => None,
            Self::Split { first, second, .. } => self.rebuilt_with(
                first.rename_leaf(leaf_id, name).map(Box::new),
                second.rename_leaf(leaf_id, name).map(Box::new),
            ),
        }
    }

    /// Where a boundary was dragged to, clamped so neither side vanishes.
    pub fn set_ratio(&self, split_id: &str, ratio: f64) -> Option<LayoutNode> {
        let Self::Split {
            id,
            direction,
            ratio: _,
            first,
            second,
        } = self
        else {
            return None;
        };

        if id == split_id {
            // NaN compares false against everything, so it survives a pair of
            // `min`/`max` calls untouched and would be persisted. Refused here
            // instead: a boundary at "not a number" is not a drag anyone made.
            if !ratio.is_finite() {
                return None;
            }
            return Some(Self::Split {
                id: id.clone(),
                direction: *direction,
                ratio: ratio.clamp(NARROWEST, WIDEST),
                first: first.clone(),
                second: second.clone(),
            });
        }

        self.rebuilt_with(
            first.set_ratio(split_id, ratio).map(Box::new),
            second.set_ratio(split_id, ratio).map(Box::new),
        )
    }

    /// Gives every unnamed split an id, and says whether it had to.
    ///
    /// Splits gained ids after trees were already on disk. Filling them on
    /// read costs one pass and no migration; the caller persists when this
    /// returns true, so the next read pays nothing.
    pub fn name_the_splits(&mut self, next_id: &mut impl FnMut() -> String) -> bool {
        let Self::Split {
            id, first, second, ..
        } = self
        else {
            return false;
        };
        let mut named = false;
        if id.is_empty() {
            *id = next_id();
            named = true;
        }
        named |= first.name_the_splits(next_id);
        named |= second.name_the_splits(next_id);
        named
    }

    /// Every split id, outermost first. What the screen draws a handle for.
    pub fn split_ids(&self) -> Vec<&str> {
        let mut out = Vec::new();
        self.collect_split_ids(&mut out);
        out
    }

    fn collect_split_ids<'a>(&'a self, out: &mut Vec<&'a str>) {
        if let Self::Split {
            id, first, second, ..
        } = self
        {
            out.push(id);
            first.collect_split_ids(out);
            second.collect_split_ids(out);
        }
    }

    fn is_leaf(&self, leaf_id: &str) -> bool {
        matches!(self, Self::Leaf { id, .. } if id == leaf_id)
    }

    /// Rebuilds this split when exactly one side changed.
    ///
    /// `None` from both sides means the target was in neither, which is the
    /// caller's "not here". Written once because all three recursive
    /// operations end the same way, and three copies of it drifted apart in
    /// the version of this file that had them.
    fn rebuilt_with(
        &self,
        changed_first: Option<Box<LayoutNode>>,
        changed_second: Option<Box<LayoutNode>>,
    ) -> Option<LayoutNode> {
        let Self::Split {
            id,
            direction,
            ratio,
            first,
            second,
        } = self
        else {
            return None;
        };
        if changed_first.is_none() && changed_second.is_none() {
            return None;
        }
        Some(Self::Split {
            id: id.clone(),
            direction: *direction,
            ratio: *ratio,
            first: changed_first.unwrap_or_else(|| first.clone()),
            second: changed_second.unwrap_or_else(|| second.clone()),
        })
    }
}

#[cfg(test)]
#[path = "session_tree_tests.rs"]
mod tests;
