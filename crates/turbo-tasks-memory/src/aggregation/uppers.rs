use super::{
    balance_queue::BalanceQueue,
    in_progress::{
        finish_in_progress_without_node, start_in_progress_all, start_in_progress_count,
    },
    increase::LEAF_NUMBER,
    increase_aggregation_number_internal, AggegatingNode, AggregationContext, AggregationNode,
    AggregationNodeGuard, PreparedInternalOperation, PreparedOperation, StackVec,
};
use crate::count_hash_set::RemovePositiveCountResult;

const MAX_UPPERS: usize = 4;

pub fn add_upper<C: AggregationContext>(
    ctx: &C,
    balance_queue: &mut BalanceQueue<C::NodeRef>,
    node: C::Guard<'_>,
    node_id: &C::NodeRef,
    upper_id: &C::NodeRef,
) {
    add_upper_count(ctx, balance_queue, node, node_id, upper_id, 1);
}

pub fn add_upper_count<C: AggregationContext>(
    ctx: &C,
    balance_queue: &mut BalanceQueue<C::NodeRef>,
    mut node: C::Guard<'_>,
    node_id: &C::NodeRef,
    upper_id: &C::NodeRef,
    count: usize,
) -> isize {
    // TODO add_clonable_count could return the current count for better performance
    let (added, count) = match &mut *node {
        AggregationNode::Leaf { uppers, .. } => {
            if uppers.add_clonable_count(upper_id, count) {
                let count = uppers.get_count(upper_id);
                (true, count)
            } else {
                (false, uppers.get_count(upper_id))
            }
        }
        AggregationNode::Aggegating(aggegating) => {
            let AggegatingNode { ref mut uppers, .. } = **aggegating;
            if uppers.add_clonable_count(upper_id, count) {
                let count = uppers.get_count(upper_id);
                (true, count)
            } else {
                (false, uppers.get_count(upper_id))
            }
        }
    };
    if added {
        on_added(ctx, balance_queue, node, node_id, upper_id);
    } else {
        drop(node);
    }
    count
}

pub fn on_added<C: AggregationContext>(
    ctx: &C,
    balance_queue: &mut BalanceQueue<C::NodeRef>,
    mut node: C::Guard<'_>,
    node_id: &C::NodeRef,
    upper_id: &C::NodeRef,
) {
    let uppers = node.uppers();
    let uppers_len = uppers.len();
    let optimize = (uppers_len > MAX_UPPERS && (uppers_len - MAX_UPPERS).count_ones() == 1)
        .then(|| (true, uppers.iter().cloned().collect::<StackVec<_>>()));
    let (add_change, followers) = match &mut *node {
        AggregationNode::Leaf { .. } => {
            let add_change = node.get_add_change();
            let children = node.children().collect::<StackVec<_>>();
            start_in_progress_count(ctx, upper_id, children.len() as u32);
            drop(node);
            (add_change, children)
        }
        AggregationNode::Aggegating(aggegating) => {
            let AggegatingNode { ref followers, .. } = **aggegating;
            let add_change = ctx.data_to_add_change(&aggegating.data);
            let followers = followers.iter().cloned().collect::<StackVec<_>>();
            start_in_progress_count(ctx, upper_id, followers.len() as u32);
            drop(node);

            (add_change, followers)
        }
    };

    // Make sure to propagate the change to the upper node
    let mut upper = ctx.node(upper_id);
    let add_prepared = add_change.and_then(|add_change| upper.apply_change(ctx, add_change));
    let prepared = followers
        .into_iter()
        .map(|child_id| upper.notify_new_follower(ctx, upper_id, &child_id))
        .collect::<StackVec<_>>();
    drop(upper);
    add_prepared.apply(ctx);
    prepared.apply(ctx, balance_queue);

    // This heuristic ensures that we don’t have too many upper edges, which would
    // degrade update performance
    if let Some((leaf, uppers)) = optimize {
        let count = uppers.len();
        let mut root_count = 0;
        let mut min = LEAF_NUMBER as u32 - 1;
        let mut uppers_uppers = 0;
        for upper_id in uppers.into_iter() {
            let upper = ctx.node(&upper_id);
            let aggregation_number = upper.aggregation_number();
            if aggregation_number == u32::MAX {
                root_count += 1;
            } else {
                let upper_uppers = upper.uppers().len();
                uppers_uppers += upper_uppers;
                if aggregation_number < min {
                    min = aggregation_number;
                }
            }
        }
        if leaf {
            increase_aggregation_number_internal(
                ctx,
                balance_queue,
                ctx.node(node_id),
                node_id,
                min + 1,
            );
        } else {
            let normal_count = count - root_count;
            if normal_count > 0 {
                let avg_uppers_uppers = uppers_uppers / normal_count;
                if count > avg_uppers_uppers && root_count * 2 < count {
                    increase_aggregation_number_internal(
                        ctx,
                        balance_queue,
                        ctx.node(node_id),
                        node_id,
                        min + 1,
                    );
                }
            }
        }
    }
}

pub fn remove_upper_count<C: AggregationContext>(
    ctx: &C,
    mut node: C::Guard<'_>,
    upper_id: &C::NodeRef,
    count: usize,
) {
    let removed = match &mut *node {
        AggregationNode::Leaf { uppers, .. } => uppers.remove_clonable_count(upper_id, count),
        AggregationNode::Aggegating(aggegating) => {
            let AggegatingNode { ref mut uppers, .. } = **aggegating;
            uppers.remove_clonable_count(upper_id, count)
        }
    };
    if removed {
        on_removed(ctx, node, upper_id);
    }
}

pub struct RemovePositiveUpperCountResult {
    pub removed_count: usize,
    pub remaining_count: isize,
}

pub fn remove_positive_upper_count<C: AggregationContext>(
    ctx: &C,
    mut node: C::Guard<'_>,
    upper_id: &C::NodeRef,
    count: usize,
) -> RemovePositiveUpperCountResult {
    let RemovePositiveCountResult {
        removed,
        removed_count,
        count,
    } = match &mut *node {
        AggregationNode::Leaf { uppers, .. } => {
            uppers.remove_positive_clonable_count(upper_id, count)
        }
        AggregationNode::Aggegating(aggegating) => {
            let AggegatingNode { ref mut uppers, .. } = **aggegating;
            uppers.remove_positive_clonable_count(upper_id, count)
        }
    };
    if removed {
        on_removed(ctx, node, upper_id);
    }
    RemovePositiveUpperCountResult {
        removed_count,
        remaining_count: count,
    }
}

pub fn on_removed<C: AggregationContext>(ctx: &C, node: C::Guard<'_>, upper_id: &C::NodeRef) {
    match &*node {
        AggregationNode::Leaf { .. } => {
            let remove_change = node.get_remove_change();
            let children = node.children().collect::<StackVec<_>>();
            drop(node);
            let mut upper = ctx.node(upper_id);
            let remove_prepared =
                remove_change.and_then(|remove_change| upper.apply_change(ctx, remove_change));
            start_in_progress_count(ctx, upper_id, children.len() as u32);
            let prepared = children
                .into_iter()
                .map(|child_id| upper.notify_lost_follower(ctx, upper_id, &child_id))
                .collect::<StackVec<_>>();
            drop(upper);
            remove_prepared.apply(ctx);
            prepared.apply(ctx);
        }
        AggregationNode::Aggegating(aggegating) => {
            let remove_change = ctx.data_to_remove_change(&aggegating.data);
            let followers = aggegating
                .followers
                .iter()
                .cloned()
                .collect::<StackVec<_>>();
            drop(node);
            let mut upper = ctx.node(upper_id);
            let remove_prepared =
                remove_change.and_then(|remove_change| upper.apply_change(ctx, remove_change));
            start_in_progress_count(ctx, upper_id, followers.len() as u32);
            let prepared = followers
                .into_iter()
                .map(|child_id| upper.notify_lost_follower(ctx, upper_id, &child_id))
                .collect::<StackVec<_>>();
            drop(upper);
            remove_prepared.apply(ctx);
            prepared.apply(ctx);
        }
    }
}

pub(super) fn get_aggregated_remove_change<C: AggregationContext>(
    ctx: &C,
    guard: &C::Guard<'_>,
) -> Option<C::DataChange> {
    match &**guard {
        AggregationNode::Leaf { .. } => guard.get_remove_change(),
        AggregationNode::Aggegating(aggegating) => ctx.data_to_remove_change(&aggegating.data),
    }
}

pub(super) fn get_aggregated_add_change<C: AggregationContext>(
    ctx: &C,
    guard: &C::Guard<'_>,
) -> Option<C::DataChange> {
    match &**guard {
        AggregationNode::Leaf { .. } => guard.get_add_change(),
        AggregationNode::Aggegating(aggegating) => ctx.data_to_add_change(&aggegating.data),
    }
}
