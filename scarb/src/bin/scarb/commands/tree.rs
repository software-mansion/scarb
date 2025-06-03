use crate::args::TreeCommandArgs;
use anyhow::Result;
use scarb::core::{Config, Package, PackageId, TargetKind};
use scarb::ops;
use scarb_ui::Message;
use serde::{Serialize, Serializer};
use std::collections::HashSet;
use std::fmt::Write;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(args: TreeCommandArgs, config: &Config) -> Result<()> {
    let ws = ops::read_workspace(config.manifest_path(), config)?;
    let packages = args
        .packages_filter
        .match_many(&ws)?
        .into_iter()
        .map(|p| p.id)
        .collect::<Vec<_>>();

    let resolve = ops::resolve_workspace(&ws)?;

    let forest = build(packages, &resolve, &args)?;

    config.ui().force_print(forest);

    Ok(())
}

fn build(
    packages: Vec<PackageId>,
    resolve: &ops::WorkspaceResolve,
    args: &TreeCommandArgs,
) -> Result<Tree> {
    struct Visitor<'a> {
        main_package_id: PackageId,
        // Tracks visited packages to avoid duplicates among separate branches.
        visited: &'a mut HashSet<PackageId>,
        // Tracks visited packages to avoid cycles within a branch.
        stack: Vec<PackageId>,
        resolve: &'a ops::WorkspaceResolve,
        args: &'a TreeCommandArgs,
    }

    impl Visitor<'_> {
        fn visit(
            &mut self,
            package_id: PackageId,
            tree: &mut Tree,
            depth: usize,
        ) -> Result<(), BuildError> {
            self.stack.push(package_id);
            let result = self.visit_inner(package_id, tree, depth);
            self.stack.pop();
            result
        }

        fn visit_inner(
            &mut self,
            package_id: PackageId,
            tree: &mut Tree,
            depth: usize,
        ) -> Result<(), BuildError> {
            // Only show the core package if explicitly requested.
            if package_id.is_core() && !self.args.core {
                return Err(BuildError::Pruned);
            }

            // Skip if this package should be pruned.
            if self.args.prune.contains(&package_id.name) {
                return Err(BuildError::Pruned);
            }

            // Check if we've reached the maximum depth.
            // For max_depth=0, we want to list roots only, hence we use strict equality.
            if let Some(max_depth) = self.args.depth {
                if depth > max_depth {
                    tree.max_depth_reached = true;
                    return Err(BuildError::MaxDepthReached);
                }
            }

            tree.package = Some(package_id);

            // Recursively visit dependencies if we haven't visited this package before
            // or if no_dedupe is true, unless there is a cycle.
            let already_visited = self.visited.contains(&package_id) && !self.args.no_dedupe;
            let is_cycle = self.stack.iter().filter(|p| **p == package_id).count() > 1;
            tree.already_visited_duplicate = already_visited || is_cycle;
            if !tree.already_visited_duplicate {
                self.visited.insert(package_id);

                // Collect normal dependencies.
                let normal_deps = self.resolve.package_dependencies(
                    package_id,
                    &TargetKind::LIB,
                    self.main_package_id,
                )?;

                self.visit_deps(&normal_deps, tree, depth)?;

                // Collect dev dependencies and put them in a grouping branch.
                let mut dev_deps = self.resolve.package_dependencies(
                    package_id,
                    &TargetKind::TEST,
                    self.main_package_id,
                )?;

                let dev_tree = tree.branch();
                dev_tree.group = Some("dev-dependencies");

                // We're only interested in packages that are ONLY dev dependencies here.
                let normal_deps_ids: HashSet<PackageId> =
                    normal_deps.iter().map(|p| p.id).collect();
                dev_deps.retain(|pkg| !normal_deps_ids.contains(&pkg.id));

                self.visit_deps(&dev_deps, dev_tree, depth)?;

                // Roll back the branch if no dev dependencies were found.
                // We use rollbacks instead of checking `dev_deps` for emptiness to account pruning.
                if dev_tree.branches.is_empty() {
                    tree.rollback();
                }
            }

            Ok(())
        }

        fn visit_deps(
            &mut self,
            deps: &[Package],
            tree: &mut Tree,
            depth: usize,
        ) -> Result<(), BuildError> {
            for dep in deps {
                let branch = tree.branch();
                match self.visit(dep.id, branch, depth + 1) {
                    Ok(()) => {}
                    Err(BuildError::Pruned) => {
                        tree.rollback();
                    }
                    Err(BuildError::MaxDepthReached) => {
                        // Avoid emitting multiple `max_depth_reached` stubs.
                        break;
                    }
                    err @ Err(BuildError::Anyhow(_)) => {
                        return err;
                    }
                }
            }
            Ok(())
        }
    }

    let mut forest = Tree::default();
    let mut visited = Default::default();
    for package_id in packages {
        let mut visitor = Visitor {
            main_package_id: package_id,
            visited: &mut visited,
            stack: Default::default(),
            resolve,
            args,
        };

        if let Err(BuildError::Anyhow(err)) = visitor.visit(package_id, forest.branch(), 0) {
            return Err(err);
        }
    }
    Ok(forest)
}

enum BuildError {
    Pruned,
    MaxDepthReached,
    Anyhow(anyhow::Error),
}

impl From<anyhow::Error> for BuildError {
    fn from(err: anyhow::Error) -> Self {
        Self::Anyhow(err)
    }
}

#[derive(Default, Serialize)]
struct Tree {
    #[serde(skip_serializing_if = "Option::is_none")]
    package: Option<PackageId>,

    #[serde(skip_serializing_if = "Option::is_none")]
    group: Option<&'static str>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    branches: Vec<Tree>,

    #[serde(skip_serializing_if = "is_false")]
    max_depth_reached: bool,

    #[serde(skip_serializing_if = "is_false")]
    already_visited_duplicate: bool,
}

impl Tree {
    /// Adds a new branch to the dependency tree and returns a mutable reference to it.
    /// The new branch is initialised as an empty [`Tree`] and added to the branches list.
    /// Returns a mutable reference to the newly created subtree.
    fn branch(&mut self) -> &mut Tree {
        self.branches.push(Tree::default());
        self.branches.last_mut().unwrap()
    }

    /// Removes the most recently added branch from the dependency tree.
    /// This effectively "rolls back" the addition of the last branch.
    /// Rust borrow checker prevents using the rolled back branch after this call.
    fn rollback(&mut self) {
        self.branches.pop();
    }
}

impl Message for Tree {
    fn text(self) -> String {
        // FAQ: What the heck is that `is_last_stack` thingy?
        // It is a stack of booleans that keeps track of whether the current branch is the last
        // branch on its depth. See the output tested in the `beautiful_tree_formatting` test.

        fn visit(tree: &Tree, is_last_stack: &mut Vec<bool>, out: &mut String) {
            // This recursive is_last contraption will probably seem evil to my beloved readers,
            // but it allowed me to minimise bookkeeping quite neatly, which is a net benefit.
            for t in is_last(is_last_stack.iter().copied()) {
                // (is this branch last on its depth?, is this a leaf level?)
                out.push_str(match t {
                    (true, true) => "└── ",
                    (false, true) => "├── ",
                    (true, false) => "    ",
                    (false, false) => "│   ",
                })
            }

            if let Some(package) = &tree.package {
                write!(out, "{package}").unwrap();
            }

            if let Some(group) = tree.group {
                write!(out, "[{group}]").unwrap();
            }

            if tree.already_visited_duplicate {
                out.push_str(" (*)");
            }

            if tree.max_depth_reached {
                out.push_str("...");
            }

            out.push('\n');

            for (branch, is_last) in is_last(&tree.branches) {
                is_last_stack.push(is_last);
                visit(branch, is_last_stack, out);
                is_last_stack.pop();
            }
        }

        let mut out = String::new();
        let mut is_last_stack = Vec::<bool>::new();
        for tree in &self.branches {
            visit(tree, &mut is_last_stack, &mut out);
            out.push('\n');
        }

        // Trim any trailing whitespace in-place.
        out.truncate(out.trim_end().len());

        out
    }

    fn structured<S: Serializer>(self, ser: S) -> Result<S::Ok, S::Error> {
        self.branches.serialize(ser)
    }
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// Iterator adapter for tracking whether an item is the last element.
///
/// This struct wraps an iterator and provides the ability to check if each element is the last one
/// in the sequence. For each item yielded, it pairs it with a boolean indicating whether that item
/// is last (i.e. no more items follow it).
struct IsLast<I: Iterator> {
    iter: std::iter::Peekable<I>,
}

/// Creates a new `IsLast` iterator adapter.
///
/// This converts any iterator into one that yields tuples of `(item, is_last)`, where `is_last`
/// indicates if this is the final element.
fn is_last<I: IntoIterator>(iter: I) -> IsLast<I::IntoIter> {
    IsLast {
        iter: iter.into_iter().peekable(),
    }
}

impl<I: Iterator> Iterator for IsLast<I> {
    type Item = (I::Item, bool);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|item| {
            let is_last = self.iter.peek().is_none();
            (item, is_last)
        })
    }
}
