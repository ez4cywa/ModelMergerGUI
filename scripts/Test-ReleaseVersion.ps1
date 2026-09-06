[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^v\d+\.\d+\.\d+$')]
    [string]$Tag,

    [switch]$RequireCleanTree,
    [switch]$RequireHeadTag
)

$ErrorActionPreference = 'Stop'
$repositoryRoot = Split-Path -Parent $PSScriptRoot
$manifestPath = Join-Path $repositoryRoot 'rust\Cargo.toml'
$manifest = Get-Content -LiteralPath $manifestPath -Raw
$match = [regex]::Match($manifest, '(?m)^version\s*=\s*"(?<version>\d+\.\d+\.\d+)"\s*$')
if (-not $match.Success) {
    throw "Unable to read workspace.package.version from $manifestPath"
}
$expectedTag = "v$($match.Groups['version'].Value)"
if ($Tag -ne $expectedTag) {
    throw "Release tag $Tag does not match Cargo workspace version $expectedTag."
}

Push-Location $repositoryRoot
try {
    if ($RequireCleanTree) {
        $changes = @(git status --porcelain=v1 --untracked-files=all)
        if ($LASTEXITCODE -ne 0 -or $changes.Count -ne 0) {
            throw 'Release requires a clean working tree, including untracked files.'
        }
    }
    if ($RequireHeadTag) {
        git rev-parse --verify "refs/tags/$Tag" 2>$null | Out-Null
        if ($LASTEXITCODE -ne 0) {
            throw "Local tag $Tag does not exist."
        }
        $head = (git rev-parse HEAD).Trim()
        $tagCommit = (git rev-list -n 1 $Tag).Trim()
        if ($head -ne $tagCommit) {
            throw "Tag $Tag does not point to HEAD."
        }
    }
}
finally {
    Pop-Location
}

Write-Host "Release identity verified: $Tag"
