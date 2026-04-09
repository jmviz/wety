use std::str::FromStr;

use anyhow::anyhow;
use serde::{Deserialize, Serialize};

use crate::pos_phf::POS;

pub type PosId = u8;

#[derive(Hash, Eq, PartialEq, Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Pos {
    id: PosId,
}

impl From<PosId> for Pos {
    fn from(pos_id: PosId) -> Self {
        Self { id: pos_id }
    }
}

impl FromStr for Pos {
    type Err = anyhow::Error;

    fn from_str(pos: &str) -> Result<Self, Self::Err> {
        if let Some(id) = POS.get_index(pos) {
            return Ok(PosId::try_from(id)?.into());
        }
        Err(anyhow!("\"{pos}\" does not exist in POS"))
    }
}

impl Pos {
    /// # Panics
    ///
    /// Panics if this `Pos` holds an invalid internal index.
    #[must_use]
    pub fn name(self) -> &'static str {
        POS.index(self.id as usize)
            .expect("id cannot have been created without being a valid index")
    }

    /// # Panics
    ///
    /// Panics if `"root"` is not present in the POS table.
    #[must_use]
    pub fn root_pos() -> Pos {
        "root".parse().expect("root pos must exist")
    }
}
