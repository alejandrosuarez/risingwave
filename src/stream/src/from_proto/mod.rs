// Copyright 2023 RisingWave Labs
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Build executor from protobuf.

mod agg_common;
mod append_only_dedup;
mod barrier_recv;
mod batch_query;
mod chain;
mod dml;
mod dynamic_filter;
mod eowc_over_window;
mod expand;
mod filter;
mod group_top_n;
mod hash_agg;
mod hash_join;
mod hop_window;
mod lookup;
mod lookup_union;
mod merge;
mod mview;
mod no_op;
mod now;
mod over_window;
mod project;
mod project_set;
mod row_id_gen;
mod simple_agg;
mod sink;
mod sort;
mod source;
mod stateless_simple_agg;
mod temporal_join;
mod top_n;
mod union;
mod values;
mod watermark_filter;

// import for submodules
use itertools::Itertools;
use risingwave_pb::stream_plan::stream_node::NodeBody;
use risingwave_pb::stream_plan::{StreamNode, TemporalJoinNode};
use risingwave_storage::StateStore;

use self::append_only_dedup::*;
use self::barrier_recv::*;
use self::batch_query::*;
use self::chain::*;
use self::dml::*;
use self::dynamic_filter::*;
use self::eowc_over_window::*;
use self::expand::*;
use self::filter::*;
use self::group_top_n::GroupTopNExecutorBuilder;
use self::hash_agg::*;
use self::hash_join::*;
use self::hop_window::*;
use self::lookup::*;
use self::lookup_union::*;
use self::merge::*;
use self::mview::*;
use self::no_op::*;
use self::now::NowExecutorBuilder;
use self::over_window::*;
use self::project::*;
use self::project_set::*;
use self::row_id_gen::RowIdGenExecutorBuilder;
use self::simple_agg::*;
use self::sink::*;
use self::sort::*;
use self::source::*;
use self::stateless_simple_agg::*;
use self::temporal_join::*;
use self::top_n::*;
use self::union::*;
use self::watermark_filter::WatermarkFilterBuilder;
use crate::error::StreamResult;
use crate::executor::{BoxedExecutor, Executor, ExecutorInfo};
use crate::from_proto::values::ValuesExecutorBuilder;
use crate::task::{ExecutorParams, LocalStreamManagerCore};

#[async_trait::async_trait]
trait ExecutorBuilder {
    type Node;

    /// Create a [`BoxedExecutor`] from [`StreamNode`].
    async fn new_boxed_executor(
        params: ExecutorParams,
        node: &Self::Node,
        store: impl StateStore,
        stream: &mut LocalStreamManagerCore,
    ) -> StreamResult<BoxedExecutor>;
}

macro_rules! build_executor {
    ($source:expr, $node:expr, $store:expr, $stream:expr, $($proto_type_name:path => $data_type:ty),* $(,)?) => {
        match $node.get_node_body().unwrap() {
            $(
                $proto_type_name(node) => {
                    <$data_type>::new_boxed_executor($source, node, $store, $stream).await
                },
            )*
            NodeBody::Exchange(_) | NodeBody::DeltaIndexJoin(_) => unreachable!()
        }
    }
}

/// Create an executor from protobuf [`StreamNode`].
pub async fn create_executor(
    params: ExecutorParams,
    stream: &mut LocalStreamManagerCore,
    node: &StreamNode,
    store: impl StateStore,
) -> StreamResult<BoxedExecutor> {
    build_executor! {
        params,
        node,
        store,
        stream,
        NodeBody::Source => SourceExecutorBuilder,
        NodeBody::Sink => SinkExecutorBuilder,
        NodeBody::Project => ProjectExecutorBuilder,
        NodeBody::TopN => TopNExecutorBuilder::<false>,
        NodeBody::AppendOnlyTopN => TopNExecutorBuilder::<true>,
        NodeBody::StatelessSimpleAgg => StatelessSimpleAggExecutorBuilder,
        NodeBody::SimpleAgg => SimpleAggExecutorBuilder,
        NodeBody::HashAgg => HashAggExecutorBuilder,
        NodeBody::HashJoin => HashJoinExecutorBuilder,
        NodeBody::HopWindow => HopWindowExecutorBuilder,
        NodeBody::Chain => ChainExecutorBuilder,
        NodeBody::BatchPlan => BatchQueryExecutorBuilder,
        NodeBody::Merge => MergeExecutorBuilder,
        NodeBody::Materialize => MaterializeExecutorBuilder,
        NodeBody::Filter => FilterExecutorBuilder,
        NodeBody::Arrange => ArrangeExecutorBuilder,
        NodeBody::Lookup => LookupExecutorBuilder,
        NodeBody::Union => UnionExecutorBuilder,
        NodeBody::LookupUnion => LookupUnionExecutorBuilder,
        NodeBody::Expand => ExpandExecutorBuilder,
        NodeBody::DynamicFilter => DynamicFilterExecutorBuilder,
        NodeBody::ProjectSet => ProjectSetExecutorBuilder,
        NodeBody::GroupTopN => GroupTopNExecutorBuilder::<false>,
        NodeBody::AppendOnlyGroupTopN => GroupTopNExecutorBuilder::<true>,
        NodeBody::Sort => SortExecutorBuilder,
        NodeBody::WatermarkFilter => WatermarkFilterBuilder,
        NodeBody::Dml => DmlExecutorBuilder,
        NodeBody::RowIdGen => RowIdGenExecutorBuilder,
        NodeBody::Now => NowExecutorBuilder,
        NodeBody::TemporalJoin => TemporalJoinExecutorBuilder,
        NodeBody::Values => ValuesExecutorBuilder,
        NodeBody::BarrierRecv => BarrierRecvExecutorBuilder,
        NodeBody::AppendOnlyDedup => AppendOnlyDedupExecutorBuilder,
        NodeBody::NoOp => NoOpExecutorBuilder,
        NodeBody::EowcOverWindow => EowcOverWindowExecutorBuilder,
        NodeBody::OverWindow => OverWindowExecutorBuilder,
    }
}