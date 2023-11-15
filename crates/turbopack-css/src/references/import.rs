use anyhow::Result;
use lightningcss::{
    printer::Printer,
    rules::{
        import::ImportRule,
        layer::{LayerName, LayerStatementRule},
        media::MediaRule,
        supports::{SupportsCondition, SupportsRule},
        unknown::UnknownAtRule,
        CssRule,
    },
    stylesheet::PrinterOptions,
    traits::ToCss,
};
use swc_core::common::DUMMY_SP;
use turbo_tasks::{Value, ValueToString, Vc};
use turbopack_core::{
    chunk::{ChunkableModuleReference, ChunkingContext},
    issue::IssueSource,
    reference::ModuleReference,
    reference_type::CssReferenceSubType,
    resolve::{origin::ResolveOrigin, parse::Request, ModuleResolveResult},
};

use crate::{
    chunk::CssImport,
    code_gen::{CodeGenerateable, CodeGeneration},
    references::{css_resolve, AstPath},
};

#[turbo_tasks::value(into = "new")]
pub struct ImportAttributes {
    #[turbo_tasks(trace_ignore)]
    pub layer_name: Option<LayerStatementRule<'static>>,
    #[turbo_tasks(trace_ignore)]
    pub supports: Option<SupportsRule<'static>>,
    #[turbo_tasks(trace_ignore)]
    pub media: Option<Vec<MediaRule<'static>>>,
}

impl ImportAttributes {
    pub fn new_from_prelude(prelude: &ImportRule<'static>) -> Self {
        let layer_name = prelude.layer.as_ref().and_then(|l| match l {
            box LayerName::Ident(_) => LayerName(Default::default()),
            box LayerName::Function(f) => {
                assert_eq!(f.value.len(), 1);
                assert!(matches!(&f.value[0], ComponentValue::LayerName(_)));
                if let ComponentValue::LayerName(layer_name) = &f.value[0] {
                    *layer_name.clone()
                } else {
                    unreachable!()
                }
            }
        });

        let (supports, media) = prelude
            .import_conditions
            .as_ref()
            .map(|c| {
                let supports = if let Some(supports) = &c.supports {
                    let v = supports.value.iter().find(|v| {
                        matches!(
                            v,
                            ComponentValue::SupportsCondition(..) | ComponentValue::Declaration(..)
                        )
                    });

                    if let Some(supports) = v {
                        match &supports {
                            ComponentValue::SupportsCondition(s) => Some(*s.clone()),
                            ComponentValue::Declaration(d) => Some(SupportsCondition {
                                span: DUMMY_SP,
                                conditions: vec![SupportsCondition::SupportsInParens(
                                    SupportsCondition::Feature(SupportsCondition::Declaration(
                                        d.clone(),
                                    )),
                                )],
                            }),
                            _ => None,
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

                let media = c.media.as_ref().map(|m| m.queries.clone());

                (supports, media)
            })
            .unwrap_or_else(|| (None, None));

        Self {
            layer_name,
            supports,
            media,
        }
    }

    pub fn print_block(&self) -> Result<(String, String)> {
        fn token(token: Token) -> TokenAndSpan {
            TokenAndSpan {
                span: DUMMY_SP,
                token,
            }
        }

        // something random that's never gonna be in real css
        // Box::new(ListOfComponentValues {
        //     span: DUMMY_SP,
        //     children: vec![ComponentValue::PreservedToken(Box::new(token(
        //         Token::String {
        //             value: Default::default(),
        //             raw: r#""""__turbopack_placeholder__""""#.into(),
        //         },
        //     )))],
        // })
        let mut rule = CssRule::Unknown(UnknownAtRule {});

        fn at_rule(
            name: &str,
            prelude: CssRule<'static>,
            inner_rule: CssRule<'static>,
        ) -> CssRule<'static> {
            Rule::AtRule(Box::new(AtRule {
                span: DUMMY_SP,
                name: AtRuleName::Ident(Ident {
                    span: DUMMY_SP,
                    value: name.into(),
                    raw: None,
                }),
                prelude: Some(Box::new(prelude)),
                block: Some(SimpleBlock {
                    span: DUMMY_SP,
                    name: token(Token::LBrace),
                    value: vec![ComponentValue::from(inner_rule)],
                }),
            }))
        }

        if let Some(media) = &self.media {
            rule = at_rule(
                "media",
                CssRule::Media(MediaRule {
                    span: DUMMY_SP,
                    queries: media.clone(),
                }),
                rule,
            );
        }
        if let Some(supports) = &self.supports {
            rule = at_rule("supports", CssRule::Supports(supports.clone()), rule);
        }
        if let Some(layer_name) = &self.layer_name {
            rule = at_rule("layer", CssRule::LayerStatement(layer_name.clone()), rule);
        }

        let mut output = String::new();
        let mut printer = Printer::new(&mut output, PrinterOptions::default());
        rule.to_css(&mut printer)?;

        let (open, close) = output
            .split_once(r#""""__turbopack_placeholder__""""#)
            .unwrap();

        Ok((open.trim().into(), close.trim().into()))
    }
}

#[turbo_tasks::value]
#[derive(Hash, Debug)]
pub struct ImportAssetReference {
    pub origin: Vc<Box<dyn ResolveOrigin>>,
    pub request: Vc<Request>,
    pub path: Vc<AstPath>,
    pub attributes: Vc<ImportAttributes>,
    pub issue_source: Vc<IssueSource>,
}

#[turbo_tasks::value_impl]
impl ImportAssetReference {
    #[turbo_tasks::function]
    pub fn new(
        origin: Vc<Box<dyn ResolveOrigin>>,
        request: Vc<Request>,
        path: Vc<AstPath>,
        attributes: Vc<ImportAttributes>,
        issue_source: Vc<IssueSource>,
    ) -> Vc<Self> {
        Self::cell(ImportAssetReference {
            origin,
            request,
            path,
            attributes,
            issue_source,
        })
    }
}

#[turbo_tasks::value_impl]
impl ModuleReference for ImportAssetReference {
    #[turbo_tasks::function]
    fn resolve_reference(&self) -> Vc<ModuleResolveResult> {
        css_resolve(
            self.origin,
            self.request,
            Value::new(CssReferenceSubType::AtImport),
            Some(self.issue_source),
        )
    }
}

#[turbo_tasks::value_impl]
impl ValueToString for ImportAssetReference {
    #[turbo_tasks::function]
    async fn to_string(&self) -> Result<Vc<String>> {
        Ok(Vc::cell(format!(
            "import(url) {}",
            self.request.to_string().await?,
        )))
    }
}

#[turbo_tasks::value_impl]
impl CodeGenerateable for ImportAssetReference {
    #[turbo_tasks::function]
    async fn code_generation(
        self: Vc<Self>,
        _context: Vc<Box<dyn ChunkingContext>>,
    ) -> Result<Vc<CodeGeneration>> {
        let this = &*self.await?;
        let mut imports = vec![];
        if let Request::Uri {
            protocol,
            remainder,
        } = &*this.request.await?
        {
            imports.push(CssImport::External(Vc::cell(format!(
                "{}{}",
                protocol, remainder
            ))))
        }

        Ok(CodeGeneration {
            visitors: vec![],
            imports,
        }
        .into())
    }
}

#[turbo_tasks::value_impl]
impl ChunkableModuleReference for ImportAssetReference {}
