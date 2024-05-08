use std::{cmp::Ordering, hash::Hash};

use super::{
    balance_queue::BalanceQueue,
    followers::add_follower,
    in_progress::{finish_in_progress_without_node, start_in_progress},
    increase_aggregation_number_internal,
    uppers::add_upper,
    AggregationContext, AggregationNode, PreparedInternalOperation,
};

impl<I: Clone + Eq + Hash, D> AggregationNode<I, D> {
    // Called when a inner node of the upper node has a new follower
    // It's expected that the upper node is flagged as "in progress"
    pub(super) fn notify_new_follower<C: AggregationContext<NodeRef = I, Data = D>>(
        &mut self,
        ctx: &C,
        balance_queue: &mut BalanceQueue<I>,
        upper_id: &C::NodeRef,
        follower_id: &C::NodeRef,
    ) -> PreparedNotifyNewFollower<C> {
        let AggregationNode::Aggegating(aggregating) = self else {
            unreachable!();
        };
        if aggregating.followers.add_if_entry(follower_id) {
            self.finish_in_progress(ctx, balance_queue, upper_id);
            PreparedNotifyNewFollower::AlreadyAdded
        } else {
            let upper_aggregation_number = aggregating.aggregation_number;
            if upper_aggregation_number == u32::MAX {
                PreparedNotifyNewFollower::Inner {
                    upper_id: upper_id.clone(),
                    follower_id: follower_id.clone(),
                }
            } else {
                PreparedNotifyNewFollower::FollowerOrInner {
                    upper_aggregation_number,
                    upper_id: upper_id.clone(),
                    follower_id: follower_id.clone(),
                }
            }
        }
    }

    // Called when a inner node of the upper node has a new follower
    // It's expected that the upper node is NOT flagged as "in progress"
    pub(super) fn notify_new_follower_not_in_progress<
        C: AggregationContext<NodeRef = I, Data = D>,
    >(
        &mut self,
        ctx: &C,
        upper_id: &C::NodeRef,
        follower_id: &C::NodeRef,
    ) -> PreparedNotifyNewFollower<C> {
        let AggregationNode::Aggegating(aggregating) = self else {
            unreachable!();
        };
        if aggregating.followers.add_if_entry(follower_id) {
            PreparedNotifyNewFollower::AlreadyAdded
        } else {
            start_in_progress(ctx, upper_id);
            let upper_aggregation_number = aggregating.aggregation_number;
            if upper_aggregation_number == u32::MAX {
                PreparedNotifyNewFollower::Inner {
                    upper_id: upper_id.clone(),
                    follower_id: follower_id.clone(),
                }
            } else {
                PreparedNotifyNewFollower::FollowerOrInner {
                    upper_aggregation_number,
                    upper_id: upper_id.clone(),
                    follower_id: follower_id.clone(),
                }
            }
        }
    }
}

pub(super) enum PreparedNotifyNewFollower<C: AggregationContext> {
    AlreadyAdded,
    Inner {
        upper_id: C::NodeRef,
        follower_id: C::NodeRef,
    },
    FollowerOrInner {
        upper_aggregation_number: u32,
        upper_id: C::NodeRef,
        follower_id: C::NodeRef,
    },
}

impl<C: AggregationContext> PreparedInternalOperation<C> for PreparedNotifyNewFollower<C> {
    type Result = ();
    fn apply(self, ctx: &C, balance_queue: &mut BalanceQueue<C::NodeRef>) {
        match self {
            PreparedNotifyNewFollower::AlreadyAdded => return,
            PreparedNotifyNewFollower::Inner {
                upper_id,
                follower_id,
            } => {
                let follower = ctx.node(&follower_id);
                add_upper(ctx, balance_queue, follower, &follower_id, &upper_id);
                finish_in_progress_without_node(ctx, balance_queue, &upper_id);
            }
            PreparedNotifyNewFollower::FollowerOrInner {
                mut upper_aggregation_number,
                upper_id,
                follower_id,
            } => loop {
                let follower = ctx.node(&follower_id);
                let follower_aggregation_number = follower.aggregation_number();
                if follower_aggregation_number < upper_aggregation_number {
                    add_upper(ctx, balance_queue, follower, &follower_id, &upper_id);
                    finish_in_progress_without_node(ctx, balance_queue, &upper_id);
                    return;
                } else {
                    drop(follower);
                    let mut upper = ctx.node(&upper_id);
                    let AggregationNode::Aggegating(aggregating) = &mut *upper else {
                        unreachable!();
                    };
                    upper_aggregation_number = aggregating.aggregation_number;
                    if upper_aggregation_number == u32::MAX {
                        // retry, concurrency
                    } else {
                        match follower_aggregation_number.cmp(&upper_aggregation_number) {
                            Ordering::Less => {
                                // retry, concurrency
                            }
                            Ordering::Equal => {
                                drop(upper);
                                let follower = ctx.node(&follower_id);
                                let follower_aggregation_number = follower.aggregation_number();
                                if follower_aggregation_number == upper_aggregation_number {
                                    increase_aggregation_number_internal(
                                        ctx,
                                        balance_queue,
                                        follower,
                                        &follower_id,
                                        upper_aggregation_number + 1,
                                    );
                                    // retry
                                } else {
                                    // retry, concurrency
                                }
                            }
                            Ordering::Greater => {
                                upper.finish_in_progress(ctx, balance_queue, &upper_id);
                                add_follower(ctx, balance_queue, upper, &follower_id);
                                return;
                            }
                        }
                    }
                }
            },
        }
    }
}

pub fn notify_new_follower<C: AggregationContext>(
    ctx: &C,
    balance_queue: &mut BalanceQueue<C::NodeRef>,
    mut upper: C::Guard<'_>,
    upper_id: &C::NodeRef,
    follower_id: &C::NodeRef,
) {
    let p = upper.notify_new_follower(ctx, balance_queue, upper_id, follower_id);
    drop(upper);
    p.apply(ctx, balance_queue);
}
