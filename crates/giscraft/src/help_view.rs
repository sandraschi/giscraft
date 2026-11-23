use eframe::egui::{self, RichText};
use giscraft_core::skills::SKILLS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HelpTab {
    #[default]
    GetStarted,
    Functions,
    Usage,
    TipsTricks,
    PitfallsSafety,
    PlannedExtensions,
    OtherCrafts,
    BundledSkills,
    ProjectDocs,
}

impl HelpTab {
    pub const ALL: &'static [HelpTab] = &[
        HelpTab::GetStarted,
        HelpTab::Functions,
        HelpTab::Usage,
        HelpTab::TipsTricks,
        HelpTab::PitfallsSafety,
        HelpTab::PlannedExtensions,
        HelpTab::OtherCrafts,
        HelpTab::BundledSkills,
        HelpTab::ProjectDocs,
    ];

    pub fn label(self) -> &'static str {
        match self {
            HelpTab::GetStarted => "Get Started",
            HelpTab::Functions => "Functions",
            HelpTab::Usage => "Usage",
            HelpTab::TipsTricks => "Tips & Tricks",
            HelpTab::PitfallsSafety => "Pitfalls & Safety",
            HelpTab::PlannedExtensions => "Planned Extensions",
            HelpTab::OtherCrafts => "Other Crafts",
            HelpTab::BundledSkills => "Bundled Skills",
            HelpTab::ProjectDocs => "Project Files & Docs",
        }
    }
}

struct DocEntry {
    name: &'static str,
    summary: &'static str,
    content: &'static str,
}

const PROJECT_DOCS: &[DocEntry] = &[
    DocEntry {
        name: "HELP.md",
        summary: "Comprehensive GISCraft Manual & Spatial Intelligence Handbook",
        content: include_str!("../../../HELP.md"),
    },
    DocEntry {
        name: "README.md",
        summary: "Project overview, install, preview, and crate status",
        content: include_str!("../../../README.md"),
    },
    DocEntry {
        name: "docs/TOOLS.md",
        summary: "Reference for all 18 MCP tools and matching CLI subcommands",
        content: include_str!("../../../docs/TOOLS.md"),
    },
    DocEntry {
        name: "docs/PRD.md",
        summary: "Product Requirements Document",
        content: include_str!("../../../docs/PRD.md"),
    },
    DocEntry {
        name: "docs/TIERED_PLAN.md",
        summary: "Tiered implementation plan across L1/L2/L3",
        content: include_str!("../../../docs/TIERED_PLAN.md"),
    },
    DocEntry {
        name: "docs/ECOSYSTEM.md",
        summary: "Data sources, OSM/GeoJSON/CRS standards, and output targets",
        content: include_str!("../../../docs/ECOSYSTEM.md"),
    },
    DocEntry {
        name: "docs/ARCHITECTURE.md",
        summary: "L1/L2/L3 architecture, command bus, and SQLite project store",
        content: include_str!("../../../docs/ARCHITECTURE.md"),
    },
    DocEntry {
        name: "docs/WRAPPEE.md",
        summary: "Clean-room desktop GIS reference scope and boundary",
        content: include_str!("../../../docs/WRAPPEE.md"),
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaletteAction {
    OpenHelp(HelpTab),
    OpenShortcuts,
    OpenLicenses,
    OpenAbout,
    Refresh,
    ZoomIn,
    ZoomOut,
    FitData,
    GoToPlace,
    ToggleAttributes,
    ResetVienna,
    ToggleChat,
    ShowAllLayers,
    HideAllLayers,
    RemoveSelectedLayer,
    ZoomToSelectedLayer,
    OpenOsmDownload,
    OpenGeocode,
    OpenWkt,
    OpenConfig,
    OpenLlmSettings,
}

pub struct PaletteEntry {
    pub label: &'static str,
    pub shortcut: &'static str,
    pub category: &'static str,
    pub action: PaletteAction,
}

pub const PALETTE_ENTRIES: &[PaletteEntry] = &[
    PaletteEntry {
        label: "Help: Getting Started",
        shortcut: "",
        category: "Help",
        action: PaletteAction::OpenHelp(HelpTab::GetStarted),
    },
    PaletteEntry {
        label: "Help: Functions & Crates",
        shortcut: "",
        category: "Help",
        action: PaletteAction::OpenHelp(HelpTab::Functions),
    },
    PaletteEntry {
        label: "Help: Usage (GUI, CLI & MCP)",
        shortcut: "",
        category: "Help",
        action: PaletteAction::OpenHelp(HelpTab::Usage),
    },
    PaletteEntry {
        label: "Help: Tips & Tricks",
        shortcut: "",
        category: "Help",
        action: PaletteAction::OpenHelp(HelpTab::TipsTricks),
    },
    PaletteEntry {
        label: "Help: Common Pitfalls & Safety",
        shortcut: "",
        category: "Help",
        action: PaletteAction::OpenHelp(HelpTab::PitfallsSafety),
    },
    PaletteEntry {
        label: "Help: Planned Extensions",
        shortcut: "",
        category: "Help",
        action: PaletteAction::OpenHelp(HelpTab::PlannedExtensions),
    },
    PaletteEntry {
        label: "Help: Bundled Skills",
        shortcut: "",
        category: "Help",
        action: PaletteAction::OpenHelp(HelpTab::BundledSkills),
    },
    PaletteEntry {
        label: "Help: Project Files & Docs",
        shortcut: "",
        category: "Help",
        action: PaletteAction::OpenHelp(HelpTab::ProjectDocs),
    },
    PaletteEntry {
        label: "Help: Keyboard Shortcuts",
        shortcut: "F1 / Ctrl+/",
        category: "Help",
        action: PaletteAction::OpenShortcuts,
    },
    PaletteEntry {
        label: "Help: Licenses & Notices",
        shortcut: "",
        category: "Help",
        action: PaletteAction::OpenLicenses,
    },
    PaletteEntry {
        label: "Help: About GISCraft",
        shortcut: "",
        category: "Help",
        action: PaletteAction::OpenAbout,
    },
    PaletteEntry {
        label: "Project: Refresh Layers & Features",
        shortcut: "F5",
        category: "Project",
        action: PaletteAction::Refresh,
    },
    PaletteEntry {
        label: "View: Zoom In",
        shortcut: "Ctrl+=",
        category: "View",
        action: PaletteAction::ZoomIn,
    },
    PaletteEntry {
        label: "View: Zoom Out",
        shortcut: "Ctrl+-",
        category: "View",
        action: PaletteAction::ZoomOut,
    },
    PaletteEntry {
        label: "View: Fit Data to Window",
        shortcut: "Ctrl+0",
        category: "View",
        action: PaletteAction::FitData,
    },
    PaletteEntry {
        label: "View: Go to Place...",
        shortcut: "Ctrl+G",
        category: "View",
        action: PaletteAction::GoToPlace,
    },
    PaletteEntry {
        label: "View: Toggle Attribute Table",
        shortcut: "F6",
        category: "View",
        action: PaletteAction::ToggleAttributes,
    },
    PaletteEntry {
        label: "View: Reset Canvas to Vienna",
        shortcut: "",
        category: "View",
        action: PaletteAction::ResetVienna,
    },
    PaletteEntry {
        label: "AI: Toggle Spatial Chat Dock",
        shortcut: "Ctrl+J",
        category: "AI",
        action: PaletteAction::ToggleChat,
    },
    PaletteEntry {
        label: "Layer: Show All Layers",
        shortcut: "",
        category: "Layer",
        action: PaletteAction::ShowAllLayers,
    },
    PaletteEntry {
        label: "Layer: Hide All Layers",
        shortcut: "",
        category: "Layer",
        action: PaletteAction::HideAllLayers,
    },
    PaletteEntry {
        label: "Layer: Remove Selected Layer",
        shortcut: "Delete",
        category: "Layer",
        action: PaletteAction::RemoveSelectedLayer,
    },
    PaletteEntry {
        label: "Layer: Zoom to Selected Layer",
        shortcut: "",
        category: "Layer",
        action: PaletteAction::ZoomToSelectedLayer,
    },
    PaletteEntry {
        label: "Processing: Download OSM Features (Overpass)...",
        shortcut: "",
        category: "Processing",
        action: PaletteAction::OpenOsmDownload,
    },
    PaletteEntry {
        label: "Processing: Geocode Places to Layer...",
        shortcut: "",
        category: "Processing",
        action: PaletteAction::OpenGeocode,
    },
    PaletteEntry {
        label: "Plugins: Show WKT Geometry...",
        shortcut: "",
        category: "Plugins",
        action: PaletteAction::OpenWkt,
    },
    PaletteEntry {
        label: "Settings: Configuration...",
        shortcut: "",
        category: "Settings",
        action: PaletteAction::OpenConfig,
    },
    PaletteEntry {
        label: "Settings: LLM & Chat Settings...",
        shortcut: "",
        category: "Settings",
        action: PaletteAction::OpenLlmSettings,
    },
];

pub struct HelpState {
    pub open: bool,
    pub active_tab: HelpTab,
    pub selected_skill: usize,
    pub selected_doc: usize,
    pub show_shortcuts: bool,
    pub show_licenses: bool,
    pub show_palette: bool,
    pub palette_query: String,
    pub palette_focus_requested: bool,
}

impl Default for HelpState {
    fn default() -> Self {
        Self {
            open: false,
            active_tab: HelpTab::GetStarted,
            selected_skill: 0,
            selected_doc: 0,
            show_shortcuts: false,
            show_licenses: false,
            show_palette: false,
            palette_query: String::new(),
            palette_focus_requested: false,
        }
    }
}

impl HelpState {
    pub fn open_tab(&mut self, tab: HelpTab) {
        self.active_tab = tab;
        self.open = true;
    }

    pub fn open_palette(&mut self) {
        self.show_palette = true;
        self.palette_query.clear();
        self.palette_focus_requested = true;
    }

    pub fn show_help_center(&mut self, ctx: &egui::Context) {
        if !self.open {
            return;
        }
        let mut keep_open = self.open;
        egui::Window::new("GISCraft Help Center")
            .open(&mut keep_open)
            .default_width(780.0)
            .default_height(540.0)
            .min_width(620.0)
            .min_height(420.0)
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for &tab in HelpTab::ALL {
                        let selected = self.active_tab == tab;
                        if ui.selectable_label(selected, tab.label()).clicked() {
                            self.active_tab = tab;
                        }
                    }
                });
                ui.separator();

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| match self.active_tab {
                        HelpTab::GetStarted => render_get_started(ui),
                        HelpTab::Functions => render_functions(ui),
                        HelpTab::Usage => render_usage(ui),
                        HelpTab::TipsTricks => render_tips_tricks(ui),
                        HelpTab::PitfallsSafety => render_pitfalls_safety(ui),
                        HelpTab::PlannedExtensions => render_planned_extensions(ui),
                        HelpTab::OtherCrafts => render_other_crafts(ui),
                        HelpTab::BundledSkills => {
                            render_bundled_skills(ui, &mut self.selected_skill)
                        }
                        HelpTab::ProjectDocs => render_project_docs(ui, &mut self.selected_doc),
                    });
            });
        self.open = keep_open;
    }

    pub fn show_shortcuts_modal(&mut self, ctx: &egui::Context) {
        if !self.show_shortcuts {
            return;
        }
        let mut keep_open = self.show_shortcuts;
        egui::Window::new("Keyboard Shortcuts")
            .open(&mut keep_open)
            .default_width(520.0)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.heading("Canvas & Navigation");
                shortcut_row(ui, "Left-drag", "Pan map canvas");
                shortcut_row(ui, "Mouse wheel", "Zoom centered at cursor");
                shortcut_row(ui, "Ctrl + =", "Zoom in");
                shortcut_row(ui, "Ctrl + -", "Zoom out");
                shortcut_row(ui, "Ctrl + 0", "Fit all visible data to window");
                shortcut_row(ui, "Ctrl + G", "Go to place (geocoder / lat, lng)");
                shortcut_row(ui, "F5", "Refresh layers & features from DB");

                ui.add_space(8.0);
                ui.heading("Layers & Attribute Table");
                shortcut_row(ui, "F6", "Toggle Attribute Table dock");
                shortcut_row(ui, "Delete", "Remove selected layer");

                ui.add_space(8.0);
                ui.heading("AI Chat & Help");
                shortcut_row(ui, "Ctrl + J", "Toggle AI Spatial Advisor dock");
                shortcut_row(ui, "Ctrl + Shift + P / Ctrl + K", "Open Command Palette");
                shortcut_row(ui, "F1 / Ctrl + /", "Open Keyboard Shortcuts");

                ui.add_space(10.0);
                if ui.button("Close").clicked() {
                    self.show_shortcuts = false;
                }
            });
        if !keep_open {
            self.show_shortcuts = false;
        }
    }

    pub fn show_licenses_modal(&mut self, ctx: &egui::Context) {
        if !self.show_licenses {
            return;
        }
        let mut keep_open = self.show_licenses;
        egui::Window::new("Licenses & Notices")
            .open(&mut keep_open)
            .default_width(580.0)
            .collapsible(false)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.heading("GISCraft Licensing & Safety Guarantees");
                    ui.label("• License: Dual-licensed under MIT OR Apache-2.0.");
                    ui.label("• Zero-Panic Core: All workspace crates enforce #![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo)].");
                    ui.label("• Clean-Room Core: Zero GPL/LGPL/AGPL code is bundled or required.");

                    ui.add_space(8.0);
                    ui.heading("Map Data & External Services");
                    ui.label("• OpenStreetMap Tiles & Overpass Data: (c) OpenStreetMap contributors, licensed under the Open Database License (ODbL).");
                    ui.label("• Photon Geocoder: Powered by Komoot Photon / OpenStreetMap data (keyless).");

                    ui.add_space(8.0);
                    ui.heading("Key Rust Crate Attributions");
                    ui.label("• egui & eframe (MIT OR Apache-2.0) — Immediate-mode native GUI.");
                    ui.label("• rusqlite (MIT) — Bundled SQLite project store.");
                    ui.label("• serde / serde_json (MIT OR Apache-2.0) — Serialization.");
                    ui.label("• tokio (MIT) — Async runtime for MCP/HTTP/network tasks.");
                    ui.label("• reqwest (MIT OR Apache-2.0) — HTTP client for Photon, Overpass, and XYZ tiles.");
                    ui.label("• clap (MIT OR Apache-2.0) — CLI argument parsing.");
                    ui.label("• image (MIT OR Apache-2.0) — Tile PNG decoding.");
                    ui.label("• rfd (MIT) — Native file open/save dialogs.");
                    ui.label("• axum / tower-http (MIT) — Track B HTTP sidecar (giscraft-web).");

                    ui.add_space(10.0);
                    if ui.button("Close").clicked() {
                        self.show_licenses = false;
                    }
                });
            });
        if !keep_open {
            self.show_licenses = false;
        }
    }

    pub fn show_command_palette(&mut self, ctx: &egui::Context) -> Option<PaletteAction> {
        if !self.show_palette {
            return None;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.show_palette = false;
            return None;
        }

        let query_lower = self.palette_query.trim().to_lowercase();
        let matches: Vec<&PaletteEntry> = PALETTE_ENTRIES
            .iter()
            .filter(|e| {
                query_lower.is_empty()
                    || e.label.to_lowercase().contains(&query_lower)
                    || e.category.to_lowercase().contains(&query_lower)
                    || e.shortcut.to_lowercase().contains(&query_lower)
            })
            .collect();

        let mut triggered: Option<PaletteAction> = None;
        let mut keep_open = self.show_palette;

        egui::Window::new("Command Palette")
            .open(&mut keep_open)
            .default_width(520.0)
            .collapsible(false)
            .show(ctx, |ui| {
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut self.palette_query)
                        .hint_text("Type a command (e.g. buffer, geocoder, shortcuts, skills)...")
                        .desired_width(f32::INFINITY),
                );
                if self.palette_focus_requested {
                    resp.request_focus();
                    self.palette_focus_requested = false;
                }
                if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    if let Some(first) = matches.first() {
                        triggered = Some(first.action);
                    }
                }
                ui.separator();
                egui::ScrollArea::vertical()
                    .max_height(320.0)
                    .show(ui, |ui| {
                        if matches.is_empty() {
                            ui.label("No matching commands.");
                        } else {
                            for entry in &matches {
                                ui.horizontal(|ui| {
                                    if ui.button(entry.label).clicked() {
                                        triggered = Some(entry.action);
                                    }
                                    if !entry.shortcut.is_empty() {
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                ui.monospace(entry.shortcut);
                                            },
                                        );
                                    }
                                });
                            }
                        }
                    });
            });

        if triggered.is_some() || !keep_open {
            self.show_palette = false;
        }
        triggered
    }
}

fn shortcut_row(ui: &mut egui::Ui, keys: &str, desc: &str) {
    ui.horizontal(|ui| {
        ui.monospace(format!("{keys:<28}"));
        ui.label(desc);
    });
}

fn render_get_started(ui: &mut egui::Ui) {
    ui.heading("What GISCraft Is & Does");
    ui.label("GISCraft is a local-first, clean-room desktop GIS workbench and spatial processing engine written in pure Rust.");
    ui.label("• Single-File Project DB: All layers, geometries, styles, and CRS live in one SQLite file (giscraft.db).");
    ui.label("• Keyless Open Data: Built-in Photon geocoding, Overpass OSM querying, and OpenStreetMap XYZ basemap tiles.");
    ui.label("• Unified Command Bus: Desktop GUI, --json CLI, and MCP stdio server drive the exact same spatial core.");

    ui.add_space(10.0);
    ui.heading("5-Step First-Run Tour");
    ui.label("1. Launch with Seeded Basemap: Start GISCraft; an empty giscraft.db automatically seeds osm-basemap centered on Vienna.");
    ui.label("2. Geocode & Jump (Ctrl+G): Press Ctrl+G (or View -> Go to place...) to jump to an address, city, or lat, lng coordinate.");
    ui.label("3. Ingest or Import Data: Use Processing -> Download OSM features... (e.g. amenity=pharmacy) or Layer -> Add Vector Layer... (.geojson / .csv).");
    ui.label("4. Analyze & Inspect (F6, Ctrl+J): Run buffers, grids, nearest-neighbor, or WKT previews. Press F6 for the Attribute Table (click rows to highlight) or Ctrl+J for the AI Spatial Advisor.");
    ui.label("5. Export & Share: Select a layer and choose Vector -> Export Layer as GeoJSON... for RFC 7946 output.");
}

fn render_functions(ui: &mut egui::Ui) {
    ui.heading("5-Crate Workspace Architecture");
    ui.label("• giscraft-core: Pure-Rust GIS engine (EPSG:4326/3857 math, ear-clipping triangulation, WKT 2D, GeoJSON/CSV I/O, fenced Overpass QL, Photon geocode, geodesic buffers/grids/nearest, Doppler localization, LLM/skills/i18n).");
    ui.label("• giscraft-store: SQLite project store (layers, draw order, colors, transactional feature storage).");
    ui.label("• giscraft-mcp: CLI (--json) and MCP 2026-07-28 stdio server exposing 18 spatial tools (+ --connect PORT live bridge).");
    ui.label("• giscraft: Native egui 0.37 / eframe desktop workstation with off-thread tile worker and Ctrl+J AI chat dock.");
    ui.label("• giscraft-web: Track B axum HTTP sidecar seed sharing the same core and store.");

    ui.add_space(10.0);
    ui.heading("Core Spatial Capabilities");
    ui.label("• Geocoding (geocode.search): Instant 15-city offline index + live Photon API.");
    ui.label("• Overpass OSM (overpass.query, map.forge): Fenced tag builder, dry-run QL by default, live execution capped at 100 elements.");
    ui.label("• Geodesic Processing: Great-circle buffers (processing.buffer), rect/hex grids (processing.grid), nearest-neighbor hubs (processing.nearest), true-distance circles/ellipses/lines (processing.geodesic).");
    ui.label("• WKT & Doppler: 2D WKT ingestion (wkt.show) and ADS-B Doppler emitter triangulation (doppler.locate).");
}

fn render_usage(ui: &mut egui::Ui) {
    ui.heading("Desktop GUI Workflow");
    ui.label("• Pan with left-drag; zoom at cursor with mouse wheel; Ctrl+0 fits all visible layers; F5 refreshes DB.");
    ui.label(
        "• F6 toggles the Attribute Table; clicking any row highlights that feature on the map.",
    );
    ui.label(
        "• Ctrl+J opens the AI Spatial Advisor dock with explicit confirmation for mutating tools.",
    );

    ui.add_space(8.0);
    ui.heading("CLI (--json) Workflow");
    ui.monospace("giscraft-mcp project-status --json");
    ui.monospace("giscraft-mcp geocode-search --query \"Stephansplatz\" --json");
    ui.monospace("giscraft-mcp overpass-query --key amenity --value pharmacy --south 48.20 --west 16.36 --north 48.22 --east 16.39 --live --ingest --json");
    ui.monospace("giscraft-mcp processing-buffer --layer-id osm-amenity-pharmacy --distance 250 --output-layer pharmacy-250m --json");

    ui.add_space(8.0);
    ui.heading("MCP stdio & Live GUI Bridge");
    ui.label("• Headless: giscraft-mcp mcp (reads/writes $GISCRAFT_DB or ./giscraft.db).");
    ui.label("• Live Bridge: Start GUI with `giscraft -- --control 19250`, then run `giscraft-mcp mcp --connect 19250` so all DB mutations render live on the canvas without SQLite lock contention.");
    ui.label("• 18 MCP Tools: status, project.status, layer.add, layer.list, layer.import, layer.remove, layer.color, features.list, geocode.search, overpass.query, processing.buffer, processing.grid, processing.nearest, processing.geodesic, map.forge, wkt.show, doppler.locate, export.png (currently unsupported).");
}

fn render_tips_tricks(ui: &mut egui::Ui) {
    ui.heading("Practical Workflow Tips");
    ui.label("• Dry-Run Overpass First: overpass.query and map.forge default to live=false, returning safe Overpass QL without hitting the network.");
    ui.label("• Live Agent Editing: Pair `giscraft -- --control 19250` with `giscraft-mcp mcp --connect 19250` to watch agent edits appear live in the desktop viewport.");
    ui.label("• Instant Offline Jumps: Ctrl+G and geocode.search resolve 15 major cities offline with zero network latency.");
    ui.label("• Cartographic Contrast: Use layer.color (#rrggbb) and hex=true grids for readable overlays over OSM basemaps.");
    ui.label("• Custom Endpoints: Override defaults before launch via GISCRAFT_DB, GISCRAFT_TILES_URL, GISCRAFT_PHOTON_URL, and GISCRAFT_OVERPASS_URL.");
}

fn render_pitfalls_safety(ui: &mut egui::Ui) {
    ui.heading("1. CRS / EPSG Projection Mismatches");
    ui.label("• GISCraft stores coordinates in WGS84 (EPSG:4326, degrees) and projects on-the-fly to Web Mercator (EPSG:3857, clamped at ±85.0511°).");
    ui.label("• Importing UTM or EPSG:3857 meter coordinates without reprojecting to EPSG:4326 triggers out-of-range validation errors or plots near Null Island (0, 0).");

    ui.add_space(8.0);
    ui.heading("2. WGS84 (lat, lng) vs. GeoJSON / WKT [lng, lat] Order");
    ui.label("• Human GPS coordinates and Ctrl+G (parse_latlng) use (lat, lng), e.g. `48.2082, 16.3738`.");
    ui.label("• RFC 7946 GeoJSON, WKT (`POINT(16.3738 48.2082)`), and internal LonLat use [lng, lat] (X, Y).");
    ui.label("• Swapping them places European points in the Indian Ocean or fails |lat| <= 90 validation.");

    ui.add_space(8.0);
    ui.heading("3. Large GeoJSON, CSV & Shapefile Memory Bounds");
    ui.label(
        "• Esri Shapefiles (.shp) are not natively parsed; convert to GeoJSON in EPSG:4326 first.",
    );
    ui.label("• Hard Safety Caps in giscraft-core:");
    ui.label("  - GeoJSON & CSV Import: max 10,000 features/rows per file.");
    ui.label("  - processing.grid: max 10,000 cells.");
    ui.label("  - processing.nearest: max 1,000,000 source×target pairs.");
    ui.label("  - processing.geodesic: max 20,000 km line length.");
    ui.label("  - overpass.query live: max 100 elements; features.list: max 500 features.");
    ui.label(
        "  - --control bridge: 127.0.0.1 only, 4 MiB max line, 16 max concurrent connections.",
    );
}

fn render_planned_extensions(ui: &mut egui::Ui) {
    ui.heading("Planned Extensions & Roadmap");
    ui.label("• Headless Cartographic Export (export.png): Wire off-screen PNG/BMP renderer with scale bars and north arrows (currently typed `unsupported`).");
    ui.label("• GeoTIFF & DEM Hillshading: Pure-Rust raster elevation parsing, slope analysis, and contour lines.");
    ui.label("• GPX & KML Track Ingestion: Direct import of hiking tracks, waypoints, and flight telemetry.");
    ui.label("• GeoPackage (.gpkg) & OSM PBF Bulk Loader: Offline regional extracts without external GDAL/C libraries.");
    ui.label("• TravelCraft Chrono-GIS Bridge & Track B Web Console (giscraft-web on ports 11253/11254).");
}

fn render_other_crafts(ui: &mut egui::Ui) {
    ui.heading("Storytold Craft Ecosystem");
    ui.label("• travelcraft: Consumes geocoded POIs, GeoJSON routes, and historical chronomap layers from GISCraft.");
    ui.label("• cadcraft: Precision local millimeter/architectural drafting complementing GISCraft's geodetic EPSG:4326/3857 engine.");
    ui.label(
        "• vectorcraft: Receives exported GeoJSON map outlines for editorial SVG cartography.",
    );
    ui.label("• celcraft / anicraft: Sibling reference implementations of the Storytold Craft Menu Bar & Help Center standard.");
    ui.label("• democraft: Drives giscraft-mcp and --smoke recipes for reproducible walkthroughs.");
}

fn render_bundled_skills(ui: &mut egui::Ui, selected_idx: &mut usize) {
    ui.heading(format!("Bundled Skills ({})", SKILLS.len()));
    if SKILLS.is_empty() {
        ui.label("No bundled skills found.");
        return;
    }
    if *selected_idx >= SKILLS.len() {
        *selected_idx = 0;
    }
    ui.horizontal_wrapped(|ui| {
        for (i, s) in SKILLS.iter().enumerate() {
            if ui.selectable_label(*selected_idx == i, s.name).clicked() {
                *selected_idx = i;
            }
        }
    });
    ui.separator();
    let skill = &SKILLS[*selected_idx];
    ui.label(RichText::new(skill.name).strong());
    ui.monospace(format!(
        "URI: {} ({} bytes)",
        skill.uri,
        skill.content.len()
    ));
    ui.label(skill.description);
    ui.add_space(6.0);
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.monospace(skill.content);
    });
}

fn render_project_docs(ui: &mut egui::Ui, selected_idx: &mut usize) {
    ui.heading(format!("Project Files & Docs ({})", PROJECT_DOCS.len()));
    if *selected_idx >= PROJECT_DOCS.len() {
        *selected_idx = 0;
    }
    ui.horizontal_wrapped(|ui| {
        for (i, doc) in PROJECT_DOCS.iter().enumerate() {
            if ui.selectable_label(*selected_idx == i, doc.name).clicked() {
                *selected_idx = i;
            }
        }
    });
    ui.separator();
    let doc = &PROJECT_DOCS[*selected_idx];
    ui.label(RichText::new(doc.name).strong());
    ui.label(doc.summary);
    ui.add_space(6.0);
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.monospace(doc.content);
    });
}
