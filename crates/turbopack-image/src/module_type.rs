use anyhow::Result;
use indexmap::indexmap;
use turbo_tasks::Value;
use turbopack_core::{
    asset::AssetVc,
    context::{AssetContext, AssetContextVc},
    plugin::{CustomModuleType, CustomModuleTypeVc},
    resolve::ModulePartVc,
};
use turbopack_ecmascript::{
    EcmascriptInputTransformsVc, EcmascriptModuleAssetType, EcmascriptModuleAssetVc,
    EcmascriptOptions, InnerAssetsVc,
};
use turbopack_static::StaticModuleAssetVc;

use crate::source::StructuredImageSourceAsset;

#[turbo_tasks::value]
pub struct StructuredImageModuleType {}

#[turbo_tasks::value_impl]
impl StructuredImageModuleTypeVc {
    #[turbo_tasks::function]
    pub fn new() -> Self {
        StructuredImageModuleTypeVc::cell(StructuredImageModuleType {})
    }
}

#[turbo_tasks::value_impl]
impl CustomModuleType for StructuredImageModuleType {
    #[turbo_tasks::function]
    async fn create_module(
        &self,
        source: AssetVc,
        context: AssetContextVc,
        _part: Option<ModulePartVc>,
    ) -> Result<AssetVc> {
        let static_asset = StaticModuleAssetVc::new(source, context);
        Ok(EcmascriptModuleAssetVc::new_with_inner_assets(
            StructuredImageSourceAsset { image: source }.cell().into(),
            context,
            Value::new(EcmascriptModuleAssetType::Ecmascript),
            EcmascriptInputTransformsVc::empty(),
            Value::new(EcmascriptOptions {
                ..Default::default()
            }),
            context.compile_time_info(),
            InnerAssetsVc::cell(indexmap!(
                "IMAGE".to_string() => static_asset.into()
            )),
        )
        .into())
    }
}
