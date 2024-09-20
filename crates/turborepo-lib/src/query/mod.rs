mod file;
mod package;
mod server;

use std::{borrow::Cow, io, sync::Arc};

use async_graphql::{http::GraphiQLSource, *};
use axum::{response, response::IntoResponse};
use itertools::Itertools;
use miette::Diagnostic;
use package::Package;
pub use server::run_server;
use thiserror::Error;
use tokio::select;
use turbo_trace::TraceError;
use turbopath::AbsoluteSystemPathBuf;
use turborepo_repository::package_graph::PackageName;
use turborepo_scm::git::ChangedFiles;

use crate::{
    get_version,
    global_deps_package_change_mapper,
    query::file::File,
    run::{builder::RunBuilder, Run},
    signal::SignalHandler,
};

#[derive(Error, Debug, Diagnostic)]
pub enum Error {
    #[error("failed to get file dependencies")]
    Trace(#[related] Vec<TraceError>),
    #[error("no signal handler")]
    NoSignalHandler,
    #[error("file `{0}` not found")]
    FileNotFound(String),
    #[error("failed to start GraphQL server")]
    Server(#[from] io::Error),
    #[error("package not found: {0}")]
    PackageNotFound(PackageName),
    #[error("failed to serialize result: {0}")]
    Serde(#[from] serde_json::Error),
    #[error(transparent)]
    Run(#[from] crate::run::Error),
    #[error(transparent)]
    Path(#[from] turbopath::PathError),
    #[error(transparent)]
    UI(#[from] turborepo_ui::Error),
    ChangeMapper(#[from] global_deps_package_change_mapper::Error),
    #[error(transparent)]
    Scm(#[from] turborepo_scm::Error),
}

pub struct RepositoryQuery {
    run: Arc<Run>,
}

impl RepositoryQuery {
    pub fn new(run: Arc<Run>) -> Self {
        Self { run }
    }
}

#[derive(Debug, SimpleObject, Default)]
#[graphql(concrete(name = "Files", params(File)))]
#[graphql(concrete(name = "Packages", params(Package)))]
pub struct Array<T: OutputType> {
    items: Vec<T>,
    length: usize,
}

impl<T: OutputType> Array<T> {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            length: 0,
        }
    }
}

impl<T: OutputType> FromIterator<T> for Array<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let items: Vec<_> = iter.into_iter().collect();
        let length = items.len();
        Self { items, length }
    }
}

impl<T: OutputType> TypeName for Array<T> {
    fn type_name() -> Cow<'static, str> {
        Cow::Owned(format!("Array<{}>", T::type_name()))
    }
}

#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
enum PackageFields {
    Name,
    TaskName,
    DirectDependencyCount,
    DirectDependentCount,
    IndirectDependentCount,
    IndirectDependencyCount,
    AllDependentCount,
    AllDependencyCount,
}

#[derive(InputObject)]
struct FieldValuePair {
    field: PackageFields,
    value: Any,
}

/// Predicates are used to filter packages. If you include multiple predicates,
/// they are combined using AND. To combine predicates using OR, use the `or`
/// field.
///
/// For pairs that do not obey type safety, e.g. `NAME` `greater_than` `10`, we
/// default to `false`.
#[derive(InputObject)]
struct PackagePredicate {
    and: Option<Vec<PackagePredicate>>,
    or: Option<Vec<PackagePredicate>>,
    equal: Option<FieldValuePair>,
    not_equal: Option<FieldValuePair>,
    greater_than: Option<FieldValuePair>,
    less_than: Option<FieldValuePair>,
    not: Option<Box<PackagePredicate>>,
    has: Option<FieldValuePair>,
}

impl PackagePredicate {
    fn check_equals(pkg: &Package, field: &PackageFields, value: &Any) -> bool {
        match (field, &value.0) {
            (PackageFields::Name, Value::String(name)) => pkg.name.as_ref() == name,
            (PackageFields::DirectDependencyCount, Value::Number(n)) => {
                let Some(n) = n.as_u64() else {
                    return false;
                };
                pkg.direct_dependencies_count() == n as usize
            }
            (PackageFields::DirectDependentCount, Value::Number(n)) => {
                let Some(n) = n.as_u64() else {
                    return false;
                };
                pkg.direct_dependents_count() == n as usize
            }
            (PackageFields::IndirectDependentCount, Value::Number(n)) => {
                let Some(n) = n.as_u64() else {
                    return false;
                };
                pkg.indirect_dependents_count() == n as usize
            }
            (PackageFields::IndirectDependencyCount, Value::Number(n)) => {
                let Some(n) = n.as_u64() else {
                    return false;
                };
                pkg.indirect_dependencies_count() == n as usize
            }
            (PackageFields::AllDependentCount, Value::Number(n)) => {
                let Some(n) = n.as_u64() else {
                    return false;
                };
                pkg.all_dependents_count() == n as usize
            }
            (PackageFields::AllDependencyCount, Value::Number(n)) => {
                let Some(n) = n.as_u64() else {
                    return false;
                };
                pkg.all_dependencies_count() == n as usize
            }
            _ => false,
        }
    }

    fn check_greater_than(pkg: &Package, field: &PackageFields, value: &Any) -> bool {
        match (field, &value.0) {
            (PackageFields::DirectDependencyCount, Value::Number(n)) => {
                let Some(n) = n.as_u64() else {
                    return false;
                };
                pkg.direct_dependencies_count() > n as usize
            }
            (PackageFields::DirectDependentCount, Value::Number(n)) => {
                let Some(n) = n.as_u64() else {
                    return false;
                };
                pkg.direct_dependents_count() > n as usize
            }
            (PackageFields::IndirectDependentCount, Value::Number(n)) => {
                let Some(n) = n.as_u64() else {
                    return false;
                };
                pkg.indirect_dependents_count() > n as usize
            }
            (PackageFields::IndirectDependencyCount, Value::Number(n)) => {
                let Some(n) = n.as_u64() else {
                    return false;
                };
                pkg.indirect_dependencies_count() > n as usize
            }
            (PackageFields::AllDependentCount, Value::Number(n)) => {
                let Some(n) = n.as_u64() else {
                    return false;
                };
                pkg.all_dependents_count() > n as usize
            }
            (PackageFields::AllDependencyCount, Value::Number(n)) => {
                let Some(n) = n.as_u64() else {
                    return false;
                };
                pkg.all_dependencies_count() > n as usize
            }
            _ => false,
        }
    }

    fn check_less_than(pkg: &Package, field: &PackageFields, value: &Any) -> bool {
        match (field, &value.0) {
            (PackageFields::DirectDependencyCount, Value::Number(n)) => {
                let Some(n) = n.as_u64() else {
                    return false;
                };
                pkg.direct_dependencies_count() < n as usize
            }
            (PackageFields::DirectDependentCount, Value::Number(n)) => {
                let Some(n) = n.as_u64() else {
                    return false;
                };
                pkg.direct_dependents_count() < n as usize
            }
            (PackageFields::IndirectDependentCount, Value::Number(n)) => {
                let Some(n) = n.as_u64() else {
                    return false;
                };
                pkg.indirect_dependents_count() < n as usize
            }
            (PackageFields::IndirectDependencyCount, Value::Number(n)) => {
                let Some(n) = n.as_u64() else {
                    return false;
                };
                pkg.indirect_dependencies_count() < n as usize
            }
            (PackageFields::AllDependentCount, Value::Number(n)) => {
                let Some(n) = n.as_u64() else {
                    return false;
                };
                pkg.all_dependents_count() < n as usize
            }
            (PackageFields::AllDependencyCount, Value::Number(n)) => {
                let Some(n) = n.as_u64() else {
                    return false;
                };
                pkg.all_dependencies_count() < n as usize
            }
            _ => false,
        }
    }

    fn check_has(pkg: &Package, field: &PackageFields, value: &Any) -> bool {
        match (field, &value.0) {
            (PackageFields::Name, Value::String(name)) => pkg.name.as_ref() == name,
            (PackageFields::TaskName, Value::String(name)) => pkg.task_names().contains(name),
            _ => false,
        }
    }

    fn check(&self, pkg: &Package) -> bool {
        let and = self
            .and
            .as_ref()
            .map(|predicates| predicates.iter().all(|p| p.check(pkg)));
        let or = self
            .or
            .as_ref()
            .map(|predicates| predicates.iter().any(|p| p.check(pkg)));
        let equal = self
            .equal
            .as_ref()
            .map(|pair| Self::check_equals(pkg, &pair.field, &pair.value));
        let not_equal = self
            .not_equal
            .as_ref()
            .map(|pair| !Self::check_equals(pkg, &pair.field, &pair.value));

        let greater_than = self
            .greater_than
            .as_ref()
            .map(|pair| Self::check_greater_than(pkg, &pair.field, &pair.value));

        let less_than = self
            .less_than
            .as_ref()
            .map(|pair| Self::check_greater_than(pkg, &pair.field, &pair.value));
        let not = self.not.as_ref().map(|predicate| !predicate.check(pkg));
        let has = self
            .has
            .as_ref()
            .map(|pair| Self::check_has(pkg, &pair.field, &pair.value));

        and.into_iter()
            .chain(or)
            .chain(equal)
            .chain(not_equal)
            .chain(greater_than)
            .chain(less_than)
            .chain(not)
            .chain(has)
            .all(|p| p)
    }
}

#[Object]
impl RepositoryQuery {
    async fn affected_packages(
        &self,
        base: Option<String>,
        head: Option<String>,
        filter: Option<PackagePredicate>,
    ) -> Result<Array<Package>, Error> {
        let mut opts = self.run.opts().clone();
        opts.scope_opts.affected_range = Some((base, head));

        Ok(RunBuilder::calculate_filtered_packages(
            self.run.repo_root(),
            &opts,
            self.run.pkg_dep_graph(),
            self.run.scm(),
            self.run.root_turbo_json(),
        )?
        .into_iter()
        .map(|package| Package {
            run: self.run.clone(),
            name: package,
        })
        .filter(|package| filter.as_ref().map_or(true, |f| f.check(package)))
        .sorted_by(|a, b| a.name.cmp(&b.name))
        .collect())
    }
    /// Gets a single package by name
    async fn package(&self, name: String) -> Result<Package, Error> {
        let name = PackageName::from(name);
        Ok(Package {
            run: self.run.clone(),
            name,
        })
    }

    async fn version(&self) -> &'static str {
        get_version()
    }

    async fn file(&self, path: String) -> Result<File, Error> {
        let abs_path = AbsoluteSystemPathBuf::from_unknown(self.run.repo_root(), path);

        if !abs_path.exists() {
            return Err(Error::FileNotFound(abs_path.to_string()));
        }

        Ok(File::new(self.run.clone(), abs_path))
    }

    async fn affected_files(
        &self,
        base: Option<String>,
        head: Option<String>,
        /// Defaults to true if `head` is not provided
        include_uncommitted: Option<bool>,
        /// Defaults to true
        merge_base: Option<bool>,
    ) -> Result<Array<File>, Error> {
        let base = base.as_deref();
        let head = head.as_deref();
        let include_uncommitted = include_uncommitted.unwrap_or_else(|| head.is_none());
        let merge_base = merge_base.unwrap_or(true);
        let repo_root = self.run.repo_root();
        let change_result = self.run.scm().changed_files(
            repo_root,
            base,
            head,
            include_uncommitted,
            false,
            merge_base,
        )?;

        let files = match change_result {
            // Shouldn't happen since we set `allow_unknown_objects` to false
            ChangedFiles::All => unreachable!(),
            ChangedFiles::Some(files) => files,
        };

        Ok(files
            .into_iter()
            .map(|file| File::new(self.run.clone(), self.run.repo_root().resolve(&file)))
            .collect())
    }

    /// Gets a list of packages that match the given filter
    async fn packages(&self, filter: Option<PackagePredicate>) -> Result<Array<Package>, Error> {
        let Some(filter) = filter else {
            return Ok(self
                .run
                .pkg_dep_graph()
                .packages()
                .map(|(name, _)| Package {
                    run: self.run.clone(),
                    name: name.clone(),
                })
                .sorted_by(|a, b| a.name.cmp(&b.name))
                .collect());
        };

        Ok(self
            .run
            .pkg_dep_graph()
            .packages()
            .map(|(name, _)| Package {
                run: self.run.clone(),
                name: name.clone(),
            })
            .filter(|pkg| filter.check(pkg))
            .sorted_by(|a, b| a.name.cmp(&b.name))
            .collect())
    }
}

pub async fn graphiql() -> impl IntoResponse {
    response::Html(GraphiQLSource::build().endpoint("/").finish())
}

pub async fn run_query_server(run: Run, signal: SignalHandler) -> Result<(), Error> {
    let subscriber = signal.subscribe().ok_or(Error::NoSignalHandler)?;
    println!("GraphiQL IDE: http://localhost:8000");
    webbrowser::open("http://localhost:8000")?;
    select! {
        biased;
        _ = subscriber.listen() => {
            println!("Shutting down GraphQL server");
            return Ok(());
        }
        result = server::run_server(None, Arc::new(run)) => {
            result?;
        }
    }

    Ok(())
}
