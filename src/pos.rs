use anyhow::anyhow;

use crate::pos_phf::POS;

// PosId refers to an index in the POS OrderedSet
pub(crate) type PosId = usize;

#[derive(Hash, Eq, PartialEq, Debug, Copy, Clone)]
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
            return Ok(id.into());
        }
        Err(anyhow!("\"{pos}\" does not exist POS"))
    }
}

impl Pos {
    pub(crate) fn id(&self) -> PosId {
        self.id
    }

    pub(crate) fn name(&self) -> &'static str {
        POS.index(self.id)
            .expect("id cannot have been created without being a valid index")
    }
}
