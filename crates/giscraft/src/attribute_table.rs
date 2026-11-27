//! Attribute table rows for the selected layer.
//! The dock draws these; the tests stay free of egui.

use eframe::egui;
use giscraft_store::StoredFeature;

#[derive(Debug, Clone, PartialEq)]
pub struct AttrRow {
    pub index: usize,
    pub name: String,
    pub kind: String,
    pub lng: f64,
    pub lat: f64,
}

/// Features of `layer_id`, in project order. `index` is the position in
/// `features`, so a row click can highlight that same feature on the canvas.
pub fn rows_for_layer(features: &[StoredFeature], layer_id: Option<&str>) -> Vec<AttrRow> {
    let Some(layer_id) = layer_id.map(str::trim).filter(|id| !id.is_empty()) else {
        return Vec::new();
    };
    features
        .iter()
        .enumerate()
        .filter_map(|(index, feature)| {
            if feature.layer_id != layer_id {
                return None;
            }
            let name = if feature.name.trim().is_empty() {
                "(unnamed)".to_string()
            } else {
                feature.name.clone()
            };
            let kind = if feature.kind.trim().is_empty() {
                "feature".to_string()
            } else {
                feature.kind.clone()
            };
            Some(AttrRow {
                index,
                name,
                kind,
                lng: feature.lng,
                lat: feature.lat,
            })
        })
        .collect()
}

impl super::GisCraftApp {
    pub(super) fn show_attribute_table(&mut self, ui: &mut egui::Ui) {
        let layer_id = self.selected.clone();
        let title = match layer_id.as_deref() {
            Some(id) => {
                let name = self
                    .layers
                    .iter()
                    .find(|layer| layer.id == id)
                    .map(|layer| layer.name.as_str())
                    .unwrap_or(id);
                format!("Attribute table: {name}")
            }
            None => "Attribute table".to_string(),
        };
        ui.horizontal(|ui| {
            ui.heading(title);
            if ui.button("Close").clicked() {
                self.show_attributes = false;
            }
        });
        let rows = rows_for_layer(&self.features, layer_id.as_deref());
        if layer_id.is_none() {
            ui.label("Select a layer to list its features.");
            return;
        }
        if rows.is_empty() {
            ui.label("This layer has no features.");
            return;
        }
        ui.label(format!(
            "{} features. Click a row to highlight it on the map.",
            rows.len()
        ));
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for row in rows {
                    let label = format!(
                        "{}    {}    {:.5}, {:.5}",
                        row.name, row.kind, row.lat, row.lng
                    );
                    let on = self.selected_feature == Some(row.index);
                    if ui.selectable_label(on, label).clicked() {
                        self.selected_feature = Some(row.index);
                        self.status = format!("{} ({})", row.name, row.kind);
                        self.reveal_feature(row.index);
                    }
                }
            });
    }

    fn reveal_feature(&mut self, index: usize) {
        let Some(feature) = self.features.get(index) else {
            return;
        };
        if !feature.lng.is_finite() || !feature.lat.is_finite() {
            return;
        }
        let lng = feature.lng;
        let lat = feature.lat;
        let outside = match self.last_rect {
            Some(rect) => {
                let pos =
                    super::project(lng, lat, self.center_lng, self.center_lat, self.zoom, &rect);
                !rect.contains(pos)
            }
            None => true,
        };
        if outside {
            self.center_lng = lng;
            self.center_lat = lat;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feature(layer: &str, name: &str, kind: &str, lng: f64, lat: f64) -> StoredFeature {
        StoredFeature {
            layer_id: layer.to_string(),
            name: name.to_string(),
            kind: kind.to_string(),
            lng,
            lat,
            geom_json: None,
        }
    }

    #[test]
    fn rows_keep_source_index_and_skip_other_layers() {
        let features = vec![
            feature("roads", "ring", "way", 16.1, 48.1),
            feature("cafes", "Cafe A", "node", 16.2, 48.2),
            feature("cafes", "", "", 16.3, 48.3),
        ];
        assert!(rows_for_layer(&features, None).is_empty());
        assert!(rows_for_layer(&features, Some("  ")).is_empty());
        let rows = rows_for_layer(&features, Some("cafes"));
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].index, 1);
        assert_eq!(rows[0].name, "Cafe A");
        assert_eq!(rows[0].kind, "node");
        assert_eq!(rows[1].index, 2);
        assert_eq!(rows[1].name, "(unnamed)");
        assert_eq!(rows[1].kind, "feature");
    }
}
