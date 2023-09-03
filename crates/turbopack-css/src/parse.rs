use std::sync::Arc;

use anyhow::Result;
use indexmap::IndexMap;
use lightningcss::{
    css_modules::{Pattern, Segment},
    stylesheet::{ParserOptions, StyleSheet},
};
use once_cell::sync::Lazy;
use regex::Regex;
use smallvec::smallvec;
use swc_core::{
    common::{
        errors::Handler, source_map::SourceMapGenConfig, BytePos, FileName, LineCol, SourceMap,
    },
    css::{
        ast::Stylesheet,
        modules::{CssClassName, TransformConfig},
        parser::{parse_file, parser::ParserConfig},
    },
    ecma::atoms::JsWord,
};
use turbo_tasks::{ValueToString, Vc};
use turbo_tasks_fs::{FileContent, FileSystemPath};
use turbopack_core::{
    asset::{Asset, AssetContent},
    source::Source,
    source_map::{GenerateSourceMap, OptionSourceMap},
    SOURCE_MAP_ROOT_NAME,
};
use turbopack_swc_utils::emitter::IssueEmitter;

use crate::{
    transform::{CssInputTransform, CssInputTransforms, TransformContext},
    CssModuleAssetType,
};

// Capture up until the first "."
static BASENAME_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[^.]*").unwrap());

#[turbo_tasks::value(shared, serialization = "none", eq = "manual")]
pub enum ParseCssResult {
    Ok {
        #[turbo_tasks(trace_ignore)]
        stylesheet: Stylesheet,
        #[turbo_tasks(debug_ignore, trace_ignore)]
        source_map: Arc<SourceMap>,
        #[turbo_tasks(debug_ignore, trace_ignore)]
        imports: Vec<JsWord>,
    },
    Unparseable,
    NotFound,
}

impl PartialEq for ParseCssResult {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Ok { .. }, Self::Ok { .. }) => false,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

#[turbo_tasks::value(shared, serialization = "none", eq = "manual")]
pub struct ParseCssResultSourceMap {
    #[turbo_tasks(debug_ignore, trace_ignore)]
    source_map: Arc<SourceMap>,

    /// The position mappings that can generate a real source map given a (SWC)
    /// SourceMap.
    #[turbo_tasks(debug_ignore, trace_ignore)]
    mappings: parcel_sourcemap::SourceMap,
}

impl PartialEq for ParseCssResultSourceMap {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.source_map, &other.source_map) && self.mappings == other.mappings
    }
}

impl ParseCssResultSourceMap {
    pub fn new(source_map: Arc<SourceMap>, mappings: parcel_sourcemap::SourceMap) -> Self {
        ParseCssResultSourceMap {
            source_map,
            mappings,
        }
    }
}

#[turbo_tasks::value_impl]
impl GenerateSourceMap for ParseCssResultSourceMap {
    #[turbo_tasks::function]
    fn generate_source_map(&self) -> Vc<OptionSourceMap> {
        let map = self.source_map.build_source_map_with_config(
            &self.mappings,
            None,
            InlineSourcesContentConfig {},
        );
        Vc::cell(Some(
            turbopack_core::source_map::SourceMap::new_regular(map).cell(),
        ))
    }
}

/// A config to generate a source map which includes the source content of every
/// source file. SWC doesn't inline sources content by default when generating a
/// sourcemap, so we need to provide a custom config to do it.
struct InlineSourcesContentConfig {}

impl SourceMapGenConfig for InlineSourcesContentConfig {
    fn file_name_to_source(&self, f: &FileName) -> String {
        match f {
            FileName::Custom(s) => format!("/{SOURCE_MAP_ROOT_NAME}/{s}"),
            _ => f.to_string(),
        }
    }

    fn inline_sources_content(&self, _f: &FileName) -> bool {
        true
    }
}

#[turbo_tasks::function]
pub async fn parse_css(
    source: Vc<Box<dyn Source>>,
    ty: CssModuleAssetType,
    transforms: Vc<CssInputTransforms>,
) -> Result<Vc<ParseCssResult>> {
    let content = source.content();
    let fs_path = &*source.ident().path().await?;
    let ident_str = &*source.ident().to_string().await?;
    Ok(match &*content.await? {
        AssetContent::Redirect { .. } => ParseCssResult::Unparseable.cell(),
        AssetContent::File(file) => match &*file.await? {
            FileContent::NotFound => ParseCssResult::NotFound.cell(),
            FileContent::Content(file) => match file.content().to_str() {
                Err(_err) => ParseCssResult::Unparseable.cell(),
                Ok(string) => {
                    let transforms = &*transforms.await?;
                    parse_content(
                        string.into_owned(),
                        fs_path,
                        ident_str,
                        source,
                        ty,
                        transforms,
                    )
                    .await?
                }
            },
        },
    })
}

async fn parse_content(
    string: String,
    fs_path: &FileSystemPath,
    ident_str: &str,
    source: Vc<Box<dyn Source>>,
    ty: CssModuleAssetType,
    transforms: &[CssInputTransform],
) -> Result<Vc<ParseCssResult>> {
    let source_map: Arc<SourceMap> = Default::default();
    let handler = Handler::with_emitter(
        true,
        false,
        Box::new(IssueEmitter {
            source,
            source_map: source_map.clone(),
            title: Some("Parsing css source code failed".to_string()),
        }),
    );

    let fm = source_map.new_source_file(FileName::Custom(ident_str.to_string()), string);

    let config = ParserOptions {
        css_modules: match ty {
            CssModuleAssetType::Module => Some(lightningcss::css_modules::Config {
                pattern: Pattern {
                    segments: smallvec![
                        Segment::Local,
                        Segment::Literal("__"),
                        Segment::Name,
                        Segment::Literal("__"),
                        Segment::Hash,
                    ],
                },
                dashed_idents: false,
            }),

            _ => None,
        },
        filename: ident_str.to_string(),
        ..Default::default()
    };

    let mut parsed_stylesheet = match StyleSheet::parse(&string, config) {
        Ok(stylesheet) => stylesheet,
        Err(e) => {
            // TODO(kdy1): Report errors
            // e.to_diagnostics(&handler).emit();
            return Ok(ParseCssResult::Unparseable.into());
        }
    };

    let mut has_errors = false;
    for e in errors {
        e.to_diagnostics(&handler).emit();
        has_errors = true
    }

    if has_errors {
        return Ok(ParseCssResult::Unparseable.into());
    }

    let transform_context = TransformContext {
        source_map: &source_map,
    };
    for transform in transforms.iter() {
        transform
            .apply(&mut parsed_stylesheet, &transform_context)
            .await?;
    }

    let imports = match ty {
        CssModuleAssetType::Default => Default::default(),
        CssModuleAssetType::Module => {
            swc_core::css::modules::imports::analyze_imports(&parsed_stylesheet)
        }
    };

    Ok(ParseCssResult::Ok {
        stylesheet: parsed_stylesheet,
        source_map,
        imports,
    }
    .into())
}

/// Trait to be implemented by assets which can be parsed as CSS.
#[turbo_tasks::value_trait]
pub trait ParseCss {
    /// Returns the parsed css.
    fn parse_css(self: Vc<Self>) -> Vc<ParseCssResult>;
}
