//! XYZ underlay for the egui canvas.
//! Fetch and PNG decode run on [`super::tile_worker::TileWorker`]. This
//! function only uploads textures that are already decoded and draws them.

use super::tile_worker::{tile_cache_key, TileEvent};
use eframe::egui;
use giscraft_core::tiles;

impl super::GisCraftApp {
    pub(super) fn paint_tile_underlay(
        &mut self,
        ui: &mut egui::Ui,
        painter: &egui::Painter,
        rect: egui::Rect,
    ) {
        let template = super::resolve_tile_template(&self.layers);
        if template != self.tile_template {
            self.tile_template = template.clone();
            self.textures.clear();
            if let Some(worker) = self.tile_worker.as_mut() {
                worker.invalidate();
            }
            self.push_log("basemap switched (cache cleared)".to_string());
        }
        self.upload_ready_tiles(ui);
        let tile_z = (self.zoom.floor() as i64).clamp(0, 19) as u8;
        let (wlng, nlat) =
            super::unproject(rect.min, self.center_lng, self.center_lat, self.zoom, &rect);
        let (elng, slat) =
            super::unproject(rect.max, self.center_lng, self.center_lat, self.zoom, &rect);
        match tiles::bbox_to_tiles(wlng, slat, elng, nlat, tile_z) {
            Ok(list) => {
                let missing: Vec<tiles::Tile> = list
                    .iter()
                    .copied()
                    .filter(|tile| {
                        !self
                            .textures
                            .contains_key(&tile_cache_key(&template, *tile))
                    })
                    .collect();
                if let Some(worker) = self.tile_worker.as_mut() {
                    for tile in missing {
                        let _ = worker.request(tile, &template);
                    }
                }
                for tile in list {
                    let key = tile_cache_key(&template, tile);
                    let Some(handle) = self.textures.get(&key) else {
                        continue;
                    };
                    let (x0, y0, x1, y1) = tiles::tile_bounds(tile);
                    let a =
                        super::project(x0, y1, self.center_lng, self.center_lat, self.zoom, &rect);
                    let b =
                        super::project(x1, y0, self.center_lng, self.center_lat, self.zoom, &rect);
                    painter.image(
                        handle.id(),
                        egui::Rect::from_min_max(a, b),
                        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }
            }
            Err(err) => {
                self.status = format!("tiles: {err}");
            }
        }
        if self
            .tile_worker
            .as_ref()
            .is_some_and(|worker| worker.has_pending())
        {
            ui.ctx().request_repaint();
        }
    }

    fn upload_ready_tiles(&mut self, ui: &mut egui::Ui) {
        let Some(worker) = self.tile_worker.as_mut() else {
            return;
        };
        let events = worker.poll();
        let generation = worker.generation();
        for event in events {
            match event {
                TileEvent::Ready {
                    generation: event_generation,
                    key,
                    width,
                    height,
                    rgba,
                } if event_generation == generation && width > 0 && height > 0 => {
                    let image = egui::ColorImage::from_rgba_unmultiplied(
                        [width as usize, height as usize],
                        &rgba,
                    );
                    let handle =
                        ui.ctx()
                            .load_texture(key.clone(), image, egui::TextureOptions::LINEAR);
                    self.textures.insert(key, handle);
                }
                TileEvent::Failed {
                    generation: event_generation,
                    error,
                    ..
                } if event_generation == generation => {
                    self.status = format!("tile {error}");
                    ui.ctx()
                        .request_repaint_after(std::time::Duration::from_secs(2));
                }
                _ => {}
            }
        }
    }
}
