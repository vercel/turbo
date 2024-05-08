use std::ops::{Deref, DerefMut};

use super::{
    increase_aggregation_number, AggregationContext, AggregationNode, AggregationNodeGuard,
};

/// Gives an reference to the root aggregated info for a given item.
pub fn aggregation_data<'l, C: AggregationContext>(
    ctx: &'l C,
    node_id: &C::NodeRef,
) -> AggregationDataGuard<C::Guard<'l>>
where
    C: 'l,
{
    {
        let guard = ctx.node(node_id);
        if guard.aggregation_number() == u32::MAX {
            return AggregationDataGuard { guard };
        }
    }
    increase_aggregation_number(ctx, ctx.node(&node_id), &node_id, u32::MAX);
    ctx.optimize_queue().force_process(ctx);
    {
        let guard = ctx.node(node_id);
        debug_assert!(guard.aggregation_number() == u32::MAX);
        AggregationDataGuard { guard }
    }
}

pub fn prepare_aggregation_data<C: AggregationContext>(ctx: &C, node_id: &C::NodeRef) {
    increase_aggregation_number(ctx, ctx.node(&node_id), &node_id, u32::MAX);
    ctx.optimize_queue().force_process(ctx);
}

/// A reference to the root aggregated info of a node.
pub struct AggregationDataGuard<G> {
    guard: G,
}

impl<G> AggregationDataGuard<G> {
    pub fn into_inner(self) -> G {
        self.guard
    }
}

impl<G: AggregationNodeGuard> Deref for AggregationDataGuard<G> {
    type Target = G::Data;

    fn deref(&self) -> &Self::Target {
        match &*self.guard {
            AggregationNode::Leaf { .. } => unreachable!(),
            AggregationNode::Aggegating(aggregating) => &aggregating.data,
        }
    }
}

impl<G: AggregationNodeGuard> DerefMut for AggregationDataGuard<G> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match &mut *self.guard {
            AggregationNode::Leaf { .. } => unreachable!(),
            AggregationNode::Aggegating(aggregating) => &mut aggregating.data,
        }
    }
}
