use std::{collections::HashMap, marker::PhantomData};

use manifold_rs::{CrossSection, Manifold};
use yascad_frontend::InputSourceSpan;

use crate::{RuntimeError, RuntimeErrorKind};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GeometryTableIndex(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeometryDisposition {
    /// The geometry physically exists in the final scene.
    Physical,

    /// The geometry was created in a buffered environment, and does not exist in the final scene.
    Virtual,
}

impl GeometryDisposition {
    pub fn flatten(dispositions: &[GeometryDisposition], span: InputSourceSpan) -> Result<GeometryDisposition, RuntimeError> {
        if dispositions.iter().all(|d| d == &GeometryDisposition::Physical) {
            Ok(GeometryDisposition::Physical)
        } else if dispositions.iter().all(|d| d == &GeometryDisposition::Virtual) {
            Ok(GeometryDisposition::Virtual)
        } else {
            Err(RuntimeError::new(
                RuntimeErrorKind::MixedGeometryDisposition,
                span,
            ))
        }
    }
}

#[derive(Debug, Clone)]
pub enum GeometryTableEntry {
    Manifold(Manifold),
    CrossSection(CrossSection),
}

impl GeometryTableEntry {
    pub fn unwrap_manifold(&self) -> &Manifold {
        match self {
            GeometryTableEntry::Manifold(manifold) => manifold,
            _ => panic!("expected manifold, got: {self:?}")
        }
    }

    pub fn unwrap_cross_section(&self) -> &CrossSection {
        match self {
            GeometryTableEntry::CrossSection(cross_section) => cross_section,
            _ => panic!("expected cross-section, got: {self:?}")
        }
    }
}

#[derive(Debug)]
pub struct GeometryTable {
    table: HashMap<usize, (GeometryTableEntry, GeometryDisposition)>,
    next_index: usize,
}

impl GeometryTable {
    pub fn new() -> Self {
        Self {
            table: HashMap::new(),
            next_index: 1,
        }
    }

    pub fn add(&mut self, geometry: GeometryTableEntry, disposition: GeometryDisposition) -> GeometryTableIndex {
        let idx = self.take_next_index();
        self.table.insert(idx.0, (geometry, disposition));
        idx
    }

    pub fn add_manifold(&mut self, manifold: Manifold, disposition: GeometryDisposition) -> GeometryTableIndex {
        self.add(GeometryTableEntry::Manifold(manifold), disposition)
    }

    pub fn add_cross_section(&mut self, cross_section: CrossSection, disposition: GeometryDisposition) -> GeometryTableIndex {
        self.add(GeometryTableEntry::CrossSection(cross_section), disposition)
    }

    pub fn remove(&mut self, index: GeometryTableIndex) -> (GeometryTableEntry, GeometryDisposition) {
        self.table.remove(&index.0).expect("geometry not in table")
    }

    pub fn get(&self, index: &GeometryTableIndex) -> &GeometryTableEntry {
        &self.table.get(&index.0).expect("geometry not in table").0
    }

    pub fn get_disposition(&self, index: &GeometryTableIndex) -> GeometryDisposition {
        self.table.get(&index.0).expect("geometry not in table").1
    }

    pub fn map(&mut self, index: GeometryTableIndex, func: impl FnOnce(GeometryTableEntry) -> GeometryTableEntry) -> GeometryTableIndex {
        let (manifold, disposition) = self.remove(index);
        self.add(func(manifold), disposition)
    }

    pub fn map_manifold(&mut self, index: GeometryTableIndex, func: impl FnOnce(Manifold) -> Manifold) -> GeometryTableIndex {
        self.map(index, |entry|
            match entry {
                GeometryTableEntry::Manifold(manifold) => GeometryTableEntry::Manifold(func(manifold)),
                _ => panic!("`map_manifold` called on non-manifold geometry")
            }
        )
    }

    pub fn iter_geometry(&self) -> impl Iterator<Item = &(GeometryTableEntry, GeometryDisposition)> {
        self.table.values()
    }

    fn take_next_index(&mut self) -> GeometryTableIndex {
        let idx = GeometryTableIndex(self.next_index);
        self.next_index += 1;
        idx
    }
}
