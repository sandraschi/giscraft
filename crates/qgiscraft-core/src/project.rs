//! In-memory QGIS-like project model: layers in draw order, CRS, extent.
use crate::error::GisError;
use serde::{Deserialize, Serialize};

/// Minimal layer record. Style is an opaque JSON blob in v0.1.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Layer {
    pub id: String,
    pub name: String,
    pub source: String,
    pub geom_type: String,
    pub crs: String,
    pub visible: bool,
}

/// Project with draw-order layers (first = bottom, like QGIS Layers panel).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub crs: String,
    pub layers: Vec<Layer>,
}

impl Project {
    pub fn new(name: String) -> Self {
        Self {
            name,
            crs: "EPSG:4326".to_string(),
            layers: Vec::new(),
        }
    }

    pub fn add_layer(&mut self, layer: Layer) -> Result<(), GisError> {
        if layer.id.trim().is_empty() {
            return Err(GisError::InvalidInput(
                "layer id must not be empty".to_string(),
            ));
        }
        if layer.name.trim().is_empty() {
            return Err(GisError::InvalidInput(
                "layer name must not be empty".to_string(),
            ));
        }
        if self.layers.iter().any(|l| l.id == layer.id) {
            return Err(GisError::InvalidInput(format!(
                "duplicate layer id: {}",
                layer.id
            )));
        }
        self.layers.push(layer);
        Ok(())
    }

    pub fn remove_layer(&mut self, id: &str) -> Result<bool, GisError> {
        if id.trim().is_empty() {
            return Err(GisError::InvalidInput(
                "layer id must not be empty".to_string(),
            ));
        }
        let before = self.layers.len();
        self.layers.retain(|l| l.id != id);
        Ok(self.layers.len() != before)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(id: &str) -> Layer {
        Layer {
            id: id.to_string(),
            name: id.to_string(),
            source: "osm".to_string(),
            geom_type: "point".to_string(),
            crs: "EPSG:4326".to_string(),
            visible: true,
        }
    }

    #[test]
    fn duplicate_id_is_typed_error() {
        let mut p = Project::new("wien".to_string());
        let _ = p.add_layer(sample("a"));
        assert!(p.add_layer(sample("a")).is_err());
    }
}
