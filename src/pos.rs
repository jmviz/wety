use anyhow::anyhow;
use serde::{Deserialize, Serialize};

use crate::pos_phf::POS;

// PosId refers to an index in the POS OrderedSet
pub(crate) type PosId = u8; // the set has ~50 elements

#[derive(Hash, Eq, PartialEq, Debug, Copy, Clone, Serialize, Deserialize)]
pub(crate) struct Pos {
    id: PosId,
}

impl From<PosId> for Pos {
    fn from(pos_id: PosId) -> Self {
        Self { id: pos_id }
    }
}

impl TryFrom<&str> for Pos {
    type Error = anyhow::Error;

    fn try_from(pos: &str) -> Result<Self, Self::Error> {
        if let Some(id) = POS.get_index(pos) {
            return Ok(PosId::try_from(id)?.into());
        }
        Err(anyhow!("\"{pos}\" does not exist POS"))
    }
}

impl Pos {
    pub(crate) fn id(&self) -> PosId {
        self.id
    }

    pub(crate) fn name(&self) -> &'static str {
        POS.index(self.id as usize)
            .expect("id cannot have been created without being a valid index")
    }
}
