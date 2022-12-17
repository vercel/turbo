use std::collections::HashMap;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use turbo_tasks::{trace::TraceRawVcs, Value};
use turbo_tasks_fs::{to_sys_path, FileSystemEntryType, FileSystemPathVc};
use turbopack_core::{
    chunk::ChunkingContextVc, context::AssetContextVc, source_asset::SourceAssetVc,
};
use turbopack_ecmascript::{
    chunk::EcmascriptChunkPlaceablesVc, EcmascriptInputTransformsVc, EcmascriptModuleAssetType,
    EcmascriptModuleAssetVc,
};
use turbopack_node::evaluate::{evaluate, JavaScriptValue};

use crate::embed_js::next_asset;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
#[turbo_tasks::value(transparent, serialization = "custom")]
pub struct NextConfig {
    pub config_file: Option<String>,
    pub config_file_name: String,
    pub typescript: Option<TypeScriptConfig>,
    pub react_strict_mode: Option<bool>,
    pub experimental: Option<ExperimentalConfig>,
    pub env: Option<HashMap<String, String>>,
    pub compiler: Option<CompilerConfig>,
    pub images: ImageConfig,
}

#[derive(Clone, Debug, Ord, PartialOrd, PartialEq, Eq, Serialize, Deserialize, TraceRawVcs)]
#[serde(rename_all = "camelCase")]
pub struct TypeScriptConfig {
    pub ignore_build_errors: Option<bool>,
    pub ts_config_path: Option<String>,
}

#[derive(Clone, Debug, Ord, PartialOrd, PartialEq, Eq, Serialize, Deserialize, TraceRawVcs)]
#[serde(rename_all = "camelCase")]
pub struct ImageConfig {
    pub device_sizes: Vec<u16>,
    pub image_sizes: Vec<u16>,
    pub path: String,
    pub loader: ImageLoader,
    pub domains: Vec<String>,
    pub disable_static_images: bool,
    #[serde(rename(deserialize = "minimumCacheTTL"))]
    pub minimum_cache_ttl: u32,
    pub formats: Vec<ImageFormat>,
    #[serde(rename(deserialize = "dangerouslyAllowSVG"))]
    pub dangerously_allow_svg: bool,
    pub content_security_policy: String,
    pub remote_patterns: Vec<RemotePattern>,
    pub unoptimized: bool,
}

impl Default for ImageConfig {
    fn default() -> Self {
        // https://github.com/vercel/next.js/blob/327634eb/packages/next/shared/lib/image-config.ts#L100-L114
        Self {
            device_sizes: vec![640, 750, 828, 1080, 1200, 1920, 2048, 3840],
            image_sizes: vec![16, 32, 48, 64, 96, 128, 256, 384],
            path: "/_next/image".to_string(),
            loader: ImageLoader::Default,
            domains: vec![],
            disable_static_images: false,
            minimum_cache_ttl: 60,
            formats: vec![ImageFormat::Webp],
            dangerously_allow_svg: false,
            content_security_policy: "".to_string(),
            remote_patterns: vec![],
            unoptimized: false,
        }
    }
}

#[derive(Clone, Debug, Ord, PartialOrd, PartialEq, Eq, Serialize, Deserialize, TraceRawVcs)]
#[serde(rename_all = "lowercase")]
pub enum ImageLoader {
    Default,
    Imgix,
    Cloudinary,
    Akamai,
    Custom,
}

#[derive(Clone, Debug, Ord, PartialOrd, PartialEq, Eq, Serialize, Deserialize, TraceRawVcs)]
pub enum ImageFormat {
    #[serde(rename(deserialize = "image/webp"))]
    Webp,
    #[serde(rename(deserialize = "image/avif"))]
    Avif,
}

#[derive(
    Clone, Debug, Default, Ord, PartialOrd, PartialEq, Eq, Serialize, Deserialize, TraceRawVcs,
)]
#[serde(rename_all = "camelCase")]
pub struct RemotePattern {
    pub protocol: Option<RemotePatternProtocal>,
    pub hostname: String,
    pub port: Option<String>,
    pub pathname: Option<String>,
}

#[derive(Clone, Debug, Ord, PartialOrd, PartialEq, Eq, Serialize, Deserialize, TraceRawVcs)]
#[serde(rename_all = "lowercase")]
pub enum RemotePatternProtocal {
    Http,
    Https,
}

#[derive(Clone, Debug, Ord, PartialOrd, PartialEq, Eq, Serialize, Deserialize, TraceRawVcs)]
#[serde(rename_all = "camelCase")]
pub struct ExperimentalConfig {
    pub server_components_external_packages: Option<Vec<String>>,
}

#[derive(Clone, Debug, Ord, PartialOrd, PartialEq, Eq, Serialize, Deserialize, TraceRawVcs)]
#[serde(rename_all = "camelCase")]
pub struct CompilerConfig {
    pub react_remove_properties: Option<bool>,
    pub relay: Option<RelayConfig>,
    pub remove_console: Option<RemoveConsoleConfig>,
}

#[derive(Clone, Debug, Ord, PartialOrd, PartialEq, Eq, Serialize, Deserialize, TraceRawVcs)]
#[serde(untagged, rename_all = "camelCase")]
pub enum ReactRemoveProperties {
    Boolean(bool),
    Config { properties: Option<Vec<String>> },
}

#[derive(Clone, Debug, Ord, PartialOrd, PartialEq, Eq, Serialize, Deserialize, TraceRawVcs)]
#[serde(rename_all = "camelCase")]
pub struct RelayConfig {
    pub src: String,
    pub artifact_directory: Option<String>,
    pub language: Option<RelayLanguage>,
}

#[derive(Clone, Debug, Ord, PartialOrd, PartialEq, Eq, Serialize, Deserialize, TraceRawVcs)]
#[serde(untagged, rename_all = "lowercase")]
pub enum RelayLanguage {
    TypeScript,
    Flow,
    JavaScript,
}

#[derive(Clone, Debug, Ord, PartialOrd, PartialEq, Eq, Serialize, Deserialize, TraceRawVcs)]
#[serde(untagged)]
pub enum RemoveConsoleConfig {
    Boolean(bool),
    Config { exclude: Option<Vec<String>> },
}

#[turbo_tasks::function]
pub async fn load_next_config(
    context: AssetContextVc,
    chunking_context: ChunkingContextVc,
    project_root: FileSystemPathVc,
    intermediate_output_path: FileSystemPathVc,
) -> Result<NextConfigVc> {
    let next_config_mjs_path = project_root.join("next.config.mjs").realpath();
    let next_config_js_path = project_root.join("next.config.js").realpath();
    let config_asset = if matches!(
        &*next_config_mjs_path.get_type().await?,
        FileSystemEntryType::File
    ) {
        Some(SourceAssetVc::new(next_config_mjs_path))
    } else if matches!(
        &*next_config_js_path.get_type().await?,
        FileSystemEntryType::File
    ) {
        Some(SourceAssetVc::new(next_config_js_path))
    } else {
        None
    };

    let chunks = config_asset.map(|config_asset| {
        let config_chunk = EcmascriptModuleAssetVc::new(
            config_asset.into(),
            context,
            Value::new(EcmascriptModuleAssetType::Ecmascript),
            EcmascriptInputTransformsVc::cell(vec![]),
            context.environment(),
        )
        .as_ecmascript_chunk_placeable();
        EcmascriptChunkPlaceablesVc::cell(vec![config_chunk])
    });
    let asset_path = intermediate_output_path.join("load-next-config.js");
    let load_next_config_asset = next_asset(asset_path, "entry/config/next.js");
    let config_value = evaluate(
        load_next_config_asset,
        context,
        if let Some(p) = to_sys_path(project_root)
            .await?
            .and_then(|p| p.to_str().map(|s| s.to_owned()))
        {
            vec![p]
        } else {
            vec![]
        },
        intermediate_output_path,
        chunking_context,
        chunks,
    )
    .await?;
    match &*config_value {
        JavaScriptValue::Value(val) => {
            let next_config: NextConfig = serde_json::from_reader(val.read())?;
            Ok(next_config.cell())
        }
        JavaScriptValue::Stream(_) => {
            unimplemented!("Stream not supported now");
        }
    }
}
