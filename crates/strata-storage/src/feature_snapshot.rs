//! Owned rows for index work; stored values follow target/cpp-ref/src/app/MainWindowEditing.cpp:782-920.
use super::FeatureRecord;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SharedFeatureRows(Arc<Vec<FeatureRecord>>);

impl From<Vec<FeatureRecord>> for SharedFeatureRows {
    fn from(records: Vec<FeatureRecord>) -> Self {
        Self(Arc::new(records))
    }
}

impl Deref for SharedFeatureRows {
    type Target = Vec<FeatureRecord>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SharedFeatureRows {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Arc::make_mut(&mut self.0)
    }
}

impl Serialize for SharedFeatureRows {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.deref().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SharedFeatureRows {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Vec::<FeatureRecord>::deserialize(deserializer).map(Self::from)
    }
}

impl<'a> IntoIterator for &'a SharedFeatureRows {
    type Item = &'a FeatureRecord;
    type IntoIter = std::slice::Iter<'a, FeatureRecord>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a> IntoIterator for &'a mut SharedFeatureRows {
    type Item = &'a mut FeatureRecord;
    type IntoIter = std::slice::IterMut<'a, FeatureRecord>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

#[derive(Clone, Debug)]
pub struct FeatureSnapshot(SharedFeatureRows);

impl FeatureSnapshot {
    pub(super) fn new(records: SharedFeatureRows) -> Self {
        Self(records)
    }
    pub fn records(&self) -> &[FeatureRecord] {
        &self.0
    }
    // Record identity only; table identity, CRS and target scope belong to the index request (SNAP1A).
    pub fn same_revision(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0 .0, &other.0 .0)
    }
}
