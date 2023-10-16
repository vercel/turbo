use anyhow::Result;
use lightningcss::{rules::import::ImportRule, values::url::Url, visitor::Visit};
use swc_core::common::{
    errors::{Handler, HANDLER},
    source_map::Pos,
    Globals, Spanned, GLOBALS,
};
use turbo_tasks::{Value, Vc};
use turbopack_core::{
    issue::{IssueSeverity, IssueSource, OptionIssueSource},
    reference::{ModuleReference, ModuleReferences},
    reference_type::{CssReferenceSubType, ReferenceType},
    resolve::{
        handle_resolve_error,
        origin::{ResolveOrigin, ResolveOriginExt},
        parse::Request,
        ModuleResolveResult,
    },
    source::Source,
};
use turbopack_swc_utils::emitter::IssueEmitter;

use crate::{
    parse::{parse_css, ParseCssResult},
    references::{
        import::{ImportAssetReference, ImportAttributes},
        url::UrlAssetReference,
    },
    CssInputTransforms, CssModuleAssetType,
};

pub(crate) mod compose;
pub(crate) mod import;
pub(crate) mod internal;
pub(crate) mod url;

#[turbo_tasks::function]
pub async fn analyze_css_stylesheet(
    source: Vc<Box<dyn Source>>,
    origin: Vc<Box<dyn ResolveOrigin>>,
    ty: CssModuleAssetType,
    transforms: Vc<CssInputTransforms>,
) -> Result<Vc<ModuleReferences>> {
    let mut references = Vec::new();

    let parsed = parse_css(source, ty, transforms).await?;

    if let ParseCssResult::Ok {
        stylesheet,
        source_map,
        ..
    } = &*parsed
    {
        let handler = Handler::with_emitter(
            true,
            false,
            Box::new(IssueEmitter {
                source,
                source_map: source_map.clone(),
                title: None,
            }),
        );
        let globals = Globals::new();
        HANDLER.set(&handler, || {
            GLOBALS.set(&globals, || {
                // TODO migrate to effects
                let mut visitor = ModuleReferencesVisitor::new(source, origin, &mut references);
                stylesheet.visit_with_path(&mut visitor, &mut Default::default());
            })
        });
    }
    Ok(Vc::cell(references))
}

struct ModuleReferencesVisitor<'a> {
    source: Vc<Box<dyn Source>>,
    origin: Vc<Box<dyn ResolveOrigin>>,
    references: &'a mut Vec<Vc<Box<dyn ModuleReference>>>,
    is_import: bool,
}

impl<'a> ModuleReferencesVisitor<'a> {
    fn new(
        source: Vc<Box<dyn Source>>,
        origin: Vc<Box<dyn ResolveOrigin>>,
        references: &'a mut Vec<Vc<Box<dyn ModuleReference>>>,
    ) -> Self {
        Self {
            source,
            origin,
            references,
            is_import: false,
        }
    }
}

pub fn as_parent_path(ast_path: &AstNodePath<'_>) -> Vec<AstParentKind> {
    ast_path.iter().map(|n| n.kind()).collect()
}

impl<'a> Visit for ModuleReferencesVisitor<'a> {
    fn visit_import<'ast: 'r, 'r>(&mut self, i: &'ast ImportRule, ast_path: &mut AstNodePath<'r>) {
        let src = &*i.ulr;

        let issue_span = i.href.span();

        self.references.push(Vc::upcast(ImportAssetReference::new(
            self.origin,
            Request::parse(Value::new(src.to_string().into())),
            Vc::cell(as_parent_path(ast_path)),
            ImportAttributes::new_from_prelude(i).into(),
            IssueSource::from_byte_offset(
                Vc::upcast(self.source),
                issue_span.lo.to_usize(),
                issue_span.hi.to_usize(),
            ),
        )));

        self.is_import = true;
        i.visit_children_with_path(self, ast_path);
        self.is_import = false;
    }

    fn visit_url<'ast: 'r, 'r>(&mut self, u: &'ast Url, ast_path: &mut AstNodePath<'r>) {
        if self.is_import {
            return u.visit_children_with_path(self, ast_path);
        }

        let src = &*u.url;

        // ignore internal urls like `url(#noiseFilter)`
        // ignore server-relative urls like `url(/foo)`
        if !matches!(src.bytes().next(), Some(b'#') | Some(b'/')) {
            let issue_span = u.span;
            self.references.push(Vc::upcast(UrlAssetReference::new(
                self.origin,
                Request::parse(Value::new(src.to_string().into())),
                Vc::cell(as_parent_path(ast_path)),
                IssueSource::from_byte_offset(
                    Vc::upcast(self.source),
                    issue_span.lo.to_usize(),
                    issue_span.hi.to_usize(),
                ),
            )));
        }

        u.visit_children_with_path(self, ast_path);
    }
}

#[turbo_tasks::function]
pub async fn css_resolve(
    origin: Vc<Box<dyn ResolveOrigin>>,
    request: Vc<Request>,
    ty: Value<CssReferenceSubType>,
    issue_source: Vc<OptionIssueSource>,
) -> Result<Vc<ModuleResolveResult>> {
    let ty = Value::new(ReferenceType::Css(ty.into_value()));
    let options = origin.resolve_options(ty.clone());
    let result = origin.resolve_asset(request, options, ty.clone());

    handle_resolve_error(
        result,
        ty,
        origin.origin_path(),
        request,
        options,
        issue_source,
        IssueSeverity::Error.cell(),
    )
    .await
}

// TODO enable serialization
#[turbo_tasks::value(transparent, serialization = "none")]
pub struct AstPath(#[turbo_tasks(trace_ignore)] Vec<AstParentKind>);
