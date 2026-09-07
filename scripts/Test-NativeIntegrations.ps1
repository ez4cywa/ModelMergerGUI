[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$repositoryRoot = Split-Path -Parent $PSScriptRoot
$features = & cargo tree --manifest-path (Join-Path $repositoryRoot 'rust\Cargo.toml') --locked --target x86_64-pc-windows-gnu -e features -i egui-winit --prefix none
if ($LASTEXITCODE -ne 0) { throw 'Unable to inspect native GUI dependency features.' }
foreach ($requiredFeature in @('links', 'webbrowser', 'clipboard')) {
    if (-not ($features | Where-Object { $_ -match ('^egui-winit feature "' + $requiredFeature + '"') })) {
        throw "Native GUI integration disabled: egui-winit/$requiredFeature. About links and clipboard actions require native platform support."
    }
}
Write-Host 'Native browser and clipboard integrations verified.'
