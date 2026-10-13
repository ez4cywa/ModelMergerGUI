[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$repositoryRoot = Split-Path -Parent $PSScriptRoot
$manifestPath = Join-Path $repositoryRoot 'rust\Cargo.toml'

& cargo fmt --manifest-path $manifestPath --all -- --check
if ($LASTEXITCODE -ne 0) {
    throw "cargo fmt failed with exit code $LASTEXITCODE."
}

& cargo clippy --manifest-path $manifestPath --locked --workspace --all-targets -- -D warnings
if ($LASTEXITCODE -ne 0) {
    throw "cargo clippy failed with exit code $LASTEXITCODE."
}

& cargo test --manifest-path $manifestPath --locked --workspace
if ($LASTEXITCODE -ne 0) {
    throw "cargo test failed with exit code $LASTEXITCODE."
}

& (Join-Path $PSScriptRoot 'Test-NativeIntegrations.ps1')

Write-Host 'Rust workspace verified: formatting, Clippy, tests and native integrations.'
