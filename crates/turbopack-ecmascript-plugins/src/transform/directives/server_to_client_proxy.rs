use swc_core::{
    common::DUMMY_SP,
    ecma::{
        ast::{
            ImportDecl, ImportDefaultSpecifier, ImportSpecifier,
            Module, ModuleDecl, ModuleItem, Program,
        },
        utils::private_ident,
    },
    quote,
};
use turbopack_ecmascript::{annotations::with_clause, TURBOPACK_HELPER};

pub fn create_proxy_module(transition_name: &str, target_import: &str) -> Program {
    let ident = private_ident!("proxy");
    Program::Module(Module {
        body: vec![
            ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
                specifiers: vec![ImportSpecifier::Default(ImportDefaultSpecifier {
                    local: ident.clone(),
                    span: DUMMY_SP,
                })],
                src: Box::new(target_import.into()),
                type_only: false,
                with: Some(with_clause(&[
                    (TURBOPACK_HELPER.as_str(), "true"),
                    ("transition", transition_name),
                ])),
                span: DUMMY_SP,
            })),
            ModuleItem::Stmt(quote!(
                "__turbopack_export_namespace__($proxy);" as Stmt,
                proxy = ident,
            )),
        ],
        shebang: None,
        span: DUMMY_SP,
    })
}
