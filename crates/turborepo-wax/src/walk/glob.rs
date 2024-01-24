use std::{
    fs::{FileType, Metadata},
    path::{Component, Path, PathBuf},
};

use itertools::Itertools;
use regex::Regex;

use crate::{
    capture::MatchedText,
    encode::CompileError,
    token::{self, Token, TokenTree},
    walk::{
        filter::{HierarchicalIterator, Separation},
        Entry, EntryResidue, FileIterator, JoinAndGetDepth, TreeEntry, WalkBehavior, WalkError,
        WalkTree,
    },
    BuildError, CandidatePath, Combine, Glob,
};

/// APIs for matching globs against directory trees.
impl<'t> Glob<'t> {
    /// Gets an iterator over matching file paths in a directory tree.
    ///
    /// This function matches a `Glob` against a directory tree, returning a
    /// [`FileIterator`] that yields a [`GlobEntry`] for each matching file.
    /// `Glob`s are the only [`Pattern`]s that support this semantic
    /// operation; it is not possible to match combinators ([`Any`]) against
    /// directory trees.
    ///
    /// As with [`Path::join`] and [`PathBuf::push`], the base directory can be
    /// escaped or overridden by rooted `Glob`s. In many cases, the current
    /// working directory `.` is an appropriate base directory and will be
    /// intuitively ignored if the `Glob` is rooted, such as in `/mnt/media/
    /// **/*.mp4`. The [`has_root`] function can be used to check if a `Glob` is
    /// rooted.
    ///
    /// The root directory is either the given directory or, if rooted, the
    /// [invariant prefix][`Glob::partition`] of the `Glob`. Either way,
    /// this function joins the given directory with any invariant prefix to
    /// potentially begin the walk as far down the tree as possible. **The
    /// prefix and any [semantic literals][`Glob::has_semantic_literals`] in
    /// this prefix are interpreted semantically as a path**, so components
    /// like `.` and `..` that precede variant patterns interact with the
    /// base directory semantically. This means that expressions like
    /// `../**` escape the base directory as expected on Unix and Windows, for
    /// example.
    ///
    /// This function uses the default [`WalkBehavior`]. To configure the
    /// behavior of the traversal, see [`Glob::walk_with_behavior`].
    ///
    /// Unlike functions in [`Pattern`], **this operation is semantic and
    /// interacts with the file system**.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use wax::walk::Entry;
    /// use wax::Glob;
    ///
    /// let glob = Glob::new("**/*.(?i){jpg,jpeg}").unwrap();
    /// for entry in glob.walk("./Pictures") {
    ///     let entry = entry.unwrap();
    ///     println!("JPEG: {:?}", entry.path());
    /// }
    /// ```
    ///
    /// Glob expressions do not support general negations, but the [`not`]
    /// combinator can be used when walking a directory tree to filter
    /// entries using patterns. **This should generally be preferred over
    /// functions like [`Iterator::filter`], because it avoids unnecessary reads
    /// of directory trees when matching [exhaustive
    /// negations][`Pattern::is_exhaustive`].**
    ///
    /// ```rust,no_run
    /// use wax::walk::{Entry, FileIterator};
    /// use wax::Glob;
    ///
    /// let glob = Glob::new("**/*.(?i){jpg,jpeg,png}").unwrap();
    /// for entry in glob
    ///     .walk("./Pictures")
    ///     .not(["**/(i?){background<s:0,1>,wallpaper<s:0,1>}/**"])
    ///     .unwrap()
    /// {
    ///     let entry = entry.unwrap();
    ///     println!("{:?}", entry.path());
    /// }
    /// ```
    ///
    /// [`Any`]: crate::Any
    /// [`Glob::walk_with_behavior`]: crate::Glob::walk_with_behavior
    /// [`GlobEntry`]: crate::walk::GlobEntry
    /// [`has_root`]: crate::Glob::has_root
    /// [`FileIterator`]: crate::walk::FileIterator
    /// [`Iterator::filter`]: std::iter::Iterator::filter
    /// [`not`]: crate::walk::FileIterator::not
    /// [`Path::join`]: std::path::Path::join
    /// [`PathBuf::push`]: std::path::PathBuf::push
    /// [`Pattern`]: crate::Pattern
    /// [`Pattern::is_exhaustive`]: crate::Pattern::is_exhaustive
    /// [`WalkBehavior`]: crate::walk::WalkBehavior
    pub fn walk(
        &self,
        directory: impl Into<PathBuf>,
    ) -> impl 'static + FileIterator<Entry = GlobEntry> {
        self.walk_with_behavior(directory, WalkBehavior::default())
    }

    /// Gets an iterator over matching files in a directory tree.
    ///
    /// This function is the same as [`Glob::walk`], but it additionally accepts
    /// a [`WalkBehavior`] that configures how the traversal interacts with
    /// symbolic links, the maximum depth from the root, etc.
    ///
    /// Depth is relative to the root directory of the traversal, which is
    /// determined by joining the given path and any [invariant
    /// prefix][`Glob::partition`] of the `Glob`.
    ///
    /// See [`Glob::walk`] for more information.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use wax::walk::{Entry, WalkBehavior};
    /// use wax::Glob;
    ///
    /// let glob = Glob::new("**/*.(?i){jpg,jpeg}").unwrap();
    /// for entry in glob.walk_with_behavior("./Pictures", WalkBehavior::default()) {
    ///     let entry = entry.unwrap();
    ///     println!("JPEG: {:?}", entry.path());
    /// }
    /// ```
    ///
    /// By default, symbolic links are read as normal files and their targets
    /// are ignored. To follow symbolic links and traverse any directories
    /// that they reference, specify a [`LinkBehavior`].
    ///
    /// ```rust,no_run
    /// use wax::walk::{Entry, LinkBehavior};
    /// use wax::Glob;
    ///
    /// let glob = Glob::new("**/*.txt").unwrap();
    /// for entry in glob.walk_with_behavior("/var/log", LinkBehavior::ReadTarget) {
    ///     let entry = entry.unwrap();
    ///     println!("Log: {:?}", entry.path());
    /// }
    /// ```
    ///
    /// [`Glob::partition`]: crate::Glob::partition
    /// [`Glob::walk`]: crate::Glob::walk
    /// [`LinkBehavior`]: crate::walk::LinkBehavior
    /// [`WalkBehavior`]: crate::walk::WalkBehavior
    pub fn walk_with_behavior(
        &self,
        directory: impl Into<PathBuf>,
        behavior: impl Into<WalkBehavior>,
    ) -> impl 'static + FileIterator<Entry = GlobEntry> {
        self.walker(directory).walk_with_behavior(behavior)
    }

    fn walker(&self, directory: impl Into<PathBuf>) -> GlobWalker {
        GlobWalker {
            anchor: self.anchor(directory),
            pattern: WalkPattern {
                complete: self.pattern.clone(),
                components: compile(self.tree.as_ref().tokens())
                    .expect("failed to compile glob sub-expressions"),
            },
        }
    }

    fn anchor(&self, directory: impl Into<PathBuf>) -> Anchor {
        fn invariant_path_prefix<'t, A, I>(tokens: I) -> Option<PathBuf>
        where
            A: 't,
            I: IntoIterator<Item = &'t Token<'t, A>>,
        {
            let prefix = token::invariant_text_prefix(tokens);
            if prefix.is_empty() {
                None
            } else {
                Some(prefix.into())
            }
        }

        let directory = directory.into();
        // Establish the root directory and any prefix in that root path that is not a
        // part of the glob expression. The directory tree is traversed from
        // `root`, which may include an invariant prefix from the glob. The
        // `prefix` is an integer that specifies how many components from the
        // end of the root path must be popped to get the portion of the root
        // path that is not present in the glob. The prefix may be empty or may be the
        // entirety of `root` depending on `directory` and the glob.
        //
        // Note that a rooted glob, like in `Path::join`, replaces `directory` when
        // establishing the root path. In this case, there is no prefix, as the
        // entire root path is present in the glob expression.
        let (root, prefix) = match invariant_path_prefix(self.tree.as_ref().tokens()) {
            Some(prefix) => directory.join_and_get_depth(prefix),
            _ => (directory, 0),
        };
        Anchor { root, prefix }
    }
}

/// Root path and prefix of a `Glob` when walking a particular path.
#[derive(Clone, Debug)]
struct Anchor {
    /// The root (starting) directory of the walk.
    root: PathBuf,
    /// The number of components from the end of `root` that are present in the
    /// `Glob`'s expression.
    prefix: usize,
}

impl Anchor {
    pub fn walk_with_behavior(self, behavior: impl Into<WalkBehavior>) -> WalkTree {
        WalkTree::with_prefix_and_behavior(self.root, self.prefix, behavior)
    }
}

#[derive(Clone, Debug)]
struct WalkPattern {
    complete: Regex,
    components: Vec<Regex>,
}

#[derive(Clone, Debug)]
struct GlobWalker {
    anchor: Anchor,
    pattern: WalkPattern,
}

impl GlobWalker {
    pub fn walk_with_behavior(
        self,
        behavior: impl Into<WalkBehavior>,
    ) -> impl 'static + FileIterator<Entry = GlobEntry, Residue = TreeEntry> {
        self.anchor
            .walk_with_behavior(behavior)
            .filter_map_tree(move |cancellation, separation| {
                use itertools::{
                    EitherOrBoth::{Both, Left, Right},
                    Position::{First, Last, Middle, Only},
                };

                let filtrate = match separation.filtrate() {
                    Some(filtrate) => match filtrate.transpose() {
                        Ok(filtrate) => filtrate,
                        Err(error) => {
                            return Separation::from(error.map(Err));
                        }
                    },
                    // `Path::walk_with_behavior` yields no residue.
                    _ => unreachable!(),
                };
                let entry = filtrate.as_ref();
                let (_, path) = entry.root_relative_paths();
                let depth = entry.depth().saturating_sub(1);
                for (position, candidate) in path
                    .components()
                    .skip(depth)
                    .filter_map(|component| match component {
                        Component::Normal(component) => Some(CandidatePath::from(component)),
                        _ => None,
                    })
                    .zip_longest(self.pattern.components.iter().skip(depth))
                    .with_position()
                {
                    match (position, candidate) {
                        (First | Middle, Both(candidate, pattern)) => {
                            if !pattern.is_match(candidate.as_ref()) {
                                // Do not walk directories that do not match the corresponding
                                // component pattern.
                                return filtrate.filter_tree(cancellation).into();
                            }
                        }
                        (Last | Only, Both(candidate, pattern)) => {
                            return if pattern.is_match(candidate.as_ref()) {
                                let candidate = CandidatePath::from(path);
                                if let Some(matched) = self
                                    .pattern
                                    .complete
                                    .captures(candidate.as_ref())
                                    .map(MatchedText::from)
                                    .map(MatchedText::into_owned)
                                {
                                    filtrate
                                        .map(|entry| Ok(GlobEntry { entry, matched }))
                                        .into()
                                } else {
                                    filtrate.filter_node().into()
                                }
                            } else {
                                // Do not walk directories that do not match the corresponding
                                // component pattern.
                                filtrate.filter_tree(cancellation).into()
                            };
                        }
                        (_, Left(_candidate)) => {
                            let candidate = CandidatePath::from(path);
                            return if let Some(matched) = self
                                .pattern
                                .complete
                                .captures(candidate.as_ref())
                                .map(MatchedText::from)
                                .map(MatchedText::into_owned)
                            {
                                filtrate
                                    .map(|entry| Ok(GlobEntry { entry, matched }))
                                    .into()
                            } else {
                                filtrate.filter_node().into()
                            };
                        }
                        (_, Right(_pattern)) => {
                            return filtrate.filter_node().into();
                        }
                    }
                }
                // If the component loop is not entered, then check for a match. This may
                // indicate that the `Glob` is empty and a single invariant path
                // may be matched.
                let candidate = CandidatePath::from(path);
                if let Some(matched) = self
                    .pattern
                    .complete
                    .captures(candidate.as_ref())
                    .map(MatchedText::from)
                    .map(MatchedText::into_owned)
                {
                    return filtrate
                        .map(|entry| Ok(GlobEntry { entry, matched }))
                        .into();
                }
                filtrate.filter_node().into()
            })
    }
}

#[derive(Clone, Debug)]
enum FilterAnyPattern {
    Empty,
    Exhaustive(Regex),
    Nonexhaustive(Regex),
    Partitioned {
        exhaustive: Regex,
        nonexhaustive: Regex,
    },
}

impl FilterAnyPattern {
    pub fn residue(&self, candidate: CandidatePath<'_>) -> Option<EntryResidue> {
        use FilterAnyPattern::{Exhaustive, Nonexhaustive, Partitioned};

        match self {
            Exhaustive(ref exhaustive) | Partitioned { ref exhaustive, .. }
                if exhaustive.is_match(candidate.as_ref()) =>
            {
                Some(EntryResidue::Tree)
            }
            Nonexhaustive(ref nonexhaustive)
            | Partitioned {
                ref nonexhaustive, ..
            } if nonexhaustive.is_match(candidate.as_ref()) => Some(EntryResidue::File),
            _ => None,
        }
    }
}

/// Negated glob combinator that efficiently filters file entries against
/// patterns.
#[derive(Clone, Debug)]
pub struct FilterAny {
    pattern: FilterAnyPattern,
}

impl FilterAny {
    /// Combines patterns into a `FilterAny`.
    ///
    /// This function accepts an [`IntoIterator`] with items that implement
    /// [`Combine`], such as [`Glob`] and `&str`.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the inputs fail to build. If the inputs are a
    /// compiled [`Pattern`] type such as [`Glob`], then this only occurs if
    /// the compiled program is too large.
    ///
    /// [`Combine`]: crate::Combine
    /// [`Glob`]: crate::Glob
    /// [`IntoIterator`]: std::iter::IntoIterator
    /// [`Pattern`]: crate::Pattern
    pub fn any<'t, I>(patterns: I) -> Result<Self, BuildError>
    where
        I: IntoIterator,
        I::Item: Combine<'t>,
    {
        let (exhaustive, nonexhaustive) = patterns
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)?
            .into_iter()
            .partition::<Vec<_>, _>(|tree| token::is_exhaustive(tree.as_ref().tokens()));
        Ok(FilterAny {
            // TODO: This kind of expression is a bit unfortunate. `FilterAnyPattern` is necessary,
            //       because empty token sequences yield the regular expression `^()$`, which
            //       matches empty strings and therefore empty `CandidatePath`s. Note that
            //       sometimes the relative path of an `Entry` is empty (when the entry represents
            //       the root). Alternatively, a pattern type that bypasses `Regex` and matches
            //       nothing when it is empty could be used instead. Then there would be no need to
            //       match against only non-empty patterns.
            pattern: match (exhaustive.is_empty(), nonexhaustive.is_empty()) {
                (false, false) => FilterAnyPattern::Partitioned {
                    exhaustive: crate::any(exhaustive)?.pattern,
                    nonexhaustive: crate::any(nonexhaustive)?.pattern,
                },
                (false, true) => FilterAnyPattern::Exhaustive(crate::any(exhaustive)?.pattern),
                (true, false) => {
                    FilterAnyPattern::Nonexhaustive(crate::any(nonexhaustive)?.pattern)
                }
                (true, true) => FilterAnyPattern::Empty,
            },
        })
    }

    /// Gets the appropriate [`EntryResidue`] for the given [`Entry`].
    ///
    /// Notably, this function returns [`EntryResidue::Tree`] if the [`Entry`]
    /// matches an [exhaustive glob expression][`Pattern::is_exhaustive`],
    /// such as `secret/**`.
    ///
    /// [`Entry`]: crate::walk::Entry
    /// [`EntryResidue`]: crate::walk::EntryResidue
    /// [`EntryResidue::Tree`]: crate::walk::EntryResidue::Tree
    /// [`Pattern::is_exhaustive`]: crate::Pattern::is_exhaustive
    pub fn residue(&self, entry: &dyn Entry) -> Option<EntryResidue> {
        let candidate = CandidatePath::from(entry.root_relative_paths().1);
        self.pattern.residue(candidate)
    }
}

/// Describes a file with a path matching a [`Glob`] in a directory tree.
///
/// See [`Glob::walk`].
///
/// [`Glob`]: crate::Glob
/// [`Glob::walk`]: crate::Glob::walk
#[derive(Debug)]
pub struct GlobEntry {
    entry: TreeEntry,
    matched: MatchedText<'static>,
}

impl GlobEntry {
    /// Converts the entry to the relative [`CandidatePath`].
    ///
    /// **This differs from [`Entry::path`] and [`Entry::into_path`], which are
    /// native paths and typically include the root path.** The
    /// [`CandidatePath`] is always relative to [the root
    /// path][`Entry::root_relative_paths`].
    ///
    /// [`CandidatePath`]: crate::CandidatePath
    /// [`Entry::into_path`]: crate::walk::Entry::into_path
    /// [`Entry::path`]: crate::walk::Entry::path
    /// [`matched`]: crate::walk::GlobEntry::matched
    pub fn to_candidate_path(&self) -> CandidatePath<'_> {
        self.matched.to_candidate_path()
    }

    /// Gets the matched text in the path of the file.
    pub fn matched(&self) -> &MatchedText<'static> {
        &self.matched
    }
}

impl Entry for GlobEntry {
    fn into_path(self) -> PathBuf {
        self.entry.into_path()
    }

    fn path(&self) -> &Path {
        self.entry.path()
    }

    fn root_relative_paths(&self) -> (&Path, &Path) {
        self.entry.root_relative_paths()
    }

    fn file_type(&self) -> FileType {
        self.entry.file_type()
    }

    fn metadata(&self) -> Result<Metadata, WalkError> {
        self.entry.metadata().map_err(WalkError::from)
    }

    // TODO: This needs some work and requires some explanation when applied to
    // globs.
    fn depth(&self) -> usize {
        self.entry.depth()
    }
}

impl From<GlobEntry> for TreeEntry {
    fn from(entry: GlobEntry) -> Self {
        entry.entry
    }
}

fn compile<'t, I>(tokens: I) -> Result<Vec<Regex>, CompileError>
where
    I: IntoIterator<Item = &'t Token<'t>>,
    I::IntoIter: Clone,
{
    let mut regexes = Vec::new();
    for component in token::components(tokens) {
        if component
            .tokens()
            .iter()
            .any(|token| token.has_component_boundary())
        {
            // Stop at component boundaries, such as tree wildcards or any boundary within a
            // group token.
            break;
        }
        regexes.push(Glob::compile(component.tokens().iter().copied())?);
    }
    Ok(regexes)
}
