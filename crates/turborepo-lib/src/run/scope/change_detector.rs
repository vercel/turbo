use std::collections::HashSet;

use turbopath::{AbsoluteSystemPath, AnchoredSystemPath, AnchoredSystemPathBuf};
use turborepo_repository::package_graph::{ChangedPackagesError, PackageGraph, WorkspaceName};
use turborepo_scm::SCM;
use wax::Pattern;

pub trait PackageChangeDetector {
    /// Get the list of changed packages between two refs.
    fn changed_packages(
        &self,
        from_ref: &str,
        to_ref: &str,
    ) -> Result<HashSet<WorkspaceName>, ChangeDetectError>;
}

pub struct SCMChangeDetector<'a> {
    turbo_root: &'a AbsoluteSystemPath,

    scm: &'a SCM,
    pkg_graph: &'a PackageGraph,

    global_deps: Vec<String>,
    ignore_patterns: Vec<String>,
}

impl<'a> PackageChangeDetector for SCMChangeDetector<'a> {
    fn changed_packages(
        &self,
        from_ref: &str,
        to_ref: &str,
    ) -> Result<HashSet<WorkspaceName>, ChangeDetectError> {
        let mut changed_files = HashSet::new();
        if !from_ref.is_empty() {
            changed_files = self
                .scm
                .changed_files(self.turbo_root, Some(from_ref), to_ref)?;
        }

        let global_change =
            self.repo_global_file_has_changed(&Self::DEFAULT_GLOBAL_DEPS, &changed_files)?;

        if global_change {
            return Ok(self
                .pkg_graph
                .workspaces()
                .map(|(n, _)| n.to_owned())
                .collect());
        }

        // get filtered files and add the packages that contain them
        let filtered_changed_files = self.filter_ignored_files(changed_files.iter())?;
        let mut changed_pkgs =
            self.get_changed_packages(filtered_changed_files.into_iter(), self.pkg_graph)?;

        // if we run into issues, don't error, just assume all pacakges have changed
        let lockfile_changes = self.get_changes_from_lockfile(&changed_files, from_ref);

        if let Ok(lockfile_changes) = lockfile_changes {
            changed_pkgs.extend(lockfile_changes);
        } else {
            return Ok(self
                .pkg_graph
                .workspaces()
                .map(|(n, _)| n.to_owned())
                .collect());
        }
        Ok(changed_pkgs)
    }
}

impl<'a> SCMChangeDetector<'a> {
    const DEFAULT_GLOBAL_DEPS: [&'static str; 2] = ["package.json", "turbo.json"];

    pub fn new(
        turbo_root: &'a AbsoluteSystemPath,

        scm: &'a SCM,
        pkg_graph: &'a PackageGraph,
        global_deps: Vec<String>,
        ignore_patterns: Vec<String>,
    ) -> Self {
        Self {
            turbo_root,
            scm,
            pkg_graph,
            global_deps,
            ignore_patterns,
        }
    }

    fn repo_global_file_has_changed(
        &self,
        default_global_deps: &[&str],
        changed_files: &HashSet<AnchoredSystemPathBuf>,
    ) -> Result<bool, turborepo_scm::Error> {
        let global_deps = self.global_deps.iter().map(|s| s.as_str());
        let filters = global_deps.chain(default_global_deps.iter().copied());
        let matcher = wax::any(filters).unwrap();
        Ok(changed_files.iter().any(|f| matcher.is_match(f.as_path())))
    }

    fn filter_ignored_files<'b>(
        &self,
        changed_files: impl Iterator<Item = &'b AnchoredSystemPathBuf> + 'b,
    ) -> Result<HashSet<&'b AnchoredSystemPathBuf>, turborepo_scm::Error> {
        let matcher = wax::any(self.ignore_patterns.iter().map(|s| s.as_str())).unwrap();
        Ok(changed_files
            .filter(move |f| !matcher.is_match(f.as_path()))
            .collect())
    }

    // note: this could probably be optimized by using a hashmap of package paths
    fn get_changed_packages<'b>(
        &self,
        files: impl Iterator<Item = &'b AnchoredSystemPathBuf>,
        graph: &PackageGraph,
    ) -> Result<HashSet<WorkspaceName>, turborepo_scm::Error> {
        let mut changed_packages = HashSet::new();
        for file in files {
            let mut found = false;
            for (name, entry) in graph.workspaces() {
                if name == &WorkspaceName::Root {
                    continue;
                }
                if let Some(package_path) = entry.package_json_path.parent() {
                    if Self::is_file_in_package(file, package_path) {
                        changed_packages.insert(name.to_owned());
                        found = true;
                        break;
                    }
                }
            }
            if !found {
                // if the file is not in any package, it must be in the root package
                changed_packages.insert(WorkspaceName::Root);
            }
        }

        Ok(changed_packages)
    }

    fn is_file_in_package(file: &AnchoredSystemPath, package_path: &AnchoredSystemPath) -> bool {
        file.components()
            .zip(package_path.components())
            .all(|(a, b)| a == b)
    }

    /// Get a list of package names that have changed due to changes in the
    /// lockfile across the HEAD and the given ref.
    fn get_changes_from_lockfile(
        &self,
        changed_files: &HashSet<AnchoredSystemPathBuf>,
        from_ref: &str,
    ) -> Result<Vec<WorkspaceName>, ChangeDetectError> {
        let lockfile_path = self
            .pkg_graph
            .package_manager()
            .lockfile_path(self.turbo_root);

        // no changes to lockfile? then no changes because of lockfile
        if !SCMChangeDetector::lockfile_changed(self.turbo_root, changed_files, &lockfile_path)? {
            return Ok(vec![]);
        }

        let previous_file = self.scm.previous_content(from_ref, &lockfile_path)?;
        let previous_lockfile = self
            .pkg_graph
            .package_manager()
            .parse_lockfile(self.pkg_graph.root_package_json(), &previous_file)?;

        let additional_packages = self
            .pkg_graph
            .changed_packages(previous_lockfile.as_ref())?;

        Ok(additional_packages)
    }

    fn lockfile_changed(
        turbo_root: &AbsoluteSystemPath,
        changed_files: &HashSet<AnchoredSystemPathBuf>,
        lockfile_path: &AbsoluteSystemPath,
    ) -> Result<bool, ChangeDetectError> {
        let lockfile_path_relative = turbo_root
            .anchor(lockfile_path)
            .expect("lockfile should be in repo");

        Ok(changed_files.iter().any(|f| f == &lockfile_path_relative))
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ChangeDetectError {
    #[error("SCM error: {0}")]
    Scm(#[from] turborepo_scm::Error),
    #[error("Wax error: {0}")]
    Wax(#[from] wax::BuildError),
    #[error("Package manager error: {0}")]
    PackageManager(#[from] turborepo_repository::package_manager::Error),
    #[error("No lockfile")]
    NoLockfile,
    #[error("Lockfile error: {0}")]
    Lockfile(turborepo_lockfiles::Error),
}

impl From<ChangedPackagesError> for ChangeDetectError {
    fn from(value: ChangedPackagesError) -> Self {
        match value {
            ChangedPackagesError::NoLockfile => Self::NoLockfile,
            ChangedPackagesError::Lockfile(e) => Self::Lockfile(e),
        }
    }
}

#[cfg(test)]
mod test {
    use test_case::test_case;

    use super::{ChangeDetectError, SCMChangeDetector};

    #[cfg(unix)]
    #[test_case("/a/b/c", &["package.lock"], "/a/b/c/package.lock", Ok(true) ; "simple")]
    #[test_case("/a/b/c", &["a", "b", "c"], "/a/b/c/package.lock", Ok(false) ; "lockfile unchanged")]
    fn test_lockfile_changed(
        turbo_root: &str,
        changed_files: &[&str],
        lockfile_path: &str,
        expected: Result<bool, ChangeDetectError>,
    ) {
        let turbo_root = turbopath::AbsoluteSystemPathBuf::new(turbo_root).unwrap();
        let lockfile_path = turbopath::AbsoluteSystemPathBuf::new(lockfile_path).unwrap();
        let changed_files = changed_files
            .iter()
            .map(|s| turbopath::AnchoredSystemPathBuf::from_raw(s).unwrap())
            .collect();
        let changes =
            SCMChangeDetector::lockfile_changed(&turbo_root, &changed_files, &lockfile_path);

        // we don't want to implement PartialEq on the error type,
        // so simply compare the debug representations
        assert_eq!(format!("{:?}", changes), format!("{:?}", expected));
    }

    #[cfg(windows)]
    #[test_case("C:\\\\a\\b\\c", &["package.lock"], "C:\\\\a\\b\\c\\package.lock", Ok(true) ; "simple")]
    #[test_case("C:\\\\a\\b\\c", &["a", "b", "c"],  "C:\\\\a\\b\\c\\package.lock", Ok(false) ; "lockfile unchanged")]
    fn test_lockfile_changed(
        turbo_root: &str,
        changed_files: &[&str],
        lockfile_path: &str,
        expected: Result<bool, ChangeDetectError>,
    ) {
        let turbo_root = turbopath::AbsoluteSystemPathBuf::new(turbo_root).unwrap();
        let lockfile_path = turbopath::AbsoluteSystemPathBuf::new(lockfile_path).unwrap();
        let changed_files = changed_files
            .iter()
            .map(|s| turbopath::AnchoredSystemPathBuf::from_raw(s).unwrap())
            .collect();
        let changes =
            SCMChangeDetector::lockfile_changed(&turbo_root, &changed_files, &lockfile_path);

        // we don't want to implement PartialEq on the error type,
        // so simply compare the debug representations
        assert_eq!(format!("{:?}", changes), format!("{:?}", expected));
    }
}
