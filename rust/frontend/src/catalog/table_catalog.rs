use std::sync::atomic::{AtomicU32, Ordering};

use risingwave_common::error::Result;

use crate::catalog::column_catalog::{ColumnCatalog, ColumnDesc};
use crate::catalog::{ColumnId, TableId};

pub struct TableCatalog {
    table_id: TableId,
    next_column_id: AtomicU32,
    column_by_name: Vec<(String, ColumnCatalog)>,
    primary_keys: Vec<ColumnId>,
}

impl TableCatalog {
    pub fn new(table_id: TableId) -> Self {
        Self {
            table_id,
            next_column_id: AtomicU32::new(0),
            column_by_name: vec![],
            primary_keys: vec![],
        }
    }

    pub fn add_column(&mut self, col_name: &str, col_desc: ColumnDesc) -> Result<()> {
        let col_catalog = ColumnCatalog::new(
            self.next_column_id.fetch_add(1, Ordering::Relaxed),
            col_name.to_string(),
            col_desc.clone(),
        );
        if col_desc.is_primary() {
            self.primary_keys.push(col_catalog.id());
        }
        self.column_by_name
            .push((col_name.to_string(), col_catalog));
        Ok(())
    }

    pub fn get_column_by_id(&self, col_id: ColumnId) -> Option<&ColumnCatalog> {
        self.column_by_name
            .get(col_id as usize)
            .map(|(_, col_catalog)| col_catalog)
    }

    pub fn id(&self) -> TableId {
        self.table_id
    }

    pub fn get_pks(&self) -> Vec<u32> {
        self.primary_keys.clone()
    }
}
