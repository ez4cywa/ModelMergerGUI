[CmdletBinding()]
param(
    [switch]$AcceptLicense
)

$ErrorActionPreference = 'Stop'

$licenseUrl = 'https://hyperos.mi.com/font/en/download/'
$archiveUrl = 'https://hyperos.mi.com/font-download/MiSans.zip'
$licenseDocumentUrl = 'https://hyperos.mi.com/font-download/MiSans%E5%AD%97%E4%BD%93%E7%9F%A5%E8%AF%86%E4%BA%A7%E6%9D%83%E8%AE%B8%E5%8F%AF%E5%8D%8F%E8%AE%AE.pdf'
$expectedArchiveHash = 'B6AA1FC827035922612DF8EDF36E5609BCA1C5441E25CD57572204569B7B81D9'
$projectRoot = Split-Path -Parent $PSScriptRoot
$targetDirectory = Join-Path $projectRoot 'src\ModelMerger.Gui\Assets\Fonts'
$archivePath = Join-Path ([System.IO.Path]::GetTempPath()) "MiSans-$([Guid]::NewGuid().ToString('N')).zip"

if (-not $AcceptLicense)
{
    throw "Read and accept the MiSans license at $licenseUrl, then rerun with -AcceptLicense."
}

New-Item -ItemType Directory -Path $targetDirectory -Force | Out-Null

try
{
    Write-Host 'Downloading the official MiSans archive...'
    Invoke-WebRequest -Uri $archiveUrl -OutFile $archivePath -UseBasicParsing
    $archiveHash = (Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash
    if ($archiveHash -ne $expectedArchiveHash)
    {
        throw "The MiSans archive checksum changed. Expected $expectedArchiveHash but received $archiveHash. Review the official release before updating this script."
    }

    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $archive = [System.IO.Compression.ZipFile]::OpenRead($archivePath)
    try
    {
        $fonts = @{
            'MiSans/ttf/MiSans-Regular.ttf' = 'MiSans-Regular.ttf'
            'MiSans/ttf/MiSans-Semibold.ttf' = 'MiSans-Semibold.ttf'
            'MiSans/ttf/MiSans-Bold.ttf' = 'MiSans-Bold.ttf'
        }

        foreach ($entryName in $fonts.Keys)
        {
            $entry = $archive.GetEntry($entryName)
            if ($null -eq $entry)
            {
                throw "The official archive does not contain $entryName."
            }

            $destination = Join-Path $targetDirectory $fonts[$entryName]
            [System.IO.Compression.ZipFileExtensions]::ExtractToFile($entry, $destination, $true)
        }
    }
    finally
    {
        $archive.Dispose()
    }

    Invoke-WebRequest `
        -Uri $licenseDocumentUrl `
        -OutFile (Join-Path $targetDirectory 'MiSans-License.pdf') `
        -UseBasicParsing

    Write-Host "MiSans was installed for source builds in $targetDirectory"
}
finally
{
    if (Test-Path -LiteralPath $archivePath)
    {
        Remove-Item -LiteralPath $archivePath -Force
    }
}
