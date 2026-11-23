#Requires -Version 5.1
# craft-floor.ps1 - workstation floor for a new Rust craft app.
# VENDORED from mcp-central-docs\templates\craft-app. Do not edit in an app repo.
# start.ps1 runs this when scripts\craft.config.psd1 sets Floor = $true.
# ASCII only. Windows PowerShell 5.1. Standard: CRAFT_START_AND_RELEASE.md
param(
    [string]$RepoRoot = ''
)

$ErrorActionPreference = 'Stop'
if (-not $RepoRoot) { $RepoRoot = Split-Path -Parent $PSScriptRoot }
$name = Split-Path -Leaf $RepoRoot
$missing = New-Object System.Collections.Generic.List[string]

function Add-Gap([string]$text) { $missing.Add($text) | Out-Null }

function Read-Tree([string]$dir, [string]$filter) {
    $text = ''
    if (-not (Test-Path -LiteralPath $dir)) { return $text }
    $files = Get-ChildItem -LiteralPath $dir -Recurse -File -Filter $filter -ErrorAction SilentlyContinue
    foreach ($file in $files) {
        if ($file.FullName -match '\\(target|node_modules|\.git)\\') { continue }
        $text += [IO.File]::ReadAllText($file.FullName)
        $text += "`n"
    }
    return $text
}

foreach ($rel in @(
        'start.ps1',
        'start.bat',
        'scripts\craft-common.ps1',
        'scripts\craft.config.psd1',
        'scripts\craft-floor.ps1',
        'HELP.md',
        'deny.toml',
        'ATTRIBUTION.md',
        'NOTICE',
        'CLAUDE.md',
        'xtask\Cargo.toml',
        'xtask\src\main.rs'
    )) {
    if (-not (Test-Path -LiteralPath (Join-Path $RepoRoot $rel))) { Add-Gap "missing $rel" }
}

$agentsPath = Join-Path $RepoRoot 'AGENTS.md'
if (-not (Test-Path -LiteralPath $agentsPath)) {
    Add-Gap 'missing AGENTS.md'
} else {
    $agentsText = [IO.File]::ReadAllText($agentsPath)
    if ($agentsText -notmatch '<!-- craft-agents:start source=[0-9a-f]{40} sha256=[0-9a-f]{64} -->') {
        Add-Gap 'AGENTS.md missing RFC-0072 craft-agents:start header block'
    }
}

$crates = Join-Path $RepoRoot 'crates'
$hasCore = $false
$hasApp = $false
$hasMcp = $false
if (Test-Path -LiteralPath $crates) {
    $dirs = Get-ChildItem -LiteralPath $crates -Directory -ErrorAction SilentlyContinue
    foreach ($dir in $dirs) {
        if ($dir.Name -like '*-core') { $hasCore = $true }
        if ($dir.Name -like '*-app') { $hasApp = $true }
        if ($dir.Name -like '*-mcp' -or $dir.Name -like '*-cli') { $hasMcp = $true }
    }
}
if (-not $hasCore) { Add-Gap 'missing L1 crate (*-core)' }
if (-not $hasMcp) { Add-Gap 'missing L2 crate (*-mcp or *-cli)' }
if (-not $hasApp) { Add-Gap 'missing L3 crate (*-app)' }

$helpPath = Join-Path $RepoRoot 'HELP.md'
if (Test-Path -LiteralPath $helpPath) {
    $help = [IO.File]::ReadAllText($helpPath)
    foreach ($heading in @('Get Started', 'Functions', 'Usage', 'Tips', 'Other Crafts')) {
        if ($help -notlike "*$heading*") { Add-Gap "HELP.md missing section: $heading" }
    }
}

$appSrc = ''
$coreSrc = ''
if (Test-Path -LiteralPath $crates) {
    $dirs = Get-ChildItem -LiteralPath $crates -Directory -ErrorAction SilentlyContinue
    foreach ($dir in $dirs) {
        $src = Join-Path $dir.FullName 'src'
        if ($dir.Name -like '*-app') { $appSrc += Read-Tree $src '*.rs' }
        if ($dir.Name -like '*-core') { $coreSrc += Read-Tree $src '*.rs' }
    }
}

if ($appSrc -notmatch 'forbid\(unsafe_code\)' -or $coreSrc -notmatch 'forbid\(unsafe_code\)') {
    Add-Gap '#![forbid(unsafe_code)] missing on the app or the core'
}
if ($coreSrc -notmatch 'fn dispatch\(') { Add-Gap 'core has no fn dispatch' }
if ($appSrc -match 'not in the 0\.1 demo') { Add-Gap 'menu still says it is not in the demo' }

foreach ($title in @('File', 'Edit', 'View', 'Insert', 'Modify', 'Commands', 'Window', 'Help')) {
    if ($appSrc -notlike "*`"$title`"*") { Add-Gap "menu missing: $title" }
}
foreach ($item in @('Command Palette', 'Keyboard Shortcuts', 'Licenses', 'About')) {
    if ($appSrc -notlike "*$item*") { Add-Gap "Help missing: $item" }
}
if ($appSrc -notmatch 'Panel::left') { Add-Gap 'no left sidebar (egui::Panel::left)' }
if ($appSrc -notmatch 'Dark' -or $appSrc -notmatch 'Light') { Add-Gap 'theme switch missing Dark and Light' }
$hasLang = ($appSrc -match 'English' -and $appSrc -match 'Deutsch') -or ($appSrc -match 'AppLocale')
if (-not $hasLang) { Add-Gap 'language switch missing (English and Deutsch, or AppLocale)' }
if ($appSrc -notmatch 'override_text_color') { Add-Gap 'readable font colors missing (override_text_color)' }
$hasChat = $appSrc -match 'Chat' -and ($appSrc -match 'LlmConfig' -or $appSrc -match 'craft_ai_core' -or $appSrc -match 'no model selected')
if (-not $hasChat) { Add-Gap 'chat dock missing (Chat plus LlmConfig, craft_ai_core, or an explicit no-model error)' }
if ($appSrc -notmatch 'dispatch\(') { Add-Gap 'the window does not call dispatch' }

$manifestPath = Join-Path $RepoRoot 'assets\MANIFEST.txt'
if (-not (Test-Path -LiteralPath $manifestPath)) {
    Add-Gap 'missing assets\MANIFEST.txt (one domain asset per line: relative-path min-bytes)'
} else {
    $sawImage = $false
    $lines = [IO.File]::ReadAllLines($manifestPath)
    foreach ($line in $lines) {
        $trim = $line.Trim()
        if ($trim.Length -eq 0 -or $trim.StartsWith('#')) { continue }
        $parts = $trim.Split(' ', [StringSplitOptions]::RemoveEmptyEntries)
        if ($parts.Count -lt 2) { Add-Gap "bad manifest line: $trim"; continue }
        $rel = $parts[0]
        $min = 0
        if (-not [int]::TryParse($parts[1], [ref]$min)) { Add-Gap "bad size on manifest line: $trim"; continue }
        $full = Join-Path $RepoRoot ($rel -replace '/', '\')
        if (-not (Test-Path -LiteralPath $full)) { Add-Gap "missing asset $rel"; continue }
        $len = (Get-Item -LiteralPath $full).Length
        if ($len -lt $min) { Add-Gap "asset $rel is $len bytes, floor is $min" }
        if ($rel -match '\.(png|svg|webp)$') { $sawImage = $true }
    }
    if (-not $sawImage) { Add-Gap 'assets\MANIFEST.txt lists no png, svg, or webp for this craft' }
}

if ($missing.Count -eq 0) {
    Write-Host "[craft-floor] ok $name" -ForegroundColor Green
    exit 0
}
Write-Host "[craft-floor] FAIL $name ($($missing.Count))" -ForegroundColor Red
foreach ($gap in $missing) { Write-Host "  - $gap" -ForegroundColor Red }
exit 1
