param(
    [string]$OutputRoot = "artifacts\release"
)

$ErrorActionPreference = 'Stop'
$repositoryRoot = Split-Path -Parent $PSScriptRoot
$resolvedOutputRoot = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot $OutputRoot))
$publishDirectory = Join-Path $resolvedOutputRoot 'CastModelMerger-portable-win-x64'
$archivePath = Join-Path $resolvedOutputRoot 'CastModelMerger-portable-win-x64.zip'
$projectPath = Join-Path $repositoryRoot 'src\ModelMerger.Gui\ModelMerger.Gui.csproj'

$resolvedPublishDirectory = [System.IO.Path]::GetFullPath($publishDirectory)
$requiredPrefix = $resolvedOutputRoot.TrimEnd([System.IO.Path]::DirectorySeparatorChar) +
    [System.IO.Path]::DirectorySeparatorChar
if (-not $resolvedPublishDirectory.StartsWith($requiredPrefix, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing to clean publish directory outside the selected output root: $resolvedPublishDirectory"
}
if (Test-Path -LiteralPath $resolvedPublishDirectory -PathType Container) {
    Get-ChildItem -LiteralPath $resolvedPublishDirectory -Force | Remove-Item -Recurse -Force
}
else {
    New-Item -ItemType Directory -Force -Path $resolvedPublishDirectory | Out-Null
}

dotnet publish $projectPath `
    -c Release `
    -r win-x64 `
    -p:SelfContained=false `
    -o $publishDirectory
if ($LASTEXITCODE -ne 0) {
    throw "dotnet publish failed with exit code $LASTEXITCODE."
}

$mainExecutable = Join-Path $publishDirectory 'CastModelMerger.exe'
$portableExecutable = Join-Path $publishDirectory 'CastModelMerger-portable-win-x64.exe'
$workerExecutable = Join-Path $publishDirectory 'model-merger-worker.exe'
if (-not (Test-Path -LiteralPath $mainExecutable -PathType Leaf)) {
    throw "Published GUI executable was not found: $mainExecutable"
}
if (-not (Test-Path -LiteralPath $workerExecutable -PathType Leaf)) {
    throw "Published Rust worker was not found: $workerExecutable"
}

Get-ChildItem -LiteralPath $publishDirectory -File |
    Where-Object Extension -In '.pdb', '.xml' |
    Remove-Item -Force
Move-Item -LiteralPath $mainExecutable -Destination $portableExecutable -Force
Compress-Archive -Path (Join-Path $publishDirectory '*') -DestinationPath $archivePath -Force
Write-Host "Created framework-dependent package: $archivePath"
