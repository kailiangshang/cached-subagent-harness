[CmdletBinding()]
param(
    [string]$CodexHome = $(if ($env:CODEX_HOME) { $env:CODEX_HOME } else { Join-Path $HOME '.codex' }),
    [switch]$Force,
    [ValidateSet('Auto', 'Download', 'Build', 'None')]
    [string]$BinarySource = 'Auto',
    [string]$ReleaseBaseUrl = ''
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Get-PackageVersion {
    param([Parameter(Mandatory)][string]$RepoRoot)
    $ManifestPath = Join-Path $RepoRoot '.codex-plugin/plugin.json'
    if (-not (Test-Path -LiteralPath $ManifestPath -PathType Leaf)) {
        throw "Missing plugin manifest: $ManifestPath"
    }
    $Manifest = Get-Content -LiteralPath $ManifestPath -Raw | ConvertFrom-Json
    $Version = [string]$Manifest.version
    if ($Version -notmatch '^[0-9]+\.[0-9]+\.[0-9]+(?:[-+][0-9A-Za-z.-]+)?$') {
        throw 'Plugin manifest contains an invalid version'
    }
    return $Version
}

function Get-ReleaseTarget {
    $Architecture = [string]$env:PROCESSOR_ARCHITECTURE
    if ($Architecture -notin @('AMD64', 'x86_64')) {
        throw "Unsupported harnessctl Windows architecture: $Architecture"
    }
    return 'x86_64-pc-windows-msvc'
}

function Copy-ReleaseFile {
    param(
        [Parameter(Mandatory)][string]$BaseUrl,
        [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)][string]$Destination
    )
    if (Test-Path -LiteralPath $BaseUrl -PathType Container) {
        $Source = Join-Path $BaseUrl $Name
        if (-not (Test-Path -LiteralPath $Source -PathType Leaf)) {
            throw "Missing local release file: $Name"
        }
        Copy-Item -LiteralPath $Source -Destination $Destination
        return
    }
    Invoke-WebRequest -Uri ($BaseUrl.TrimEnd('/') + '/' + $Name) -OutFile $Destination
}

function Test-ZipEntryIsRegularFile {
    param([Parameter(Mandatory)][System.IO.Compression.ZipArchiveEntry]$Entry)
    $UnixMode = ($Entry.ExternalAttributes -shr 16) -band 0xFFFF
    $UnixType = $UnixMode -band 0xF000
    return $UnixType -eq 0x8000
}

function Install-StagedRuntime {
    param(
        [Parameter(Mandatory)][string]$RuntimeSource,
        [Parameter(Mandatory)][string]$BinDir,
        [Parameter(Mandatory)][ValidateSet('install', 'build')]
        [string]$StageKind
    )
    New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
    $Destination = Join-Path $BinDir 'harnessctl.exe'
    $Staged = Join-Path $BinDir ('.harnessctl.' + $StageKind + '.' + [guid]::NewGuid() + '.exe')
    try {
        if (Test-Path -LiteralPath $Destination -PathType Container) {
            throw 'runtime destination is a directory'
        }
        Copy-Item -LiteralPath $RuntimeSource -Destination $Staged
        Move-Item -LiteralPath $Staged -Destination $Destination -Force
    }
    catch {
        Remove-Item -LiteralPath $Staged -Force -ErrorAction SilentlyContinue
        throw "harnessctl runtime replacement failed: $($_.Exception.Message)"
    }
}

function Install-VerifiedRelease {
    param(
        [Parameter(Mandatory)][string]$Version,
        [Parameter(Mandatory)][string]$BaseUrl,
        [Parameter(Mandatory)][string]$SkillRoot
    )

    $Target = Get-ReleaseTarget
    $Asset = "harnessctl-v$Version-$Target.zip"
    $TempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("harness-release-" + [guid]::NewGuid())
    $ArchivePath = Join-Path $TempRoot $Asset
    $ChecksumPath = Join-Path $TempRoot 'SHA256SUMS'
    $ExtractPath = Join-Path $TempRoot 'extract'
    New-Item -ItemType Directory -Path $ExtractPath -Force | Out-Null

    try {
        try {
            Copy-ReleaseFile -BaseUrl $BaseUrl -Name $Asset -Destination $ArchivePath
            Copy-ReleaseFile -BaseUrl $BaseUrl -Name 'SHA256SUMS' -Destination $ChecksumPath
        }
        catch {
            throw 'harnessctl release download failed'
        }

        $ExpectedDigests = @(
            Get-Content -LiteralPath $ChecksumPath | ForEach-Object {
                if ($_ -match '^([0-9A-Fa-f]{64})  (.+)$' -and $Matches[2] -eq $Asset) {
                    $Matches[1].ToLowerInvariant()
                }
            }
        )
        if ($ExpectedDigests.Count -ne 1) {
            throw 'Release checksum entry is missing, duplicated, or invalid'
        }
        $ActualDigest = (Get-FileHash -LiteralPath $ArchivePath -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($ActualDigest -ne $ExpectedDigests[0]) {
            throw 'harnessctl release checksum mismatch'
        }

        Add-Type -AssemblyName System.IO.Compression.FileSystem
        $Zip = [System.IO.Compression.ZipFile]::OpenRead($ArchivePath)
        try {
            $Members = @($Zip.Entries | ForEach-Object { $_.FullName })
            $UnsafeMemberTypes = @(
                $Zip.Entries | Where-Object {
                    -not (Test-ZipEntryIsRegularFile -Entry $_)
                }
            )
        }
        finally {
            $Zip.Dispose()
        }
        $ExpectedMembers = @('harnessctl.exe', 'LICENSE')
        if ($Members.Count -ne 2 -or
            (Compare-Object -ReferenceObject $ExpectedMembers -DifferenceObject $Members).Count -ne 0 -or
            $UnsafeMemberTypes.Count -ne 0) {
            throw 'harnessctl release archive has an unsafe member set'
        }

        Expand-Archive -LiteralPath $ArchivePath -DestinationPath $ExtractPath
        $RuntimeSource = Join-Path $ExtractPath 'harnessctl.exe'
        $LicenseSource = Join-Path $ExtractPath 'LICENSE'
        if (-not (Test-Path -LiteralPath $RuntimeSource -PathType Leaf) -or
            -not (Test-Path -LiteralPath $LicenseSource -PathType Leaf) -or
            ((Get-Item -LiteralPath $RuntimeSource).Attributes -band [IO.FileAttributes]::ReparsePoint) -or
            ((Get-Item -LiteralPath $LicenseSource).Attributes -band [IO.FileAttributes]::ReparsePoint)) {
            throw 'harnessctl release executable is missing'
        }
        Install-StagedRuntime `
            -RuntimeSource $RuntimeSource `
            -BinDir (Join-Path $SkillRoot 'scripts/bin') `
            -StageKind install
        Write-Output "Installed verified harnessctl $Version for $Target"
    }
    finally {
        Remove-Item -LiteralPath $TempRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
}

function Build-HarnessRuntime {
    param([Parameter(Mandatory)][string]$SkillRoot)
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw 'Cargo is required to build harnessctl'
    }
    $CrateRoot = Join-Path $SkillRoot 'scripts/harnessctl'
    $ManifestPath = Join-Path $CrateRoot 'Cargo.toml'
    & cargo build --release --locked --manifest-path $ManifestPath
    if ($LASTEXITCODE -ne 0) {
        throw 'harnessctl source build failed'
    }
    $RuntimeSource = Join-Path $CrateRoot 'target/release/harnessctl.exe'
    if (-not (Test-Path -LiteralPath $RuntimeSource -PathType Leaf)) {
        throw "Cargo did not produce $RuntimeSource"
    }
    Install-StagedRuntime `
        -RuntimeSource $RuntimeSource `
        -BinDir (Join-Path $SkillRoot 'scripts/bin') `
        -StageKind build
    Write-Output "Built $($SkillRoot)/scripts/bin/harnessctl.exe"
}

function Invoke-HarnessInstall {
    param(
        [Parameter(Mandatory)][string]$RepoRoot,
        [Parameter(Mandatory)][string]$CodexHome,
        [switch]$Force,
        [ValidateSet('Auto', 'Download', 'Build', 'None')]
        [string]$BinarySource = 'Auto',
        [string]$ReleaseBaseUrl = ''
    )

    $SkillSource = Join-Path $RepoRoot 'skills/cached-subagent-harness'
    $SkillsRoot = Join-Path $CodexHome 'skills'
    $SkillRoot = Join-Path $SkillsRoot 'cached-subagent-harness'
    if (-not (Test-Path -LiteralPath (Join-Path $SkillSource 'SKILL.md') -PathType Leaf)) {
        throw "Missing cached-subagent-harness source: $SkillSource"
    }
    New-Item -ItemType Directory -Path $SkillsRoot -Force | Out-Null
    if (Test-Path -LiteralPath $SkillRoot) {
        if (-not $Force) {
            throw "$SkillRoot already exists; rerun with -Force to replace it"
        }
        Remove-Item -LiteralPath $SkillRoot -Recurse -Force
    }
    Copy-Item -LiteralPath $SkillSource -Destination $SkillRoot -Recurse -Force
    Remove-Item -LiteralPath (Join-Path $SkillRoot 'scripts/bin/harnessctl') -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath (Join-Path $SkillRoot 'scripts/bin/harnessctl.exe') -Force -ErrorAction SilentlyContinue
    Write-Output "Installed cached-subagent-harness Skill: $SkillRoot"

    $Version = Get-PackageVersion -RepoRoot $RepoRoot
    if (-not $ReleaseBaseUrl) {
        $ReleaseBaseUrl = "https://github.com/kailiangshang/cached-subagent-harness/releases/download/v$Version"
    }

    switch ($BinarySource) {
        'None' {
            Write-Warning 'harnessctl runtime was not installed (-BinarySource None)'
        }
        'Download' {
            try {
                Install-VerifiedRelease -Version $Version -BaseUrl $ReleaseBaseUrl -SkillRoot $SkillRoot
            }
            catch {
                throw "Verified harnessctl download failed; installed Skill is preserved. $($_.Exception.Message)"
            }
        }
        'Build' {
            try {
                Build-HarnessRuntime -SkillRoot $SkillRoot
            }
            catch {
                throw "harnessctl source build failed; installed Skill is preserved. $($_.Exception.Message)"
            }
        }
        'Auto' {
            try {
                Install-VerifiedRelease -Version $Version -BaseUrl $ReleaseBaseUrl -SkillRoot $SkillRoot
            }
            catch {
                Write-Warning 'Verified harnessctl download unavailable; falling back to locked Cargo build'
                try {
                    Build-HarnessRuntime -SkillRoot $SkillRoot
                }
                catch {
                    throw "No verified harnessctl runtime could be installed; installed Skill is preserved. $($_.Exception.Message)"
                }
            }
        }
    }
    Write-Output 'Restart your CLI runtime to load the installed Skill.'
}

$ScriptRepoRoot = Split-Path -Parent $PSScriptRoot
if ($MyInvocation.InvocationName -ne '.') {
    Invoke-HarnessInstall `
        -RepoRoot $ScriptRepoRoot `
        -CodexHome $CodexHome `
        -Force:$Force `
        -BinarySource $BinarySource `
        -ReleaseBaseUrl $ReleaseBaseUrl
}
