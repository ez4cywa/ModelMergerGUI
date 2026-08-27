$ErrorActionPreference = 'Stop'

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$apiBase = 'https://api.github.com/repos/ez4cywa/ModelMergerGUI'
$tag = 'v2.0.0'

$credentialInput = [string]::Join(
    [Environment]::NewLine,
    @('protocol=https', 'host=github.com', '', ''))
$credentialLines = $credentialInput | git credential fill
if ($LASTEXITCODE -ne 0) {
    throw 'Unable to read GitHub credentials from git credential manager.'
}

$credential = @{}
foreach ($line in $credentialLines) {
    $parts = $line -split '=', 2
    if ($parts.Count -eq 2) {
        $credential[$parts[0]] = $parts[1]
    }
}
$releaseToken = $credential['password']
if ([string]::IsNullOrWhiteSpace($releaseToken)) {
    throw 'GitHub credential manager returned no token.'
}

$headers = @{
    Authorization = "Bearer $releaseToken"
    Accept = 'application/vnd.github+json'
    'X-GitHub-Api-Version' = '2022-11-28'
    'User-Agent' = 'CastModelMerger-Release'
}

$archivePath = Join-Path $repositoryRoot 'artifacts\release\CastModelMerger-win-x64.zip'
$hashPath = "$archivePath.sha256"
foreach ($requiredPath in @($archivePath, $hashPath)) {
    if (-not (Test-Path -LiteralPath $requiredPath -PathType Leaf)) {
        throw "Release asset is missing: $requiredPath"
    }
}
$sha256 = ((Get-Content -LiteralPath $hashPath -Raw).Trim() -split '\s+')[0]

$body = @'
## v2.0.0 中文说明

这是 Cast Model Merger GUI 的全 Rust 原生版本，合并引擎、任务调度、五语界面和 3D 预览均已迁移到 Rust。

### 本次改进

- “添加下一个”和空槽位现在都支持一次多选多个 `.cast` 文件，并按返回顺序填入剩余槽位；每组仍严格限制为 15 个部件。
- 部件卡片改为固定宽度和垂直布局，长文件名自动省略，悬停可查看完整路径，不再挤压右侧设置区。
- “本组状态”改为始终可见的横向百分比进度条，同时保留本地化阶段状态和运行日志。
- 更新中文、English 最新主界面截图，并完成五种语言、键盘焦点和 AccessKit 进度语义检查。
- 单一 Windows x64 原生程序，不需要安装 .NET、Rust 或额外运行环境；预览需要支持 Direct3D 12 的显卡驱动。

下载并完整解压 `CastModelMerger-win-x64.zip`，然后运行 `CastModelMerger.exe`。

SHA-256：`$sha256`

![中文主界面](https://github.com/ez4cywa/ModelMergerGUI/releases/download/$tag/main-window-zh.png)

![English interface](https://github.com/ez4cywa/ModelMergerGUI/releases/download/$tag/main-window-en.png)

![模型预览](https://github.com/ez4cywa/ModelMergerGUI/releases/download/$tag/model-preview-zh.png)

---

## v2.0.0 English notes

This is the fully Rust-native Cast Model Merger GUI. The merge engine, scheduler, five-language UI, settings, and interactive 3D preview now run in one Rust application.

- Add next and every empty slot now open a multi-select `.cast` picker and fill the remaining slots in returned order, up to 15 parts per group.
- Fixed-width vertical part cards truncate long names and show the full path on hover, so file names can no longer squeeze the settings pane.
- Group status now uses an always-visible horizontal percentage progress bar with localized stage text and logs.
- Updated Chinese and English screenshots and rechecked all five languages, keyboard focus, and AccessKit progress semantics.
- The Windows x64 ZIP requires no .NET or Rust installation. A Direct3D 12-capable graphics driver is required for preview rendering.

Extract `CastModelMerger-win-x64.zip` completely and run `CastModelMerger.exe`.

SHA-256: `$sha256`
'@
$body = $body.Replace('$sha256', $sha256).Replace('$tag', $tag)

$release = $null
try {
    $existing = Invoke-RestMethod -Method Get -Uri "$apiBase/releases/tags/$tag" -Headers $headers
    if (-not $existing.draft) {
        throw "A published release already exists for ${tag}: $($existing.html_url)"
    }
    $release = $existing
}
catch {
    if ($_.Exception.Response.StatusCode.value__ -ne 404) {
        throw
    }
}

$payload = @{
    tag_name = $tag
    target_commitish = 'main'
    name = 'Cast Model Merger GUI v2.0.0'
    body = $body
    draft = $true
    prerelease = $false
    generate_release_notes = $false
    make_latest = 'true'
} | ConvertTo-Json -Depth 5

if ($null -eq $release) {
    $release = Invoke-RestMethod -Method Post -Uri "$apiBase/releases" -Headers $headers -ContentType 'application/json' -Body $payload
}

$assets = @(
    @{ Path = $archivePath; Name = 'CastModelMerger-win-x64.zip'; Type = 'application/zip' },
    @{ Path = $hashPath; Name = 'CastModelMerger-win-x64.zip.sha256'; Type = 'text/plain' },
    @{ Path = (Join-Path $repositoryRoot 'THIRD-PARTY-NOTICES.md'); Name = 'THIRD-PARTY-NOTICES.md'; Type = 'text/markdown' },
    @{ Path = (Join-Path $repositoryRoot 'docs\images\rust-native\main-window-zh.png'); Name = 'main-window-zh.png'; Type = 'image/png' },
    @{ Path = (Join-Path $repositoryRoot 'docs\images\rust-native\main-window-en.png'); Name = 'main-window-en.png'; Type = 'image/png' },
    @{ Path = (Join-Path $repositoryRoot 'docs\images\rust-native\model-preview-zh.png'); Name = 'model-preview-zh.png'; Type = 'image/png' }
)

foreach ($existingAsset in @($release.assets)) {
    if ($existingAsset.name -in $assets.Name) {
        Invoke-RestMethod -Method Delete -Uri "$apiBase/releases/assets/$($existingAsset.id)" -Headers $headers | Out-Null
    }
}

$uploadBase = $release.upload_url -replace '\{\?name,label\}$', ''
foreach ($asset in $assets) {
    $resolvedPath = (Resolve-Path -LiteralPath $asset.Path).Path
    $uploadUri = "${uploadBase}?name=$([Uri]::EscapeDataString($asset.Name))"
    $uploaded = Invoke-RestMethod -Method Post -Uri $uploadUri -Headers $headers -ContentType $asset.Type -InFile $resolvedPath
    if ($uploaded.state -ne 'uploaded') {
        throw "Asset upload did not complete: $($asset.Name)"
    }
}

$draftCheck = Invoke-RestMethod -Method Get -Uri "$apiBase/releases/$($release.id)" -Headers $headers
$actualNames = @($draftCheck.assets | ForEach-Object { $_.name })
$missingNames = @($assets.Name | Where-Object { $_ -notin $actualNames })
if ($missingNames.Count -gt 0 -or $draftCheck.assets.Count -ne $assets.Count) {
    throw "Release asset verification failed. Missing: $($missingNames -join ', ')"
}

$publishPayload = @{ draft = $false; make_latest = 'true' } | ConvertTo-Json
$published = Invoke-RestMethod -Method Patch -Uri "$apiBase/releases/$($release.id)" -Headers $headers -ContentType 'application/json' -Body $publishPayload

[pscustomobject]@{
    Tag = $published.tag_name
    Url = $published.html_url
    Draft = $published.draft
    AssetCount = $published.assets.Count
    Assets = ($published.assets.name -join ', ')
} | Format-List
