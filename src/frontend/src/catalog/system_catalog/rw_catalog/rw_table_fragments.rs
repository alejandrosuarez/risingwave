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

use itertools::Itertools;
use risingwave_common::catalog::RW_CATALOG_SCHEMA_NAME;
use risingwave_common::error::Result;
use risingwave_common::row::OwnedRow;
use risingwave_common::types::{DataType, ScalarImpl};

use crate::catalog::system_catalog::{BuiltinTable, SysCatalogReaderImpl};

pub const RW_TABLE_FRAGMENTS: BuiltinTable = BuiltinTable {
    name: "rw_table_fragments",
    schema: RW_CATALOG_SCHEMA_NAME,
    columns: &[(DataType::Int32, "table_id"), (DataType::Varchar, "status")],
    pk: &[0],
};

impl SysCatalogReaderImpl {
    pub async fn read_rw_table_fragments_info(&self) -> Result<Vec<OwnedRow>> {
        let states = self.meta_client.list_table_fragment_states().await?;

        Ok(states
            .into_iter()
            .map(|state| {
                OwnedRow::new(vec![
                    Some(ScalarImpl::Int32(state.table_id as i32)),
                    Some(ScalarImpl::Utf8(state.state().as_str_name().into())),
                ])
            })
            .collect_vec())
    }
}