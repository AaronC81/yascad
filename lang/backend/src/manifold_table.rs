use std::collections::HashMap;

use manifold_rs::Manifold;
use yascad_frontend::InputSourceSpan;

use crate::{RuntimeError, RuntimeErrorKind};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ManifoldTableIndex(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifoldDisposition {
    /// The manifold physically exists in the final scene.
    Physical,

    /// The manifold was created in a buffered environment, and does not exist in the final scene.
    Virtual,
}

impl ManifoldDisposition {
    pub fn flatten(dispositions: &[ManifoldDisposition], span: InputSourceSpan) -> Result<ManifoldDisposition, RuntimeError> {
        if dispositions.iter().all(|d| d == &ManifoldDisposition::Physical) {
            Ok(ManifoldDisposition::Physical)
        } else if dispositions.iter().all(|d| d == &ManifoldDisposition::Virtual) {
            Ok(ManifoldDisposition::Virtual)
        } else {
            Err(RuntimeError::new(
                RuntimeErrorKind::MixedManifoldDisposition,
                span,
            ))
        }
    }
}

pub struct ManifoldTable {
    table: HashMap<usize, (Manifold, ManifoldDisposition)>,
    next_index: usize,
}

impl ManifoldTable {
    pub fn new() -> Self {
        Self {
            table: HashMap::new(),
            next_index: 1,
        }
    }

    pub fn add(&mut self, manifold: Manifold, disposition: ManifoldDisposition) -> ManifoldTableIndex {
        let idx = self.take_next_index();
        self.table.insert(idx.0, (manifold, disposition));
        idx
    }

    pub fn remove(&mut self, index: ManifoldTableIndex) -> (Manifold, ManifoldDisposition) {
        self.table.remove(&index.0).expect("manifold not in table")
    }

    pub fn get(&self, index: &ManifoldTableIndex) -> &Manifold {
        &self.table.get(&index.0).expect("manifold not in table").0
    }

    pub fn map(&mut self, index: ManifoldTableIndex, func: impl FnOnce(Manifold) -> Manifold) -> ManifoldTableIndex {
        let (manifold, disposition) = self.remove(index);
        self.add(func(manifold), disposition)
    }

    pub fn iter_manifolds(&self) -> impl Iterator<Item = &(Manifold, ManifoldDisposition)> {
        self.table.values()
    }

    fn take_next_index(&mut self) -> ManifoldTableIndex {
        let idx = ManifoldTableIndex(self.next_index);
        self.next_index += 1;
        idx
    }
}
