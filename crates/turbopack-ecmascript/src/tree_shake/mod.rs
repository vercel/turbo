use anyhow::Result;
use fxhash::FxHashMap;
use indexmap::IndexSet;
use swc_core::ecma::ast::{Id, Module};
use turbo_tasks::{primitives::StringVc, ValueToString, ValueToStringVc};
use turbo_tasks_fs::FileSystemPathVc;
use turbopack_core::{
    asset::{Asset, AssetContentVc, AssetVc},
    chunk::{
        ChunkItem, ChunkItemVc, ChunkVc, ChunkableAsset, ChunkableAssetVc, ChunkingContextVc,
        ModuleIdVc,
    },
    reference::{
        AssetReference, AssetReferenceVc, AssetReferences, AssetReferencesVc, SingleAssetReference,
    },
    resolve::{ResolveResult, ResolveResultVc},
    version::VersionedContentVc,
};

use self::graph::{DepGraph, ItemData, ItemId, ItemIdKind};
use crate::{
    chunk::{EcmascriptChunkItem, EcmascriptChunkItemContentVc, EcmascriptChunkItemVc},
    EcmascriptModuleAssetVc,
};

mod graph;
pub mod merge;
#[cfg(test)]
mod tests;
mod util;

pub struct Analyzer<'a> {
    g: &'a mut DepGraph,
    item_ids: &'a Vec<ItemId>,
    items: &'a mut FxHashMap<ItemId, ItemData>,

    last_side_effect: Option<ItemId>,
    last_side_effects: Vec<ItemId>,

    vars: FxHashMap<Id, VarState>,
}

#[derive(Debug, Default)]
struct VarState {
    /// The module items that might triggered side effects on that variable.
    /// We also store if this is a `const` write, so no further change will
    /// happen to this var.
    last_writes: Vec<ItemId>,
    /// The module items that might read that variable.
    last_reads: Vec<ItemId>,
}

impl Analyzer<'_> {
    pub fn analyze(module: &Module) -> DepGraph {
        let mut g = DepGraph::default();
        let (item_ids, mut items) = g.init(module);

        let mut analyzer = Analyzer {
            g: &mut g,
            item_ids: &item_ids,
            items: &mut items,
            last_side_effect: Default::default(),
            last_side_effects: Default::default(),
            vars: Default::default(),
        };

        let eventual_ids = analyzer.hoist_vars_and_bindings(module);

        analyzer.evaluate_immediate(module, &eventual_ids);

        analyzer.evaluate_eventual(module);

        analyzer.handle_exports(module);

        g
    }

    /// Phase 1: Hoisted Variables and Bindings
    ///
    ///
    /// Returns all (EVENTUAL_READ/WRITE_VARS) in the module.
    fn hoist_vars_and_bindings(&mut self, module: &Module) -> IndexSet<Id> {
        let mut eventual_ids = IndexSet::default();

        for item_id in self.item_ids.iter() {
            if let Some(item) = self.items.get(item_id) {
                eventual_ids.extend(item.eventual_read_vars.iter().cloned());
                eventual_ids.extend(item.eventual_write_vars.iter().cloned());

                if item.is_hoisted && item.side_effects {
                    if let Some(last) = self.last_side_effect.take() {
                        self.g.add_strong_dep(item_id, &last)
                    }

                    self.last_side_effect = Some(item_id.clone());
                    self.last_side_effects.push(item_id.clone());
                }

                for id in item.var_decls.iter() {
                    let state = self.vars.entry(id.clone()).or_default();

                    if item.is_hoisted {
                        state.last_writes.push(item_id.clone());
                    } else {
                        // TODO: Create a fake module item
                        // state.last_writes.push(item_id.clone());
                    }
                }
            }
        }

        eventual_ids
    }

    /// Phase 2: Immediate evaluation
    fn evaluate_immediate(&mut self, module: &Module, eventual_ids: &IndexSet<Id>) {
        for item_id in self.item_ids.iter() {
            if let Some(item) = self.items.get(item_id) {
                // Ignore HOISTED module items, they have been processed in phase 1 already.
                if item.is_hoisted {
                    continue;
                }

                let mut items_to_remove_from_last_reads = FxHashMap::<_, Vec<_>>::default();

                // For each var in READ_VARS:
                for id in item.read_vars.iter() {
                    // Create a strong dependency to all module items listed in LAST_WRITES for that
                    // var.

                    // (the write need to be executed before this read)
                    if let Some(state) = self.vars.get(id) {
                        for last_write in state.last_writes.iter() {
                            self.g.add_strong_dep(item_id, last_write);

                            items_to_remove_from_last_reads
                                .entry(id.clone())
                                .or_default()
                                .push(last_write.clone());
                        }
                    }
                }

                // For each var in WRITE_VARS:
                for id in item.write_vars.iter() {
                    // Create a weak dependency to all module items listed in
                    // LAST_READS for that var.

                    // (the read need to be executed before this write, when
                    // it’s needed)

                    if let Some(state) = self.vars.get(id) {
                        for last_read in state.last_reads.iter() {
                            self.g.add_weak_dep(item_id, last_read);
                        }
                    }
                }

                if item.side_effects {
                    // Create a strong dependency to LAST_SIDE_EFFECT.

                    if let Some(last) = &self.last_side_effect {
                        self.g.add_strong_dep(item_id, last);
                    }

                    // Create weak dependencies to all LAST_WRITES and
                    // LAST_READS.
                    for id in eventual_ids.iter() {
                        if let Some(state) = self.vars.get(id) {
                            for last_write in state.last_writes.iter() {
                                self.g.add_weak_dep(item_id, last_write);
                            }

                            for last_read in state.last_reads.iter() {
                                self.g.add_weak_dep(item_id, last_read);
                            }
                        }
                    }
                }

                // For each var in WRITE_VARS:
                for id in item.write_vars.iter() {
                    // Add this module item to LAST_WRITES

                    let state = self.vars.entry(id.clone()).or_default();
                    state.last_writes.push(item_id.clone());

                    // TODO: Optimization: Remove each module item to which we
                    // just created a strong dependency from LAST_WRITES
                }

                // For each var in READ_VARS:
                for id in item.read_vars.iter() {
                    // Add this module item to LAST_READS

                    let state = self.vars.entry(id.clone()).or_default();
                    state.last_reads.push(item_id.clone());

                    // Optimization: Remove each module item to which we
                    // just created a strong dependency from LAST_READS

                    if let Some(items) = items_to_remove_from_last_reads.get(id) {
                        for item in items {
                            if let Some(pos) = state.last_reads.iter().position(|v| *v == *item) {
                                state.last_reads.remove(pos);
                            }
                        }
                    }
                }

                if item.side_effects {
                    self.last_side_effect = Some(item_id.clone());
                    self.last_side_effects.push(item_id.clone());
                }
            }
        }
    }

    /// Phase 3: Eventual evaluation
    fn evaluate_eventual(&mut self, module: &Module) {
        for item_id in self.item_ids.iter() {
            if let Some(item) = self.items.get(item_id) {
                // For each var in EVENTUAL_READ_VARS:

                for id in item.eventual_read_vars.iter() {
                    // Create a strong dependency to all module items listed in
                    // LAST_WRITES for that var.

                    if let Some(state) = self.vars.get(id) {
                        for last_write in state.last_writes.iter() {
                            self.g.add_strong_dep(item_id, last_write);
                        }
                    }
                }

                // For each var in EVENTUAL_WRITE_VARS:
                for id in item.eventual_write_vars.iter() {
                    // Create a weak dependency to all module items listed in
                    // LAST_READS for that var.

                    if let Some(state) = self.vars.get(id) {
                        for last_read in state.last_reads.iter() {
                            self.g.add_weak_dep(item_id, last_read);
                        }
                    }
                }

                // (no state update happens, since this is only triggered by
                // side effects, which we already handled)
            }
        }
    }

    /// Phase 4: Exports
    fn handle_exports(&mut self, module: &Module) {
        for item_id in self.item_ids.iter() {
            if item_id.index == usize::MAX {
                match &item_id.kind {
                    ItemIdKind::ModuleEvaluation => {
                        // Create a strong dependency to LAST_SIDE_EFFECTS

                        for last in self.last_side_effects.iter() {
                            self.g.add_strong_dep(item_id, last);
                        }

                        // // Create weak dependencies to all LAST_WRITES and
                        // // LAST_READS.

                        // for (.., state) in self.vars.iter() {
                        //     for last_write in state.last_writes.iter() {
                        //         self.g.add_weak_dep(item_id, last_write);
                        //     }

                        //     for last_read in state.last_reads.iter() {
                        //         self.g.add_weak_dep(item_id, last_read);
                        //     }
                        // }
                    }
                    ItemIdKind::Export(id) => {
                        // Create a strong dependency to LAST_WRITES for this var

                        if let Some(state) = self.vars.get(id) {
                            for last_write in state.last_writes.iter() {
                                self.g.add_strong_dep(item_id, last_write);
                            }
                        }
                    }

                    _ => {}
                }
            }
        }
    }
}

#[turbo_tasks::value]
pub struct EcmascriptModulePartAsset {
    module: EcmascriptModuleAssetVc,
    chunk_id: u32,
}

#[turbo_tasks::value_impl]
impl Asset for EcmascriptModulePartAsset {
    #[turbo_tasks::function]
    fn path(&self) -> FileSystemPathVc {
        self.module.path()
    }

    #[turbo_tasks::function]
    fn content(&self) -> AssetContentVc {
        todo!()
    }

    #[turbo_tasks::function]
    fn references(&self) -> AssetReferencesVc {
        todo!()
    }

    #[turbo_tasks::function]
    fn versioned_content(&self) -> VersionedContentVc {}
}

#[turbo_tasks::value_impl]
impl ChunkableAsset for EcmascriptModulePartAsset {
    #[turbo_tasks::function]
    fn as_chunk(&self, context: ChunkingContextVc) -> ChunkVc {}
}

#[turbo_tasks::value]
pub struct EcmascriptModulePartChunkItem {
    full_module: EcmascriptModuleAssetVc,

    module: EcmascriptModulePartAssetVc,
    context: ChunkingContextVc,

    chunk_id: u32,
    deps: Vec<u32>,
}

#[turbo_tasks::value_impl]
impl ValueToString for EcmascriptModulePartChunkItem {
    #[turbo_tasks::function]
    async fn to_string(&self) -> Result<StringVc> {
        Ok(StringVc::cell(format!(
            "{} (ecmascript) -> chunk {}",
            self.module.await?.source.path().to_string().await?,
            self.chunk_id
        )))
    }
}

#[turbo_tasks::value_impl]
impl EcmascriptChunkItem for EcmascriptModulePartChunkItem {
    #[turbo_tasks::function]
    fn content(&self) -> EcmascriptChunkItemContentVc {}

    #[turbo_tasks::function]
    fn chunking_context(&self) -> ChunkingContextVc {
        self.context
    }

    #[turbo_tasks::function]
    fn id(&self) -> ModuleIdVc {}
}

#[turbo_tasks::value_impl]
impl ChunkItem for EcmascriptModulePartChunkItem {
    #[turbo_tasks::function]
    async fn references(&self) -> AssetReferencesVc {
        let assets = self
            .deps
            .iter()
            .map(|&chunk_id| {
                SingleAssetReference::new(
                    EcmascriptModulePartAsset {
                        module: self.full_module.clone(),
                        chunk_id,
                    }
                    .into(),
                )
            })
            .collect::<Vec<_>>();

        AssetReferences::new(assets)
    }
}
