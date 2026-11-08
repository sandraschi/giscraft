# Per-repo fleet start config for giscraft
# Ports were allocated by fleet-gate/claim_ports.py and registered in
# operations/WEBAPP_PORTS.md. Do not hand-edit them -- re-run claim_ports.py.
# NOTE (Rust ArtCraft): Track A native egui + MCP stdio use ZERO ports.
# 11253/11254 below are reserved for Track B sandraschi webapp parallel only.
@{
    Name         = 'giscraft'
    BackendPort  = 11254
    FrontendPort = 11253
    HealthPath   = '/api/health'
    WebRoot      = 'web_sota'
    Backend = @{
        Kind          = 'cargo-run'
        McpCrate      = 'giscraft-mcp'
        Env           = @{ WEB_PORT = '11254' }
    }
    Frontend = @{
        Kind           = 'vite-npm'
        PackageManager = 'npm'
        PortEnvVar     = 'VITE_PORT'
        ApiTargetEnv   = 'VITE_API_TARGET'
    }
}
