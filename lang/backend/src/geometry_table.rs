use std::collections::HashMap;

use manifold_rs::{CrossSection, Manifold};
use yascad_frontend::InputSourceSpan;

use crate::{RuntimeError, RuntimeErrorKind, object::Object};

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

    /// Add new geometry to the table, and return its index.
    pub fn add(&mut self, geometry: GeometryTableEntry, disposition: GeometryDisposition) -> GeometryTableIndex {
        let idx = self.take_next_index();
        self.table.insert(idx.0, (geometry, disposition));
        idx
    }

    /// Like [`Self::add`] but wraps the index in an [`Object`] for easy use in the interpreter.
    pub fn add_into_object(&mut self, geometry: GeometryTableEntry, disposition: GeometryDisposition) -> Object {
        match geometry {
            GeometryTableEntry::Manifold(manifold) =>
                Object::Manifold(self.add_manifold(manifold, disposition)),
            GeometryTableEntry::CrossSection(cross_section) => 
                Object::CrossSection(self.add_cross_section(cross_section, disposition)),
        }
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

    pub fn map_cross_section(&mut self, index: GeometryTableIndex, func: impl FnOnce(CrossSection) -> CrossSection) -> GeometryTableIndex {
        self.map(index, |entry|
            match entry {
                GeometryTableEntry::CrossSection(cross_section) => GeometryTableEntry::CrossSection(func(cross_section)),
                _ => panic!("`map_cross_section` called on non-cross-section geometry")
            }
        )
    }

    /// Remove a list of geometries from the table, union them together, and return details of the
    /// union.
    /// 
    /// The union has NOT yet been added to the table - you can do that yourself if you want.
    /// This method doesn't do that because this method will be used to implement many other
    /// geometry operations which don't want to immediately put the geometry back in the table,
    /// rather use it for another operation.
    /// 
    /// Returns an [`Err`] if the given geometries do not all have the same disposition or
    /// dimension.
    pub fn remove_many_into_union(&mut self, mut indices: Vec<GeometryTableIndex>, span: InputSourceSpan) -> Result<(GeometryTableEntry, GeometryDisposition), RuntimeError> {
        if indices.len() == 1 {
            return Ok(self.remove(indices.remove(0)));
        }

        let (all_entries, all_dispositions): (Vec<_>, Vec<_>) = indices.into_iter()
            .map(|child| self.remove(child))
            .unzip();

        let disposition = GeometryDisposition::flatten(&all_dispositions, span.clone())?;
        
        let (first, rest) = all_entries.split_first().unwrap();
    
        match first {
            GeometryTableEntry::Manifold(first_manifold) => {
                let mut result = first_manifold.clone();
                for entry in rest {
                    let GeometryTableEntry::Manifold(manifold) = entry
                    else { return Err(RuntimeError::new(RuntimeErrorKind::MixedGeometryDimensions, span)) };

                    result = result.union(manifold);
                }
                
                Ok((GeometryTableEntry::Manifold(result), disposition))
            },
            GeometryTableEntry::CrossSection(first_cross_section) => {
                let mut result = first_cross_section.clone();
                for entry in rest {
                    let GeometryTableEntry::CrossSection(cross_section) = entry
                    else { return Err(RuntimeError::new(RuntimeErrorKind::MixedGeometryDimensions, span)) };

                    result = result.union(cross_section);
                }
                
                Ok((GeometryTableEntry::CrossSection(result), disposition))
            },
        }        
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

impl Default for GeometryTable {
    fn default() -> Self {
        Self::new()
    }
}
