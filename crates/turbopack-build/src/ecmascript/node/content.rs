use std::io::Write;

use anyhow::Result;
use indoc::writedoc;
use turbo_tasks::{TryJoinIterExt, Vc};
use turbo_tasks_fs::File;
use turbopack_core::{
    asset::AssetContent,
    chunk::ChunkItemExt,
    code_builder::{Code, CodeBuilder},
    output::OutputAsset,
    source_map::{GenerateSourceMap, OptionSourceMap},
};
use turbopack_ecmascript::{
    chunk::{EcmascriptChunkContent, EcmascriptChunkItemExt},
    utils::StringifyJs,
};

use super::chunk::EcmascriptBuildNodeChunk;
use crate::{chunking_context::MinifyType, ecmascript::minify::minify, BuildChunkingContext};

#[turbo_tasks::value]
pub(super) struct EcmascriptBuildNodeChunkContent {
    pub(super) content: Vc<EcmascriptChunkContent>,
    pub(super) chunking_context: Vc<BuildChunkingContext>,
    pub(super) chunk: Vc<EcmascriptBuildNodeChunk>,
}

#[turbo_tasks::value_impl]
impl EcmascriptBuildNodeChunkContent {
    #[turbo_tasks::function]
    pub(crate) async fn new(
        chunking_context: Vc<BuildChunkingContext>,
        chunk: Vc<EcmascriptBuildNodeChunk>,
        content: Vc<EcmascriptChunkContent>,
    ) -> Result<Vc<Self>> {
        Ok(EcmascriptBuildNodeChunkContent {
            content,
            chunking_context,
            chunk,
        }
        .cell())
    }
}

#[turbo_tasks::value_impl]
impl EcmascriptBuildNodeChunkContent {
    #[turbo_tasks::function]
    async fn code(self: Vc<Self>) -> Result<Vc<Code>> {
        let this = self.await?;
        let chunk_path_vc = this.chunk.ident().path();
        let chunk_path = chunk_path_vc.await?;

        let mut code = CodeBuilder::default();

        writedoc!(
            code,
            r#"
                module.exports = {{

            "#,
        )?;

        let content = this.content.await?;
        let chunk_group_root = content.chunk_group_root;
        for (id, item_code) in content
            .chunk_items
            .iter()
            .map(|(chunk_item, _)| async move {
                Ok((
                    chunk_item.id().await?,
                    chunk_item.code(chunk_group_root).await?,
                ))
            })
            .try_join()
            .await?
        {
            write!(code, "{}: ", StringifyJs(&id))?;
            code.push_code(&item_code);
            writeln!(code, ",")?;
        }

        write!(code, "\n}};")?;

        if code.has_source_map() {
            let filename = chunk_path.file_name();
            write!(code, "\n\n//# sourceMappingURL={}.map", filename)?;
        }

        let code = code.build().cell();
        if matches!(
            this.chunking_context.await?.minify_type(),
            MinifyType::Minify
        ) {
            return Ok(minify(chunk_path_vc, code));
        }

        Ok(code)
    }

    #[turbo_tasks::function]
    pub async fn content(self: Vc<Self>) -> Result<Vc<AssetContent>> {
        let code = self.code().await?;
        Ok(AssetContent::file(
            File::from(code.source_code().clone()).into(),
        ))
    }
}

#[turbo_tasks::value_impl]
impl GenerateSourceMap for EcmascriptBuildNodeChunkContent {
    #[turbo_tasks::function]
    fn generate_source_map(self: Vc<Self>) -> Vc<OptionSourceMap> {
        self.code().generate_source_map()
    }
}
