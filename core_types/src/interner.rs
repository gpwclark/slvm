use std::hash::{Hash, Hasher};

#[derive(Clone, Copy, Debug)]
pub struct Interned {
    pub id: u32,
}

impl PartialEq for Interned {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Interned {}

impl Hash for Interned {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u32(self.id);
    }
}
