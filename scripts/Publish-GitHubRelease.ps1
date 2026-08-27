param(
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^v\d+\.\d+\.\d+$')]
    [string]$Tag,

    [Parameter(Mandatory = $true)]
    [string]$NotesPath
)

$ErrorActionPreference = 'Stop'

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$apiBase = 'https://api.github.com/repos/ez4cywa/ModelMergerGUI'
$resolvedNotesPath = if ([System.IO.Path]::IsPathRooted($NotesPath)) {
    $NotesPath
}
else {
    Join-Path $repositoryRoot $NotesPath
}
if (-not (Test-Path -LiteralPath $resolvedNotesPath -PathType Leaf)) {
    throw "Release notes are missing: $resolvedNotesPath"
}

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
$body = (Get-Content -LiteralPath $resolvedNotesPath -Raw)
$body = $body.Replace('{{SHA256}}', $sha256).Replace('{{TAG}}', $Tag)

$release = $null
try {
    $existing = Invoke-RestMethod -Method Get -Uri "$apiBase/releases/tags/$Tag" -Headers $headers
    if (-not $existing.draft) {
        throw "A published release already exists for ${Tag}: $($existing.html_url)"
    }
    $release = $existing
}
catch {
    if ($null -eq $_.Exception.Response -or $_.Exception.Response.StatusCode.value__ -ne 404) {
        throw
    }
}

$payload = @{
    tag_name = $Tag
    target_commitish = 'main'
    name = "Cast Model Merger GUI $Tag"
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
    @{ Path = $archivePath; Name = 'CastModelMerger-win-x64.zip'; Type = 'application/zip' }
)
$assetNames = @($assets | ForEach-Object { $_.Name })

foreach ($existingAsset in @($release.assets)) {
    Invoke-RestMethod -Method Delete -Uri "$apiBase/releases/assets/$($existingAsset.id)" -Headers $headers | Out-Null
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
$missingNames = @($assetNames | Where-Object { $_ -notin $actualNames })
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
