//! mapforge: prompt-free map compiler (gamecraft forge pattern, GIS edition).
//! LLM writes MapSpec JSON (data). This module validates it and compiles
//! a deterministic offline plan (Overpass QL per layer). No network, no DB,
//! no UI. Live execution stays in `dispatch` (opt-in `live=true`), persistence
//! stays in L2. Every failure is a `GisError`, never a panic.
use crate::error::GisError;
use crate::geo::Bbox;
use serde::{Deserialize, Serialize};

/// One layer request inside a MapSpec.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerSpec {
    pub id: String,
    pub key: String,
    pub value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Bounding box in south/west/north/east order (human order).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BboxSpec {
    pub south: f64,
    pub west: f64,
    pub north: f64,
    pub east: f64,
}

/// A forge spec: title + slug + bbox + 1..8 layer requests.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapSpec {
    pub title: String,
    pub slug: String,
    pub bbox: BboxSpec,
    pub layers: Vec<LayerSpec>,
}

/// One compiled layer: validated ids plus deterministic QL.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForgeItem {
    pub id: String,
    pub name: String,
    pub key: String,
    pub value: String,
    pub ql: String,
}

/// Offline plan: slug + title + bbox echo + per-layer QL.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForgePlan {
    pub slug: String,
    pub title: String,
    pub bbox: BboxSpec,
    pub layers: Vec<ForgeItem>,
}

fn require_text(label: &str, raw: &str, max_chars: usize) -> Result<String, GisError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(GisError::InvalidInput(format!("{label} must not be empty")));
    }
    if trimmed.chars().count() > max_chars {
        return Err(GisError::InvalidInput(format!(
            "{label} exceeds {max_chars} chars"
        )));
    }
    if trimmed.chars().any(|c| c.is_control()) {
        return Err(GisError::InvalidInput(format!(
            "{label} must not contain control chars"
        )));
    }
    Ok(trimmed.to_string())
}

fn require_slug(label: &str, raw: &str, max_chars: usize) -> Result<String, GisError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(GisError::InvalidInput(format!("{label} must not be empty")));
    }
    if trimmed.chars().count() > max_chars {
        return Err(GisError::InvalidInput(format!(
            "{label} exceeds {max_chars} chars"
        )));
    }
    let ok = trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !ok {
        return Err(GisError::InvalidInput(format!(
            "{label} must be ascii alphanumeric, '-' or '_'"
        )));
    }
    Ok(trimmed.to_string())
}

/// Parse spec JSON text into a MapSpec. Parse failures are typed errors.
pub fn parse_spec(text: &str) -> Result<MapSpec, GisError> {
    if text.trim().is_empty() {
        return Err(GisError::InvalidInput("spec must not be empty".to_string()));
    }
    serde_json::from_str::<MapSpec>(text)
        .map_err(|e| GisError::InvalidInput(format!("spec parse: {e}")))
}

/// Validate a spec and compile the offline plan (QL per layer).
pub fn compile_plan(spec: &MapSpec) -> Result<ForgePlan, GisError> {
    let title = require_text("title", &spec.title, 120)?;
    let slug = require_slug("slug", &spec.slug, 64)?;
    if spec.layers.is_empty() {
        return Err(GisError::InvalidInput(
            "layers must contain 1..8 entries".to_string(),
        ));
    }
    if spec.layers.len() > 8 {
        return Err(GisError::InvalidInput(
            "layers must contain 1..8 entries".to_string(),
        ));
    }
    let bbox = Bbox::new(
        spec.bbox.west,
        spec.bbox.south,
        spec.bbox.east,
        spec.bbox.north,
    )?;
    let mut seen: Vec<String> = Vec::new();
    let mut items: Vec<ForgeItem> = Vec::new();
    for layer in &spec.layers {
        let id = require_slug("layer id", &layer.id, 64)?;
        if seen.iter().any(|s| s == &id) {
            return Err(GisError::InvalidInput(format!("duplicate layer id: {id}")));
        }
        seen.push(id.clone());
        let display = match &layer.name {
            Some(n) => require_text("layer name", n, 120)?,
            None => format!("{}={}", layer.key.trim(), layer.value.trim()),
        };
        let ql = crate::overpass::build_tag_query(&layer.key, &layer.value, Some(bbox))?;
        let key = layer.key.trim().to_string();
        let value = layer.value.trim().to_string();
        items.push(ForgeItem {
            id,
            name: display,
            key,
            value,
            ql,
        });
    }
    Ok(ForgePlan {
        slug,
        title,
        bbox: spec.bbox,
        layers: items,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_spec() -> MapSpec {
        MapSpec {
            title: "Innere Stadt pharmacies".to_string(),
            slug: "innere-stadt-pharmacies".to_string(),
            bbox: BboxSpec {
                south: 48.195,
                west: 16.355,
                north: 48.225,
                east: 16.395,
            },
            layers: vec![LayerSpec {
                id: "pharmacies".to_string(),
                key: "amenity".to_string(),
                value: "pharmacy".to_string(),
                name: None,
            }],
        }
    }

    #[test]
    fn valid_spec_compiles_one_ql_offline() {
        let plan = compile_plan(&sample_spec());
        assert!(plan.is_ok());
        if let Ok(plan) = plan {
            assert_eq!(plan.layers.len(), 1);
            if let Some(item) = plan.layers.first() {
                assert!(item.ql.contains("amenity"));
                assert!(item.ql.contains("pharmacy"));
                assert!(item.ql.contains("out geom 100"));
            }
        }
    }

    #[test]
    fn hostile_specs_rejected() {
        let mut bad_title = sample_spec();
        bad_title.title = " ".to_string();
        assert!(compile_plan(&bad_title).is_err());
        let mut bad_slug = sample_spec();
        bad_slug.slug = "evil;slug".to_string();
        assert!(compile_plan(&bad_slug).is_err());
        let mut no_layers = sample_spec();
        no_layers.layers = Vec::new();
        assert!(compile_plan(&no_layers).is_err());
        let mut nine = sample_spec();
        nine.layers = (0..9)
            .map(|i| LayerSpec {
                id: format!("l-{i}"),
                key: "amenity".to_string(),
                value: "pharmacy".to_string(),
                name: None,
            })
            .collect();
        assert!(compile_plan(&nine).is_err());
        let mut dup = sample_spec();
        dup.layers.push(LayerSpec {
            id: "pharmacies".to_string(),
            key: "shop".to_string(),
            value: "bicycle".to_string(),
            name: None,
        });
        assert!(compile_plan(&dup).is_err());
        let mut bad_bbox = sample_spec();
        bad_bbox.bbox = BboxSpec {
            south: 48.225,
            west: 16.355,
            north: 48.195,
            east: 16.395,
        };
        assert!(compile_plan(&bad_bbox).is_err());
        let mut injection = sample_spec();
        injection.layers[0].key = "amenity\";".to_string();
        assert!(compile_plan(&injection).is_err());
    }

    #[test]
    fn parse_roundtrip_and_bad_json() {
        let spec = sample_spec();
        let text = serde_json::to_string(&spec).unwrap_or_default();
        assert!(!text.is_empty());
        let back = parse_spec(&text);
        assert!(back.is_ok());
        if let Ok(back) = back {
            assert_eq!(back, spec);
        }
        assert!(parse_spec("").is_err());
        assert!(parse_spec("{not json").is_err());
    }
}
