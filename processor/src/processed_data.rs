use serde::{Deserialize, Serialize};

use crate::{ety_graph::EtyGraph, items::Items, string_pool::StringPool};

#[derive(Serialize, Deserialize)]
pub(crate) struct ProcessedData {
    pub(crate) string_pool: StringPool,
    pub(crate) items: Items,
    pub(crate) ety_graph: EtyGraph,
}

// pub struct ServerData {
//     string_pool: StringPool,
//     items: ItemStore,
//     graph:
// }
