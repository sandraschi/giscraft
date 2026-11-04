//! QGISCraft Internationalization (i18n) Engine.
//!
//! Follows the fleet standard: typed locale enums, zero-panic string lookups,
//! multi-language translations (EN, DE, FR, ES, JA, KO, ZH),
//! and spatial GIS terminology localization.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AppLocale {
    #[default]
    En, // English (Global Default)
    De, // German (Austrian / German Fleet native)
    Fr, // French
    Es, // Spanish
    Ja, // Japanese
    Ko, // Korean
    Zh, // Chinese
}

impl AppLocale {
    pub fn all() -> &'static [AppLocale] {
        &[
            AppLocale::En,
            AppLocale::De,
            AppLocale::Fr,
            AppLocale::Es,
            AppLocale::Ja,
            AppLocale::Ko,
            AppLocale::Zh,
        ]
    }

    pub fn code(&self) -> &'static str {
        match self {
            AppLocale::En => "en",
            AppLocale::De => "de",
            AppLocale::Fr => "fr",
            AppLocale::Es => "es",
            AppLocale::Ja => "ja",
            AppLocale::Ko => "ko",
            AppLocale::Zh => "zh",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            AppLocale::En => "English (US/UK)",
            AppLocale::De => "Deutsch (DE/AT/CH)",
            AppLocale::Fr => "Français (FR)",
            AppLocale::Es => "Español (ES/LATAM)",
            AppLocale::Ja => "日本語 (JA)",
            AppLocale::Ko => "한국어 (KO)",
            AppLocale::Zh => "中文 (ZH)",
        }
    }

    pub fn flag(&self) -> &'static str {
        match self {
            AppLocale::En => "🇬🇧",
            AppLocale::De => "🇩🇪",
            AppLocale::Fr => "🇫🇷",
            AppLocale::Es => "🇪🇸",
            AppLocale::Ja => "🇯🇵",
            AppLocale::Ko => "🇰🇷",
            AppLocale::Zh => "🇨🇳",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum I18nKey {
    // Menu Bar Top Level
    MenuProject,
    MenuEdit,
    MenuView,
    MenuLayer,
    MenuSettings,
    MenuPlugins,
    MenuVector,
    MenuRaster,
    MenuDatabase,
    MenuWeb,
    MenuProcessing,
    MenuHelp,

    // Project Actions
    ProjectNew,
    ProjectRefresh,
    ProjectQuit,

    // Layer Actions
    LayerAddVector,
    LayerAddCsv,
    LayerRemove,
    LayerZoomTo,
    LayerShowAll,
    LayerHideAll,

    // View Actions
    ViewZoomIn,
    ViewZoomOut,
    ViewFitData,
    ViewResetVienna,

    // Docks & Panels
    DockLayers,
    DockBrowser,
    DockToolbox,
    DockLog,
    StatusReady,

    // Dialogs & Tools
    ToolGeocode,
    ToolOsmDownload,
    ToolBuffer,
    ToolGrid,
    ToolNearest,
    ToolGeodesic,
    ToolDoppler,
}

pub fn t(key: I18nKey, locale: AppLocale) -> &'static str {
    match (key, locale) {
        // Project Menu
        (I18nKey::MenuProject, AppLocale::En) => "Project",
        (I18nKey::MenuProject, AppLocale::De) => "Projekt",
        (I18nKey::MenuProject, AppLocale::Fr) => "Projet",
        (I18nKey::MenuProject, AppLocale::Es) => "Proyecto",
        (I18nKey::MenuProject, AppLocale::Ja) => "プロジェクト",
        (I18nKey::MenuProject, AppLocale::Ko) => "프로젝트",
        (I18nKey::MenuProject, AppLocale::Zh) => "项目",

        // Edit Menu
        (I18nKey::MenuEdit, AppLocale::En) => "Edit",
        (I18nKey::MenuEdit, AppLocale::De) => "Bearbeiten",
        (I18nKey::MenuEdit, AppLocale::Fr) => "Éditer",
        (I18nKey::MenuEdit, AppLocale::Es) => "Editar",
        (I18nKey::MenuEdit, AppLocale::Ja) => "編集",
        (I18nKey::MenuEdit, AppLocale::Ko) => "편집",
        (I18nKey::MenuEdit, AppLocale::Zh) => "编辑",

        // View Menu
        (I18nKey::MenuView, AppLocale::En) => "View",
        (I18nKey::MenuView, AppLocale::De) => "Ansicht",
        (I18nKey::MenuView, AppLocale::Fr) => "Vue",
        (I18nKey::MenuView, AppLocale::Es) => "Ver",
        (I18nKey::MenuView, AppLocale::Ja) => "表示",
        (I18nKey::MenuView, AppLocale::Ko) => "보기",
        (I18nKey::MenuView, AppLocale::Zh) => "视图",

        // Layer Menu
        (I18nKey::MenuLayer, AppLocale::En) => "Layer",
        (I18nKey::MenuLayer, AppLocale::De) => "Layer",
        (I18nKey::MenuLayer, AppLocale::Fr) => "Couche",
        (I18nKey::MenuLayer, AppLocale::Es) => "Capa",
        (I18nKey::MenuLayer, AppLocale::Ja) => "レイヤ",
        (I18nKey::MenuLayer, AppLocale::Ko) => "레이어",
        (I18nKey::MenuLayer, AppLocale::Zh) => "图层",

        // Settings Menu
        (I18nKey::MenuSettings, AppLocale::En) => "Settings",
        (I18nKey::MenuSettings, AppLocale::De) => "Einstellungen",
        (I18nKey::MenuSettings, AppLocale::Fr) => "Paramètres",
        (I18nKey::MenuSettings, AppLocale::Es) => "Configuración",
        (I18nKey::MenuSettings, AppLocale::Ja) => "設定",
        (I18nKey::MenuSettings, AppLocale::Ko) => "설정",
        (I18nKey::MenuSettings, AppLocale::Zh) => "设置",

        // Plugins Menu
        (I18nKey::MenuPlugins, AppLocale::En) => "Plugins",
        (I18nKey::MenuPlugins, AppLocale::De) => "Erweiterungen",
        (I18nKey::MenuPlugins, AppLocale::Fr) => "Extensions",
        (I18nKey::MenuPlugins, AppLocale::Es) => "Complementos",
        (I18nKey::MenuPlugins, AppLocale::Ja) => "プラグイン",
        (I18nKey::MenuPlugins, AppLocale::Ko) => "플러그인",
        (I18nKey::MenuPlugins, AppLocale::Zh) => "插件",

        // Vector Menu
        (I18nKey::MenuVector, AppLocale::En) => "Vector",
        (I18nKey::MenuVector, AppLocale::De) => "Vektor",
        (I18nKey::MenuVector, AppLocale::Fr) => "Vecteur",
        (I18nKey::MenuVector, AppLocale::Es) => "Vectorial",
        (I18nKey::MenuVector, AppLocale::Ja) => "ベクター",
        (I18nKey::MenuVector, AppLocale::Ko) => "벡터",
        (I18nKey::MenuVector, AppLocale::Zh) => "矢量",

        // Raster Menu
        (I18nKey::MenuRaster, AppLocale::En) => "Raster",
        (I18nKey::MenuRaster, AppLocale::De) => "Raster",
        (I18nKey::MenuRaster, AppLocale::Fr) => "Raster",
        (I18nKey::MenuRaster, AppLocale::Es) => "Ráster",
        (I18nKey::MenuRaster, AppLocale::Ja) => "ラスタ",
        (I18nKey::MenuRaster, AppLocale::Ko) => "래스터",
        (I18nKey::MenuRaster, AppLocale::Zh) => "栅格",

        // Database Menu
        (I18nKey::MenuDatabase, AppLocale::En) => "Database",
        (I18nKey::MenuDatabase, AppLocale::De) => "Datenbank",
        (I18nKey::MenuDatabase, AppLocale::Fr) => "Base de données",
        (I18nKey::MenuDatabase, AppLocale::Es) => "Base de datos",
        (I18nKey::MenuDatabase, AppLocale::Ja) => "データベース",
        (I18nKey::MenuDatabase, AppLocale::Ko) => "데이터베이스",
        (I18nKey::MenuDatabase, AppLocale::Zh) => "数据库",

        // Web Menu
        (I18nKey::MenuWeb, AppLocale::En) => "Web",
        (I18nKey::MenuWeb, AppLocale::De) => "Web",
        (I18nKey::MenuWeb, AppLocale::Fr) => "Web",
        (I18nKey::MenuWeb, AppLocale::Es) => "Web",
        (I18nKey::MenuWeb, AppLocale::Ja) => "Web",
        (I18nKey::MenuWeb, AppLocale::Ko) => "웹",
        (I18nKey::MenuWeb, AppLocale::Zh) => "网页",

        // Processing Menu
        (I18nKey::MenuProcessing, AppLocale::En) => "Processing",
        (I18nKey::MenuProcessing, AppLocale::De) => "Verarbeitung",
        (I18nKey::MenuProcessing, AppLocale::Fr) => "Traitement",
        (I18nKey::MenuProcessing, AppLocale::Es) => "Procesos",
        (I18nKey::MenuProcessing, AppLocale::Ja) => "プロセシング",
        (I18nKey::MenuProcessing, AppLocale::Ko) => "공간 처리",
        (I18nKey::MenuProcessing, AppLocale::Zh) => "处理",

        // Help Menu
        (I18nKey::MenuHelp, AppLocale::En) => "Help",
        (I18nKey::MenuHelp, AppLocale::De) => "Hilfe",
        (I18nKey::MenuHelp, AppLocale::Fr) => "Aide",
        (I18nKey::MenuHelp, AppLocale::Es) => "Ayuda",
        (I18nKey::MenuHelp, AppLocale::Ja) => "ヘルプ",
        (I18nKey::MenuHelp, AppLocale::Ko) => "도움말",
        (I18nKey::MenuHelp, AppLocale::Zh) => "帮助",

        // Project Actions
        (I18nKey::ProjectNew, AppLocale::En) => "New (clears project)",
        (I18nKey::ProjectNew, AppLocale::De) => "Neu (Projekt leeren)",
        (I18nKey::ProjectNew, AppLocale::Fr) => "Nouveau (effacer projet)",
        (I18nKey::ProjectNew, AppLocale::Es) => "Nuevo (limpiar proyecto)",
        (I18nKey::ProjectNew, AppLocale::Ja) => "新規 (プロジェクト消去)",
        (I18nKey::ProjectNew, AppLocale::Ko) => "새로 만들기 (프로젝트 초기화)",
        (I18nKey::ProjectNew, AppLocale::Zh) => "新建 (清空项目)",

        (I18nKey::ProjectRefresh, AppLocale::En) => "Refresh (F5)",
        (I18nKey::ProjectRefresh, AppLocale::De) => "Aktualisieren (F5)",
        (I18nKey::ProjectRefresh, AppLocale::Fr) => "Rafraîchir (F5)",
        (I18nKey::ProjectRefresh, AppLocale::Es) => "Actualizar (F5)",
        (I18nKey::ProjectRefresh, AppLocale::Ja) => "更新 (F5)",
        (I18nKey::ProjectRefresh, AppLocale::Ko) => "새로고침 (F5)",
        (I18nKey::ProjectRefresh, AppLocale::Zh) => "刷新 (F5)",

        (I18nKey::ProjectQuit, AppLocale::En) => "Quit",
        (I18nKey::ProjectQuit, AppLocale::De) => "Beenden",
        (I18nKey::ProjectQuit, AppLocale::Fr) => "Quitter",
        (I18nKey::ProjectQuit, AppLocale::Es) => "Salir",
        (I18nKey::ProjectQuit, AppLocale::Ja) => "終了",
        (I18nKey::ProjectQuit, AppLocale::Ko) => "종료",
        (I18nKey::ProjectQuit, AppLocale::Zh) => "退出",

        // Layer Actions
        (I18nKey::LayerAddVector, AppLocale::En) => "Add Vector Layer...",
        (I18nKey::LayerAddVector, AppLocale::De) => "Vektorlayer hinzufügen...",
        (I18nKey::LayerAddVector, AppLocale::Fr) => "Ajouter une couche vecteur...",
        (I18nKey::LayerAddVector, AppLocale::Es) => "Añadir capa vectorial...",
        (I18nKey::LayerAddVector, AppLocale::Ja) => "ベクタレイヤの追加...",
        (I18nKey::LayerAddVector, AppLocale::Ko) => "벡터 레이어 추가...",
        (I18nKey::LayerAddVector, AppLocale::Zh) => "添加矢量图层...",

        (I18nKey::LayerAddCsv, AppLocale::En) => "Add Delimited Text Layer...",
        (I18nKey::LayerAddCsv, AppLocale::De) => "Textgetrennte Layerdatei hinzufügen...",
        (I18nKey::LayerAddCsv, AppLocale::Fr) => "Ajouter une couche texte délimité...",
        (I18nKey::LayerAddCsv, AppLocale::Es) => "Añadir capa de texto delimitado...",
        (I18nKey::LayerAddCsv, AppLocale::Ja) => "CSV/区切りテキストレイヤの追加...",
        (I18nKey::LayerAddCsv, AppLocale::Ko) => "구분자로 분리된 텍스트 레이어 추가...",
        (I18nKey::LayerAddCsv, AppLocale::Zh) => "添加文本分隔图层 (CSV)...",

        (I18nKey::LayerRemove, AppLocale::En) => "Remove selected (Del)",
        (I18nKey::LayerRemove, AppLocale::De) => "Ausgewählte entfernen (Entf)",
        (I18nKey::LayerRemove, AppLocale::Fr) => "Supprimer la sélection (Suppr)",
        (I18nKey::LayerRemove, AppLocale::Es) => "Eliminar seleccionado (Supr)",
        (I18nKey::LayerRemove, AppLocale::Ja) => "選択レイヤを削除 (Del)",
        (I18nKey::LayerRemove, AppLocale::Ko) => "선택 레이어 삭제 (Del)",
        (I18nKey::LayerRemove, AppLocale::Zh) => "移除选中的图层 (Del)",

        (I18nKey::LayerZoomTo, AppLocale::En) => "Zoom to selected",
        (I18nKey::LayerZoomTo, AppLocale::De) => "Auf ausgewählte zoomen",
        (I18nKey::LayerZoomTo, AppLocale::Fr) => "Zoomer sur la sélection",
        (I18nKey::LayerZoomTo, AppLocale::Es) => "Zoom a la selección",
        (I18nKey::LayerZoomTo, AppLocale::Ja) => "選択レイヤにズーム",
        (I18nKey::LayerZoomTo, AppLocale::Ko) => "선택 레이어로 줌",
        (I18nKey::LayerZoomTo, AppLocale::Zh) => "缩放到选中的图层",

        (I18nKey::LayerShowAll, AppLocale::En) => "Show all",
        (I18nKey::LayerShowAll, AppLocale::De) => "Alle anzeigen",
        (I18nKey::LayerShowAll, AppLocale::Fr) => "Tout afficher",
        (I18nKey::LayerShowAll, AppLocale::Es) => "Mostrar todos",
        (I18nKey::LayerShowAll, AppLocale::Ja) => "すべて表示",
        (I18nKey::LayerShowAll, AppLocale::Ko) => "모두 표시",
        (I18nKey::LayerShowAll, AppLocale::Zh) => "显示全部",

        (I18nKey::LayerHideAll, AppLocale::En) => "Hide all",
        (I18nKey::LayerHideAll, AppLocale::De) => "Alle ausblenden",
        (I18nKey::LayerHideAll, AppLocale::Fr) => "Tout masquer",
        (I18nKey::LayerHideAll, AppLocale::Es) => "Ocultar todos",
        (I18nKey::LayerHideAll, AppLocale::Ja) => "すべて非表示",
        (I18nKey::LayerHideAll, AppLocale::Ko) => "모두 숨기기",
        (I18nKey::LayerHideAll, AppLocale::Zh) => "隐藏全部",

        // View Actions
        (I18nKey::ViewZoomIn, AppLocale::En) => "Zoom in (Ctrl++)",
        (I18nKey::ViewZoomIn, AppLocale::De) => "Vergrößern (Ctrl++)",
        (I18nKey::ViewZoomIn, AppLocale::Fr) => "Zoom avant (Ctrl++)",
        (I18nKey::ViewZoomIn, AppLocale::Es) => "Acercar (Ctrl++)",
        (I18nKey::ViewZoomIn, AppLocale::Ja) => "拡大 (Ctrl++)",
        (I18nKey::ViewZoomIn, AppLocale::Ko) => "확대 (Ctrl++)",
        (I18nKey::ViewZoomIn, AppLocale::Zh) => "放大 (Ctrl++)",

        (I18nKey::ViewZoomOut, AppLocale::En) => "Zoom out (Ctrl+-)",
        (I18nKey::ViewZoomOut, AppLocale::De) => "Verkleinern (Ctrl+-)",
        (I18nKey::ViewZoomOut, AppLocale::Fr) => "Zoom arrière (Ctrl+-)",
        (I18nKey::ViewZoomOut, AppLocale::Es) => "Alejar (Ctrl+-)",
        (I18nKey::ViewZoomOut, AppLocale::Ja) => "縮小 (Ctrl+-)",
        (I18nKey::ViewZoomOut, AppLocale::Ko) => "축소 (Ctrl+-)",
        (I18nKey::ViewZoomOut, AppLocale::Zh) => "缩小 (Ctrl+-)",

        (I18nKey::ViewFitData, AppLocale::En) => "Fit data (Ctrl+0)",
        (I18nKey::ViewFitData, AppLocale::De) => "An Daten anpassen (Ctrl+0)",
        (I18nKey::ViewFitData, AppLocale::Fr) => "Ajuster aux données (Ctrl+0)",
        (I18nKey::ViewFitData, AppLocale::Es) => "Ajustar a datos (Ctrl+0)",
        (I18nKey::ViewFitData, AppLocale::Ja) => "全域表示 (Ctrl+0)",
        (I18nKey::ViewFitData, AppLocale::Ko) => "데이터 영역 맞추기 (Ctrl+0)",
        (I18nKey::ViewFitData, AppLocale::Zh) => "缩放到图层范围 (Ctrl+0)",

        (I18nKey::ViewResetVienna, AppLocale::En) => "Reset view (Wien)",
        (I18nKey::ViewResetVienna, AppLocale::De) => "Ansicht zurücksetzen (Wien)",
        (I18nKey::ViewResetVienna, AppLocale::Fr) => "Réinitialiser la vue (Vienne)",
        (I18nKey::ViewResetVienna, AppLocale::Es) => "Restablecer vista (Viena)",
        (I18nKey::ViewResetVienna, AppLocale::Ja) => "表示をリセット (ウィーン)",
        (I18nKey::ViewResetVienna, AppLocale::Ko) => "뷰 리셋 (비엔나)",
        (I18nKey::ViewResetVienna, AppLocale::Zh) => "重置视图 (维也纳)",

        // Docks
        (I18nKey::DockLayers, AppLocale::En) => "Layers",
        (I18nKey::DockLayers, AppLocale::De) => "Layer",
        (I18nKey::DockLayers, AppLocale::Fr) => "Couches",
        (I18nKey::DockLayers, AppLocale::Es) => "Capas",
        (I18nKey::DockLayers, AppLocale::Ja) => "レイヤ一覧",
        (I18nKey::DockLayers, AppLocale::Ko) => "레이어 패널",
        (I18nKey::DockLayers, AppLocale::Zh) => "图层面板",

        (I18nKey::DockBrowser, AppLocale::En) => "Browser",
        (I18nKey::DockBrowser, AppLocale::De) => "Browser",
        (I18nKey::DockBrowser, AppLocale::Fr) => "Explorateur",
        (I18nKey::DockBrowser, AppLocale::Es) => "Navegador",
        (I18nKey::DockBrowser, AppLocale::Ja) => "ブラウザ",
        (I18nKey::DockBrowser, AppLocale::Ko) => "탐색기",
        (I18nKey::DockBrowser, AppLocale::Zh) => "资源浏览",

        (I18nKey::DockToolbox, AppLocale::En) => "Processing Toolbox",
        (I18nKey::DockToolbox, AppLocale::De) => "Verarbeitungswerkzeuge",
        (I18nKey::DockToolbox, AppLocale::Fr) => "Boîte à outils de traitement",
        (I18nKey::DockToolbox, AppLocale::Es) => "Caja de herramientas de procesado",
        (I18nKey::DockToolbox, AppLocale::Ja) => "プロセシングツールボックス",
        (I18nKey::DockToolbox, AppLocale::Ko) => "공간 처리 툴박스",
        (I18nKey::DockToolbox, AppLocale::Zh) => "空间处理工具箱",

        (I18nKey::DockLog, AppLocale::En) => "Log Messages",
        (I18nKey::DockLog, AppLocale::De) => "Protokollmeldungen",
        (I18nKey::DockLog, AppLocale::Fr) => "Messages du journal",
        (I18nKey::DockLog, AppLocale::Es) => "Mensajes de registro",
        (I18nKey::DockLog, AppLocale::Ja) => "ログメッセージ",
        (I18nKey::DockLog, AppLocale::Ko) => "로그 메시지",
        (I18nKey::DockLog, AppLocale::Zh) => "日志消息",

        (I18nKey::StatusReady, AppLocale::En) => "Ready",
        (I18nKey::StatusReady, AppLocale::De) => "Bereit",
        (I18nKey::StatusReady, AppLocale::Fr) => "Prêt",
        (I18nKey::StatusReady, AppLocale::Es) => "Listo",
        (I18nKey::StatusReady, AppLocale::Ja) => "準備完了",
        (I18nKey::StatusReady, AppLocale::Ko) => "준비됨",
        (I18nKey::StatusReady, AppLocale::Zh) => "就绪",

        // Tools
        (I18nKey::ToolGeocode, AppLocale::En) => "Geocode Places",
        (I18nKey::ToolGeocode, AppLocale::De) => "Orte Geocodieren",
        (I18nKey::ToolGeocode, AppLocale::Fr) => "Géocodage de lieux",
        (I18nKey::ToolGeocode, AppLocale::Es) => "Geocodificar lugares",
        (I18nKey::ToolGeocode, AppLocale::Ja) => "地名ジオコーディング",
        (I18nKey::ToolGeocode, AppLocale::Ko) => "지명 지오코딩",
        (I18nKey::ToolGeocode, AppLocale::Zh) => "地名地理编码",

        (I18nKey::ToolOsmDownload, AppLocale::En) => "Download OSM Features",
        (I18nKey::ToolOsmDownload, AppLocale::De) => "OSM-Daten herunterladen",
        (I18nKey::ToolOsmDownload, AppLocale::Fr) => "Télécharger des entités OSM",
        (I18nKey::ToolOsmDownload, AppLocale::Es) => "Descargar datos de OSM",
        (I18nKey::ToolOsmDownload, AppLocale::Ja) => "OSM地物ダウンロード",
        (I18nKey::ToolOsmDownload, AppLocale::Ko) => "OSM 데이터 다운로드",
        (I18nKey::ToolOsmDownload, AppLocale::Zh) => "下载 OSM 地理要素",

        (I18nKey::ToolBuffer, AppLocale::En) => "Buffer Points",
        (I18nKey::ToolBuffer, AppLocale::De) => "Punkte puffern",
        (I18nKey::ToolBuffer, AppLocale::Fr) => "Tamponner les points",
        (I18nKey::ToolBuffer, AppLocale::Es) => "Crear buffer de puntos",
        (I18nKey::ToolBuffer, AppLocale::Ja) => "ポイントバッファ作成",
        (I18nKey::ToolBuffer, AppLocale::Ko) => "포인트 버퍼 생성",
        (I18nKey::ToolBuffer, AppLocale::Zh) => "创建点缓冲区",

        (I18nKey::ToolGrid, AppLocale::En) => "Create Grid",
        (I18nKey::ToolGrid, AppLocale::De) => "Gitter erstellen",
        (I18nKey::ToolGrid, AppLocale::Fr) => "Créer une grille",
        (I18nKey::ToolGrid, AppLocale::Es) => "Crear cuadrícula",
        (I18nKey::ToolGrid, AppLocale::Ja) => "グリッド作成",
        (I18nKey::ToolGrid, AppLocale::Ko) => "격자 생성",
        (I18nKey::ToolGrid, AppLocale::Zh) => "创建网格",

        (I18nKey::ToolNearest, AppLocale::En) => "Nearest Neighbor",
        (I18nKey::ToolNearest, AppLocale::De) => "Nächster Nachbar",
        (I18nKey::ToolNearest, AppLocale::Fr) => "Plus proche voisin",
        (I18nKey::ToolNearest, AppLocale::Es) => "Vecino más cercano",
        (I18nKey::ToolNearest, AppLocale::Ja) => "最近傍探索 (ハブライン)",
        (I18nKey::ToolNearest, AppLocale::Ko) => "최근접 이웃 분석",
        (I18nKey::ToolNearest, AppLocale::Zh) => "最近邻分析",

        (I18nKey::ToolGeodesic, AppLocale::En) => "Geodesic Shape",
        (I18nKey::ToolGeodesic, AppLocale::De) => "Geodätische Form",
        (I18nKey::ToolGeodesic, AppLocale::Fr) => "Forme géodésique",
        (I18nKey::ToolGeodesic, AppLocale::Es) => "Forma geodésica",
        (I18nKey::ToolGeodesic, AppLocale::Ja) => "測地線図形 (円/楕円)",
        (I18nKey::ToolGeodesic, AppLocale::Ko) => "측지선 도형 생성",
        (I18nKey::ToolGeodesic, AppLocale::Zh) => "测地线几何体",

        (I18nKey::ToolDoppler, AppLocale::En) => "Doppler Locate",
        (I18nKey::ToolDoppler, AppLocale::De) => "Doppler-Ortung",
        (I18nKey::ToolDoppler, AppLocale::Fr) => "Localisation Doppler",
        (I18nKey::ToolDoppler, AppLocale::Es) => "Localización Doppler",
        (I18nKey::ToolDoppler, AppLocale::Ja) => "ドップラー電波測位",
        (I18nKey::ToolDoppler, AppLocale::Ko) => "도플러 신호 위치추정",
        (I18nKey::ToolDoppler, AppLocale::Zh) => "多普勒无线电定位",
    }
}
