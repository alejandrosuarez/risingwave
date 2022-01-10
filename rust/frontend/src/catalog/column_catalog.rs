use risingwave_common::types::DataTypeRef;

use crate::catalog::ColumnId;

/// A descriptor of a column.
#[derive(Debug, Clone)]
pub struct ColumnDesc {
    data_type: DataTypeRef,
    is_primary: bool,
}

impl ColumnDesc {
    pub fn new(data_type: DataTypeRef, is_primary: bool) -> Self {
        ColumnDesc {
            data_type,
            is_primary,
        }
    }

    pub fn is_primary(&self) -> bool {
        self.is_primary
    }

    pub fn is_nullable(&self) -> bool {
        self.data_type.is_nullable()
    }

    pub fn data_type_ref(&self) -> &DataTypeRef {
        &self.data_type
    }
}

/// The catalog of a column.
#[derive(Debug, Clone)]
pub struct ColumnCatalog {
    id: ColumnId,
    name: String,
    desc: ColumnDesc,
}

impl ColumnCatalog {
    pub fn new(id: ColumnId, name: String, desc: ColumnDesc) -> ColumnCatalog {
        ColumnCatalog { id, name, desc }
    }

    pub fn id(&self) -> ColumnId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn data_type_clone(&self) -> DataTypeRef {
        self.desc.data_type.clone()
    }

    pub fn is_primary(&self) -> bool {
        self.desc.is_primary()
    }

    pub fn is_nullable(&self) -> bool {
        self.desc.is_nullable()
    }

    pub fn col_desc_ref(&self) -> &ColumnDesc {
        &self.desc
    }
}
