param(
    [string]$OutputRoot = "artifacts\release",
    [switch]$SkipTests
)

$ErrorActionPreference = 'Stop'
$repositoryRoot = Split-Path -Parent $PSScriptRoot
$resolvedOutputRoot = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot $OutputRoot))
$packageName = 'CastModelMerger-win-x64'
$publishDirectory = Join-Path $resolvedOutputRoot $packageName
$archivePath = Join-Path $resolvedOutputRoot "$packageName.zip"
$rustRoot = Join-Path $repositoryRoot 'rust'
$fontPath = Join-Path $repositoryRoot 'rust\crates\model-merger-gui\assets\fonts\MiSans-Medium.ttf'
$targetTriple = 'x86_64-pc-windows-gnu'

$resolvedPublishDirectory = [System.IO.Path]::GetFullPath($publishDirectory)
$requiredPrefix = $resolvedOutputRoot.TrimEnd([System.IO.Path]::DirectorySeparatorChar) +
    [System.IO.Path]::DirectorySeparatorChar
if (-not $resolvedPublishDirectory.StartsWith($requiredPrefix, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing to clean publish directory outside the selected output root: $resolvedPublishDirectory"
}
if (-not (Test-Path -LiteralPath $fontPath -PathType Leaf)) {
    throw "MiSans Medium is required for the embedded Chinese interface. Run scripts\Install-MiSans.ps1 -AcceptLicense first."
}

if (-not $SkipTests) {
    & cargo clippy --manifest-path (Join-Path $rustRoot 'Cargo.toml') --workspace --all-targets -- -D warnings
    if ($LASTEXITCODE -ne 0) {
        throw "cargo clippy failed with exit code $LASTEXITCODE."
    }
    & cargo test --manifest-path (Join-Path $rustRoot 'Cargo.toml') --workspace
    if ($LASTEXITCODE -ne 0) {
        throw "cargo test failed with exit code $LASTEXITCODE."
    }
}

& cargo build `
    --manifest-path (Join-Path $rustRoot 'Cargo.toml') `
    --release `
    --target $targetTriple `
    --package model-merger-gui `
    --bin CastModelMerger
if ($LASTEXITCODE -ne 0) {
    throw "cargo build failed with exit code $LASTEXITCODE."
}

if (Test-Path -LiteralPath $resolvedPublishDirectory -PathType Container) {
    Get-ChildItem -LiteralPath $resolvedPublishDirectory -Force | Remove-Item -Recurse -Force
}
else {
    New-Item -ItemType Directory -Force -Path $resolvedPublishDirectory | Out-Null
}

$executable = Join-Path $rustRoot "target\$targetTriple\release\CastModelMerger.exe"
if (-not (Test-Path -LiteralPath $executable -PathType Leaf)) {
    throw "Native executable was not found: $executable"
}
$stream = [System.IO.File]::OpenRead($executable)
try {
    $reader = [System.IO.BinaryReader]::new($stream)
    try {
        $stream.Position = 0x3c
        $peOffset = $reader.ReadInt32()
        $stream.Position = $peOffset
        $signature = $reader.ReadUInt32()
        $machine = $reader.ReadUInt16()
        if ($signature -ne 0x00004550 -or $machine -ne 0x8664) {
            throw "Published executable is not a Windows x64 PE image."
        }
    }
    finally {
        $reader.Dispose()
    }
}
finally {
    $stream.Dispose()
}
Copy-Item -LiteralPath $executable -Destination $resolvedPublishDirectory
$publishedExecutable = Join-Path $resolvedPublishDirectory 'CastModelMerger.exe'
& (Join-Path $PSScriptRoot 'Test-WindowsIcon.ps1') -ExecutablePath $publishedExecutable
if ($LASTEXITCODE -ne 0) {
    throw "Windows icon verification failed with exit code $LASTEXITCODE."
}
Copy-Item -LiteralPath (Join-Path $repositoryRoot 'README.md') -Destination $resolvedPublishDirectory
Copy-Item -LiteralPath (Join-Path $repositoryRoot 'LICENSE') -Destination $resolvedPublishDirectory
Copy-Item -LiteralPath (Join-Path $repositoryRoot 'THIRD-PARTY-NOTICES.md') -Destination $resolvedPublishDirectory
$screenshotSource = Join-Path $repositoryRoot 'docs\images\rust-native'
$screenshotDestination = Join-Path $resolvedPublishDirectory 'docs\images\rust-native'
New-Item -ItemType Directory -Force -Path $screenshotDestination | Out-Null
Copy-Item -LiteralPath (Join-Path $screenshotSource 'main-window-zh.png') -Destination $screenshotDestination
Copy-Item -LiteralPath (Join-Path $screenshotSource 'main-window-en.png') -Destination $screenshotDestination
Copy-Item -LiteralPath (Join-Path $screenshotSource 'model-preview-zh.png') -Destination $screenshotDestination
Copy-Item -LiteralPath (Join-Path $screenshotSource 'model-preview-dark-zh.png') -Destination $screenshotDestination
Copy-Item -LiteralPath (Join-Path $screenshotSource 'ammo-fill-zh.png') -Destination $screenshotDestination

Compress-Archive -Path (Join-Path $resolvedPublishDirectory '*') -DestinationPath $archivePath -Force
$hash = Get-FileHash -LiteralPath $archivePath -Algorithm SHA256
$hash | ForEach-Object { "$($_.Hash.ToLowerInvariant())  $packageName.zip" } |
    Set-Content -LiteralPath "$archivePath.sha256" -Encoding ascii
Write-Host "Created Rust-native package: $archivePath"
