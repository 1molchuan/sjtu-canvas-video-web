[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true
Set-StrictMode -Version Latest

$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$Staging = Join-Path $Root ".local/release-staging-$PID"
$Output = Join-Path $Root "release"

function Invoke-FrontendChecks {
    Push-Location (Join-Path $Root "frontend")
    try {
        npm ci
        npm run typecheck
        npm run lint
        npm run test
        npm run test:e2e
        npm run build
    } finally {
        Pop-Location
    }
}

function Invoke-RustChecks {
    Push-Location $Root
    try {
        cargo fmt --all -- --check
        cargo test --workspace --all-targets
        cargo build --release -p server
    } finally {
        Pop-Location
    }
}

function Write-VersionFile {
    $Package = Get-Content (Join-Path $Root "frontend/package.json") -Raw | ConvertFrom-Json
    $Cargo = Get-Content (Join-Path $Root "Cargo.toml")
    $VersionLine = $Cargo | Where-Object { $_ -match '^version = "' } | Select-Object -First 1
    if ($null -eq $VersionLine -or $VersionLine -notmatch '"([^"]+)"') {
        throw "Workspace package version is missing"
    }
    $Lines = @(
        "git_sha=$(git -C $Root rev-parse HEAD)"
        "built_at=$([DateTimeOffset]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ'))"
        "frontend_version=$($Package.version)"
        "rust_package_version=$($Matches[1])"
    )
    [IO.File]::WriteAllLines((Join-Path $Staging "VERSION"), $Lines)
}

function New-ReleasePackage {
    New-Item -ItemType Directory -Force -Path (Join-Path $Staging "bin") | Out-Null
    New-Item -ItemType Directory -Force -Path (Join-Path $Staging "frontend") | Out-Null
    New-Item -ItemType Directory -Force -Path (Join-Path $Staging "config") | Out-Null
    Copy-Item (Join-Path $Root "target/release/server.exe") (Join-Path $Staging "bin/sjtu-canvas-video-server.exe")
    Copy-Item (Join-Path $Root "frontend/dist") (Join-Path $Staging "frontend/dist") -Recurse
    Copy-Item (Join-Path $Root "config/production.example.toml") (Join-Path $Staging "config/example.toml")
    Copy-Item (Join-Path $Root "deploy") (Join-Path $Staging "deploy") -Recurse
    Copy-Item (Join-Path $Root "scripts") (Join-Path $Staging "scripts") -Recurse
    Write-VersionFile
    if (Test-Path -LiteralPath $Output) {
        Remove-Item -LiteralPath $Output -Recurse -Force
    }
    Move-Item -LiteralPath $Staging -Destination $Output
}

try {
    Invoke-FrontendChecks
    Invoke-RustChecks
    New-ReleasePackage
    Write-Host "Release package: $Output"
} finally {
    if (Test-Path -LiteralPath $Staging) {
        Remove-Item -LiteralPath $Staging -Recurse -Force
    }
}
