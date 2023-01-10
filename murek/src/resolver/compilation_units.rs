use std::collections::{HashMap, HashSet};
use std::hash::Hash;

pub trait Node {
    type Id: Copy + Eq + Hash;
    fn id(&self) -> Self::Id;
    fn direct_dependencies(&self) -> &HashSet<Self::Id>;
}

/// Collects all [`Node::Id`]s needed to include in a compilation unit for each input [`Node`],
/// according to package resolution rules.
///
/// For examples, consult unit tests of this function.
///
/// ## Invariants
///
/// 1. This function assumes that all reachable nodes are present in input collection.
///    Upon spotting an unknown node, a panic is raised.
/// 2. For each [`Node`], its ID will always be present in its collected compilation unit.
///
/// ## Algorithm
///
/// The algorithm here is basically tree flattening.
/// For each [`Node`] in `nodes`, this node's dependency tree is recursively searched depth-first,
/// and all spotted nodes are stored in a set.
/// Upon entire tree is visited, the stored set is saved as this node's compilation unit.
#[tracing::instrument(level = "trace", skip_all)]
pub fn collect<T: Node>(nodes: impl Iterator<Item = T>) -> HashMap<T::Id, HashSet<T::Id>> {
    type Output<T> = HashMap<<T as Node>::Id, HashSet<<T as Node>::Id>>;
    type Nodes<T> = HashMap<<T as Node>::Id, T>;
    type CurrentSet<T> = HashSet<<T as Node>::Id>;

    fn visit_all<T: Node>(nodes: &Nodes<T>) -> Output<T> {
        let mut current_set: CurrentSet<T> = Default::default();
        nodes
            .keys()
            .map(|&id| {
                current_set.clear();
                visit(id, nodes, &mut current_set);
                (id, current_set.clone())
            })
            .collect()
    }

    fn visit<T: Node>(id: T::Id, nodes: &Nodes<T>, current_set: &mut CurrentSet<T>) {
        assert!(nodes.contains_key(&id));

        if current_set.contains(&id) {
            return;
        }

        current_set.insert(id);

        for dep in nodes[&id].direct_dependencies() {
            visit(*dep, nodes, current_set);
        }
    }

    let nodes: Nodes<T> = nodes.map(|node| (node.id(), node)).collect();
    visit_all(&nodes)
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;
    use std::collections::HashSet;
    use std::{assert_eq, fmt, vec, write};

    use itertools::Itertools;

    use super::Node;

    #[derive(Eq, PartialEq)]
    struct TestNode {
        id: u8,
        deps: HashSet<u8>,
    }

    impl TestNode {
        fn new<const N: usize>(id: u8, deps: [u8; N]) -> Self {
            Self {
                id,
                deps: deps.into_iter().collect(),
            }
        }
    }

    impl Node for &TestNode {
        type Id = u8;

        fn id(&self) -> Self::Id {
            self.id
        }

        fn direct_dependencies(&self) -> &HashSet<Self::Id> {
            &self.deps
        }
    }

    impl PartialOrd<Self> for TestNode {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            self.id.partial_cmp(&other.id)
        }
    }

    impl Ord for TestNode {
        fn cmp(&self, other: &Self) -> Ordering {
            self.id.cmp(&other.id)
        }
    }

    impl From<&TestNode> for (u8, HashSet<u8>) {
        fn from(node: &TestNode) -> Self {
            (node.id, node.deps.clone())
        }
    }

    impl From<(u8, HashSet<u8>)> for TestNode {
        fn from((id, deps): (u8, HashSet<u8>)) -> Self {
            TestNode { id, deps }
        }
    }

    impl fmt::Debug for TestNode {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            if self.deps.is_empty() {
                write!(f, "TestNode({})", self.id)
            } else {
                write!(f, "TestNode({}:", self.id)?;
                for dep in self.deps.iter().sorted() {
                    write!(f, " {dep}")?;
                }
                write!(f, ")")?;
                Ok(())
            }
        }
    }

    #[test]
    fn collect() {
        let input = &[
            TestNode::new(0, []),
            TestNode::new(1, [0]),
            TestNode::new(2, [1]),
            TestNode::new(3, [2]),
            TestNode::new(4, []),
            TestNode::new(5, []),
            TestNode::new(6, []),
            TestNode::new(7, [5, 6]),
            TestNode::new(8, [0, 4, 7]),
            TestNode::new(9, [3, 8]),
            TestNode::new(10, []),
        ];

        let expected = vec![
            TestNode::new(0, [0]),
            TestNode::new(1, [0, 1]),
            TestNode::new(2, [0, 1, 2]),
            TestNode::new(3, [0, 1, 2, 3]),
            TestNode::new(4, [4]),
            TestNode::new(5, [5]),
            TestNode::new(6, [6]),
            TestNode::new(7, [5, 6, 7]),
            TestNode::new(8, [0, 4, 5, 6, 7, 8]),
            TestNode::new(9, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]),
            TestNode::new(10, [10]),
        ];

        let actual: Vec<TestNode> = super::collect(input.iter())
            .into_iter()
            .map(Into::into)
            .sorted()
            .collect();

        assert_eq!(expected, actual);
    }
}
