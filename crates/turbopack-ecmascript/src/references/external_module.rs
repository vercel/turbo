use std::io::Write;

use anyhow::{Context, Result};
use turbo_tasks::Vc;
use turbo_tasks_fs::{rope::RopeBuilder, FileContent, FileSystem, VirtualFileSystem};
use turbopack_core::{
    asset::{Asset, AssetContent},
    chunk::{AsyncModuleInfo, ChunkItem, ChunkType, ChunkableModule, ChunkingContext},
    ident::AssetIdent,
    module::Module,
    reference::ModuleReferences,
    resolve::ExternalType,
};

use crate::{
    chunk::{
        EcmascriptChunkItem, EcmascriptChunkItemContent, EcmascriptChunkPlaceable,
        EcmascriptChunkType, EcmascriptChunkingContext, EcmascriptExports,
    },
    references::async_module::{AsyncModule, OptionAsyncModule},
    utils::StringifyJs,
    EcmascriptModuleContent,
};

#[turbo_tasks::function]
fn layer() -> Vc<String> {
    Vc::cell("external".to_string())
}

#[turbo_tasks::value]
pub struct ExternalModuleAsset {
    pub request: String,
    pub external_type: ExternalType,
    pub import_externals: bool,
}

impl ExternalModuleAsset {
    pub fn is_async(&self) -> bool {
        self.external_type == ExternalType::EcmaScriptModule && self.import_externals
    }
}

#[turbo_tasks::value_impl]
impl ExternalModuleAsset {
    #[turbo_tasks::function]
    pub fn new(request: String, external_type: ExternalType, import_externals: bool) -> Vc<Self> {
        Self::cell(ExternalModuleAsset {
            request,
            external_type,
            import_externals,
        })
    }

    #[turbo_tasks::function]
    pub fn is_async_vc(&self) -> Vc<bool> {
        Vc::cell(self.is_async())
    }

    #[turbo_tasks::function]
    pub fn content(&self) -> Vc<EcmascriptModuleContent> {
        let mut code = RopeBuilder::default();

        if self.is_async() {
            writeln!(
                code,
                "const mod = await __turbopack_external_import__({});",
                StringifyJs(&self.request)
            )
            .unwrap();
        } else {
            writeln!(
                code,
                "const mod = __turbopack_external_require__({});",
                StringifyJs(&self.request)
            )
            .unwrap();
        }

        writeln!(code).unwrap();
        writeln!(code, "__turbopack_export_namespace__(mod);").unwrap();

        EcmascriptModuleContent {
            inner_code: code.build(),
            source_map: None,
            is_esm: true,
        }
        .cell()
    }
}

#[turbo_tasks::value_impl]
impl Module for ExternalModuleAsset {
    #[turbo_tasks::function]
    fn ident(&self) -> Vc<AssetIdent> {
        let fs = VirtualFileSystem::new_with_name("external-assets".to_string());
        let path = fs.root().join(self.request.clone());

        AssetIdent::from_path(path).with_layer(layer())
    }
}

#[turbo_tasks::value_impl]
impl Asset for ExternalModuleAsset {
    #[turbo_tasks::function]
    fn content(self: Vc<Self>) -> Vc<AssetContent> {
        AssetContent::file(FileContent::NotFound.cell())
    }
}

#[turbo_tasks::value_impl]
impl ChunkableModule for ExternalModuleAsset {
    #[turbo_tasks::function]
    async fn as_chunk_item(
        self: Vc<Self>,
        chunking_context: Vc<Box<dyn ChunkingContext>>,
    ) -> Result<Vc<Box<dyn ChunkItem>>> {
        let chunking_context =
            Vc::try_resolve_downcast::<Box<dyn EcmascriptChunkingContext>>(chunking_context)
                .await?
                .context(
                    "chunking context must impl EcmascriptChunkingContext to use \
                     WebAssemblyModuleAsset",
                )?;

        Ok(Vc::upcast(
            ExternalModuleChunkItem {
                module: self,
                chunking_context,
            }
            .cell(),
        ))
    }
}

#[turbo_tasks::value_impl]
impl EcmascriptChunkPlaceable for ExternalModuleAsset {
    #[turbo_tasks::function]
    fn get_exports(self: Vc<Self>) -> Vc<EcmascriptExports> {
        EcmascriptExports::DynamicNamespace.cell()
    }

    #[turbo_tasks::function]
    fn get_async_module(&self) -> Vc<OptionAsyncModule> {
        Vc::cell(if self.is_async() {
            Some(
                AsyncModule {
                    references: ModuleReferences::empty(),
                    has_top_level_await: self.is_async(),
                    import_externals: self.import_externals,
                }
                .cell(),
            )
        } else {
            None
        })
    }
}

#[turbo_tasks::value]
pub struct ExternalModuleChunkItem {
    module: Vc<ExternalModuleAsset>,
    chunking_context: Vc<Box<dyn EcmascriptChunkingContext>>,
}

#[turbo_tasks::value_impl]
impl ChunkItem for ExternalModuleChunkItem {
    #[turbo_tasks::function]
    fn asset_ident(&self) -> Vc<AssetIdent> {
        self.module.ident()
    }

    #[turbo_tasks::function]
    fn references(&self) -> Vc<ModuleReferences> {
        self.module.references()
    }

    #[turbo_tasks::function]
    fn ty(self: Vc<Self>) -> Vc<Box<dyn ChunkType>> {
        Vc::upcast(Vc::<EcmascriptChunkType>::default())
    }

    #[turbo_tasks::function]
    fn module(&self) -> Vc<Box<dyn Module>> {
        Vc::upcast(self.module)
    }

    #[turbo_tasks::function]
    fn chunking_context(&self) -> Vc<Box<dyn ChunkingContext>> {
        Vc::upcast(self.chunking_context)
    }

    #[turbo_tasks::function]
    fn is_self_async(&self) -> Vc<bool> {
        Vc::cell(true)
    }
}

#[turbo_tasks::value_impl]
impl EcmascriptChunkItem for ExternalModuleChunkItem {
    #[turbo_tasks::function]
    fn chunking_context(&self) -> Vc<Box<dyn EcmascriptChunkingContext>> {
        self.chunking_context
    }

    #[turbo_tasks::function]
    fn content(self: Vc<Self>) -> Vc<EcmascriptChunkItemContent> {
        panic!("content() should not be called");
    }

    #[turbo_tasks::function]
    async fn content_with_async_module_info(
        &self,
        async_module_info: Option<Vc<AsyncModuleInfo>>,
    ) -> Result<Vc<EcmascriptChunkItemContent>> {
        let async_module_options = self
            .module
            .get_async_module()
            .module_options(async_module_info);

        Ok(EcmascriptChunkItemContent::new(
            self.module.content(),
            self.chunking_context,
            async_module_options,
        ))
    }
}
