use anyhow::Result;
use turbo_tasks::{Value, Vc};
use turbopack_core::{
    issue::{IssueSeverity, IssueSource},
    reference_type::{CommonJsReferenceSubType, EcmaScriptModulesReferenceSubType, ReferenceType},
    resolve::{
        handle_resolve_error,
        options::ResolveOptions,
        origin::{ResolveOrigin, ResolveOriginExt},
        parse::Request,
        ModuleResolveResult, ModuleResolveResultItem,
    },
};
// TODO: remove
pub use turbopack_resolve::ecmascript::{apply_cjs_specific_options, apply_esm_specific_options};

use crate::references::external_module::ExternalModuleAsset;

#[turbo_tasks::function]
pub async fn esm_resolve(
    origin: Vc<Box<dyn ResolveOrigin>>,
    request: Vc<Request>,
    ty: Value<EcmaScriptModulesReferenceSubType>,
    issue_severity: Vc<IssueSeverity>,
    issue_source: Option<Vc<IssueSource>>,
    import_externals: bool,
) -> Result<Vc<ModuleResolveResult>> {
    let ty = Value::new(ReferenceType::EcmaScriptModules(ty.into_value()));
    let options = apply_esm_specific_options(origin.resolve_options(ty.clone()))
        .resolve()
        .await?;

    specific_resolve(
        origin,
        request,
        options,
        ty,
        issue_severity,
        issue_source,
        import_externals,
    )
    .await
}

#[turbo_tasks::function]
pub async fn cjs_resolve(
    origin: Vc<Box<dyn ResolveOrigin>>,
    request: Vc<Request>,
    issue_source: Option<Vc<IssueSource>>,
    issue_severity: Vc<IssueSeverity>,
) -> Result<Vc<ModuleResolveResult>> {
    // TODO pass CommonJsReferenceSubType
    let ty = Value::new(ReferenceType::CommonJs(CommonJsReferenceSubType::Undefined));
    let options = apply_cjs_specific_options(origin.resolve_options(ty.clone()))
        .resolve()
        .await?;

    specific_resolve(
        origin,
        request,
        options,
        ty,
        issue_severity,
        issue_source,
        false,
    )
    .await
}

async fn specific_resolve(
    origin: Vc<Box<dyn ResolveOrigin>>,
    request: Vc<Request>,
    options: Vc<ResolveOptions>,
    reference_type: Value<ReferenceType>,
    issue_severity: Vc<IssueSeverity>,
    issue_source: Option<Vc<IssueSource>>,
    import_externals: bool,
) -> Result<Vc<ModuleResolveResult>> {
    let result = origin.resolve_asset(request, options, reference_type.clone());

    let result = handle_resolve_error(
        result,
        reference_type,
        origin.origin_path(),
        request,
        options,
        issue_severity,
        issue_source,
    )
    .await?;

    replace_externals(result, import_externals).await
}

pub fn try_to_severity(in_try: bool) -> Vc<IssueSeverity> {
    if in_try {
        IssueSeverity::Warning.cell()
    } else {
        IssueSeverity::Error.cell()
    }
}

/// Replaces the externals in the result with `ExternalModuleAsset` instances.
pub async fn replace_externals(
    result: Vc<ModuleResolveResult>,
    import_externals: bool,
) -> Result<Vc<ModuleResolveResult>> {
    let mut result = result.await?.clone_value();

    for item in result.primary.values_mut() {
        if let ModuleResolveResultItem::External(request, ty) = item {
            let module = ExternalModuleAsset::new(request.clone(), *ty, import_externals)
                .resolve()
                .await?;

            *item = ModuleResolveResultItem::Module(Vc::upcast(module));
        }
    }

    Ok(result.cell())
}
