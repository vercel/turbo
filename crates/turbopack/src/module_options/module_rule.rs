use anyhow::Result;
use serde::{Deserialize, Serialize};
use turbo_tasks::trace::TraceRawVcs;
use turbo_tasks_fs::FileSystemPath;
use turbopack_core::{
    asset::AssetVc, reference_type::ReferenceType, source_transform::SourceTransformsVc,
};
use turbopack_css::CssInputTransformsVc;
use turbopack_ecmascript::EcmascriptInputTransformsVc;

use super::ModuleRuleCondition;

#[derive(Debug, Clone, Serialize, Deserialize, TraceRawVcs, PartialEq, Eq)]
pub struct ModuleRule {
    condition: ModuleRuleCondition,
    effects: Vec<ModuleRuleEffect>,
}

impl ModuleRule {
    pub fn new(condition: ModuleRuleCondition, effects: Vec<ModuleRuleEffect>) -> Self {
        ModuleRule { condition, effects }
    }

    pub fn effects(&self) -> impl Iterator<Item = &ModuleRuleEffect> {
        self.effects.iter()
    }

    pub async fn matches(
        &self,
        source: AssetVc,
        path: &FileSystemPath,
        reference_type: &ReferenceType,
    ) -> Result<bool> {
        self.condition.matches(source, path, reference_type).await
    }
}

#[turbo_tasks::value(shared)]
#[derive(Debug, Clone)]
pub enum ModuleRuleEffect {
    ModuleType(ModuleType),
    AddEcmascriptTransforms(EcmascriptInputTransformsVc),
    SourceTransforms(SourceTransformsVc),
    Custom,
}

#[turbo_tasks::value(serialization = "auto_for_input", shared)]
#[derive(PartialOrd, Ord, Hash, Debug, Copy, Clone)]
pub enum ModuleType {
    Ecmascript {
        transforms: EcmascriptInputTransformsVc,
        options: EcmascriptOptions,
    },
    Typescript(EcmascriptInputTransformsVc),
    TypescriptWithTypes(EcmascriptInputTransformsVc),
    TypescriptDeclaration(EcmascriptInputTransformsVc),
    Json,
    Raw,
    Mdx(EcmascriptInputTransformsVc),
    Css(CssInputTransformsVc),
    CssModule(CssInputTransformsVc),
    Static,
    // TODO allow custom function when we support function pointers
    Custom(u8),
}

#[derive(Eq, PartialEq, PartialOrd, Ord, Hash, Debug, Copy, Clone, Serialize, Deserialize)]
pub struct EcmascriptOptions {
    /// module is split into smaller module parts and they can selectively
    /// imported
    pub split_into_parts: bool,
    /// imports will import parts of modules
    pub import_parts: bool,
}
