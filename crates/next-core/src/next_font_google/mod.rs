use anyhow::{anyhow, Context, Result};
use indexmap::IndexMap;
use turbo_tasks::{
    primitives::{OptionStringVc, StringVc},
    TryJoinIterExt,
};
use turbo_tasks_fetch::fetch;
use turbo_tasks_fs::{FileContent, FileSystemPathVc};
use turbopack_core::{
    resolve::{
        options::{
            ImportMapResult, ImportMapResultVc, ImportMapping, ImportMappingReplacement,
            ImportMappingReplacementVc, ImportMappingVc,
        },
        parse::{Request, RequestVc},
        ResolveResult,
    },
    virtual_asset::VirtualAssetVc,
};

use crate::{
    embed_js::attached_next_js_package_path,
    next_font_google::{
        options::{options_from_request, FontDataEntry},
        request::NextFontRequest,
        util::{extract_font_urls, get_font_axes, get_stylesheet_url},
    },
};

mod options;
pub(crate) mod request;
mod util;

type FontData = IndexMap<String, FontDataEntry>;

#[turbo_tasks::value(shared)]
pub struct NextFontGoogleReplacer {
    project_path: FileSystemPathVc,
}

#[turbo_tasks::value_impl]
impl NextFontGoogleReplacerVc {
    #[turbo_tasks::function]
    pub fn new(project_path: FileSystemPathVc) -> Self {
        Self::cell(NextFontGoogleReplacer { project_path })
    }
}

#[turbo_tasks::value_impl]
impl ImportMappingReplacement for NextFontGoogleReplacer {
    #[turbo_tasks::function]
    fn replace(&self, _capture: &str) -> ImportMappingVc {
        ImportMapping::Ignore.into()
    }

    #[turbo_tasks::function]
    async fn result(&self, request: RequestVc) -> Result<ImportMapResultVc> {
        let request = &*request.await?;
        if let Request::Module {
            module: _,
            path: _,
            query,
        } = request
        {
            let q = &*query.await?;

            let js_asset = VirtualAssetVc::new(
                attached_next_js_package_path(self.project_path)
                    .join("internal/font/google/inter.js"),
                FileContent::Content(
                    format!(
                        r#"
                    import cssModule from "@vercel/turbopack-next/internal/font/google/cssmodule.module.css?{}";
                    export default {{
                        className: cssModule.className
                    }};
                "#,
                        // Pass along whichever options we received to the css handler
                        qstring::QString::new(q.as_ref().unwrap().iter().collect())
                    )
                    .into(),
                )
                .into(),
            );

            return Ok(ImportMapResult::Result(
                ResolveResult::Single(js_asset.into(), vec![]).into(),
            )
            .into());
        };

        Ok(ImportMapResult::NoEntry.into())
    }
}

#[turbo_tasks::value(shared)]
pub struct NextFontGoogleCssModuleReplacer {
    project_path: FileSystemPathVc,
}

#[turbo_tasks::value_impl]
impl NextFontGoogleCssModuleReplacerVc {
    #[turbo_tasks::function]
    pub fn new(project_path: FileSystemPathVc) -> Self {
        Self::cell(NextFontGoogleCssModuleReplacer { project_path })
    }
}

#[turbo_tasks::value_impl]
impl ImportMappingReplacement for NextFontGoogleCssModuleReplacer {
    #[turbo_tasks::function]
    fn replace(&self, _capture: &str) -> ImportMappingVc {
        ImportMapping::Ignore.into()
    }

    #[turbo_tasks::function]
    async fn result(&self, request: RequestVc) -> Result<ImportMapResultVc> {
        let request = &*request.await?;
        if let Request::Module {
            module: _,
            path: _,
            query,
        } = request
        {
            let query_map = &*query.await?;
            // These are invariants from the next/font swc transform. Regular errors instead
            // of Issues should be okay.
            let mut query_map = query_map
                .clone()
                .context("@next/font/google queries must exist")?;

            assert_eq!(
                query_map.len(),
                1,
                "@next/font/google queries must only have one entry"
            );

            let Some((json, _)) = query_map.pop() else {
                bail!("Expected one entry");
            };

            let request: NextFontRequest = serde_json::from_str(&json)?;
            let font_data: FontData = serde_json::from_str(include_str!("font-data.json"))?;
            let options = options_from_request(&request, &font_data)?;
            let url = get_stylesheet_url(
                &options.font_family,
                &get_font_axes(
                    &font_data,
                    &options.font_family,
                    &options.weights,
                    &options.styles,
                    &options.selected_variable_axes,
                )?,
                &options.display,
            )?;

            let res = fetch(
                StringVc::cell(url),
                OptionStringVc::cell(Some(
                    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, \
                     like Gecko) Chrome/104.0.0.0 Safari/537.36"
                        .to_owned(),
                )),
            );
            let font_urls = extract_font_urls(
                &res.await?.body.to_string().await?,
                options.subsets.as_ref(),
                options.preload,
            )?;

            let mut requests = vec![];
            for font_url in &font_urls {
                requests.push(fetch(
                    StringVc::cell(font_url.url.to_owned()),
                    OptionStringVc::cell(None),
                ));
            }

            let css_asset = VirtualAssetVc::new(
                attached_next_js_package_path(self.project_path)
                    .join("internal/font/google/cssmodule.module.css"),
                FileContent::Content(
                    r#".className {
                            color: blue;
                        }
                "#
                    .into(),
                )
                .into(),
            );

            return Ok(ImportMapResult::Result(
                ResolveResult::Single(css_asset.into(), vec![]).into(),
            )
            .into());
        };

        Ok(ImportMapResult::NoEntry.into())
    }
}
