//! The session layout contract.
//!
//! The tree is the Orca answer: nested splits, one process per leaf. The axis
//! is the project, not the worktree. Leaves are terminals today; a later leaf
//! kind does not need a tmux window.

use serde::{Deserialize, Serialize};
use specta::Type;

/// What a leaf shows. Only `terminal` in this slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum PaneKind {
    Terminal,
}

/// Who is running in the leaf. `none` is a plain shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum AgentPresence {
    None,
}

/// Orca's names: horizontal is left/right, vertical is top/bottom.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum LayoutNode {
    Leaf {
        id: String,
        /// `session:window` on our private tmux server.
        #[serde(rename = "tmuxTarget")]
        tmux_target: String,
        kind: PaneKind,
        agent: AgentPresence,
    },
    Split {
        direction: SplitDirection,
        ratio: f64,
        first: Box<LayoutNode>,
        second: Box<LayoutNode>,
    },
}

/// Response of `session.layout` / `session.ensure` / `session.split`.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SessionLayout {
    pub project_id: String,
    pub focused_id: String,
    pub tree: LayoutNode,
}

impl LayoutNode {
    pub fn leaf(id: impl Into<String>, tmux_target: impl Into<String>) -> Self {
        Self::Leaf {
            id: id.into(),
            tmux_target: tmux_target.into(),
            kind: PaneKind::Terminal,
            agent: AgentPresence::None,
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
                direction,
                ratio: 0.5,
                first: Box::new(self.clone()),
                second: Box::new(new_leaf),
            }),
            Self::Leaf { .. } => None,
            Self::Split {
                direction: dir,
                ratio,
                first,
                second,
            } => {
                if let Some(next) = first.split_leaf(leaf_id, direction, new_leaf.clone()) {
                    return Some(Self::Split {
                        direction: *dir,
                        ratio: *ratio,
                        first: Box::new(next),
                        second: second.clone(),
                    });
                }
                let next = second.split_leaf(leaf_id, direction, new_leaf)?;
                Some(Self::Split {
                    direction: *dir,
                    ratio: *ratio,
                    first: first.clone(),
                    second: Box::new(next),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_tree_is_one_leaf() {
        let tree = LayoutNode::leaf("leaf_a", "sess:leaf_a");
        assert_eq!(tree.leaves(), vec![("leaf_a", "sess:leaf_a")]);
        assert_eq!(tree.first_leaf_id(), "leaf_a");
    }

    #[test]
    fn split_leaf_puts_the_new_pane_on_the_second_side() {
        let tree = LayoutNode::leaf("leaf_a", "sess:leaf_a");
        let split = tree
            .split_leaf(
                "leaf_a",
                SplitDirection::Horizontal,
                LayoutNode::leaf("leaf_b", "sess:leaf_b"),
            )
            .expect("split");

        match &split {
            LayoutNode::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                assert_eq!(*direction, SplitDirection::Horizontal);
                assert_eq!(*ratio, 0.5);
                assert_eq!(first.first_leaf_id(), "leaf_a");
                assert_eq!(second.first_leaf_id(), "leaf_b");
            }
            LayoutNode::Leaf { .. } => panic!("expected a split"),
        }

        assert_eq!(split.leaves().len(), 2);
    }

    #[test]
    fn nested_split_finds_the_inner_leaf() {
        let tree = LayoutNode::leaf("a", "s:a")
            .split_leaf(
                "a",
                SplitDirection::Horizontal,
                LayoutNode::leaf("b", "s:b"),
            )
            .unwrap()
            .split_leaf("b", SplitDirection::Vertical, LayoutNode::leaf("c", "s:c"))
            .unwrap();

        assert_eq!(
            tree.leaves()
                .into_iter()
                .map(|(id, _)| id)
                .collect::<Vec<_>>(),
            vec!["a", "b", "c"]
        );
    }

    #[test]
    fn json_roundtrip_keeps_the_tag() {
        let tree = LayoutNode::leaf("leaf_a", "sess:leaf_a");
        let json = serde_json::to_value(&tree).expect("serialize");
        assert_eq!(json["type"], "leaf");
        assert_eq!(json["tmuxTarget"], "sess:leaf_a");
        let back: LayoutNode = serde_json::from_value(json).expect("deserialize");
        assert_eq!(back.first_leaf_id(), "leaf_a");
    }
}
