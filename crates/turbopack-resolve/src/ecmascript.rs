use anyhow::Result;
use turbo_tasks::Vc;
use turbopack_core::resolve::options::{
    ConditionValue, ResolutionConditions, ResolveInPackage, ResolveIntoPackage, ResolveOptions,
};

/// Retrieves the [ResolutionConditions] of both the "into" package (allowing a
/// package to control how it can be imported) and the "in" package (controlling
/// how this package imports others) resolution options, so that they can be
/// manipulated together.
pub fn get_condition_maps(
    options: &mut ResolveOptions,
) -> impl Iterator<Item = &mut ResolutionConditions> {
    options
        .into_package
        .iter_mut()
        .filter_map(|item| {
            if let ResolveIntoPackage::ExportsField { conditions, .. } = item {
                Some(conditions)
            } else {
                None
            }
        })
        .chain(options.in_package.iter_mut().filter_map(|item| {
            if let ResolveInPackage::ImportsField { conditions, .. } = item {
                Some(conditions)
            } else {
                None
            }
        }))
}

#[turbo_tasks::function]
pub async fn apply_esm_specific_options(options: Vc<ResolveOptions>) -> Result<Vc<ResolveOptions>> {
    let mut options: ResolveOptions = options.await?.clone_value();
    // TODO set fully_specified when in strict ESM mode
    // options.fully_specified = true;
    for conditions in get_condition_maps(&mut options) {
        conditions.insert("import".to_string(), ConditionValue::Set);
        conditions.insert("require".to_string(), ConditionValue::Unset);
    }
    Ok(options.into())
}

#[turbo_tasks::function]
pub async fn apply_cjs_specific_options(options: Vc<ResolveOptions>) -> Result<Vc<ResolveOptions>> {
    let mut options: ResolveOptions = options.await?.clone_value();
    for conditions in get_condition_maps(&mut options) {
        conditions.insert("import".to_string(), ConditionValue::Unset);
        conditions.insert("require".to_string(), ConditionValue::Set);
    }
    Ok(options.into())
}
