$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$RepoRoot = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'install.ps1')
Add-Type -AssemblyName System.IO.Compression.FileSystem

function Assert-Equal {
    param($Actual, $Expected, [string]$Message)
    if ($Actual -ne $Expected) {
        throw "$Message expected=$Expected actual=$Actual"
    }
}

function Assert-True {
    param([bool]$Value, [string]$Message)
    if (-not $Value) {
        throw $Message
    }
}

function Assert-False {
    param([bool]$Value, [string]$Message)
    if ($Value) {
        throw $Message
    }
}

function Assert-Throws {
    param(
        [scriptblock]$Action,
        [string]$Contains,
        [string]$Message
    )
    $Caught = $null
    try {
        & $Action
    }
    catch {
        $Caught = $_.Exception.Message
    }
    if ($null -eq $Caught) {
        throw "$Message did not throw"
    }
    if ($Contains -and $Caught -notlike "*$Contains*") {
        throw "$Message expected error containing '$Contains', actual='$Caught'"
    }
}

function ConvertTo-SignedAttributes {
    param([uint32]$Value)
    return [BitConverter]::ToInt32([BitConverter]::GetBytes($Value), 0)
}

function Set-RegularZipAttributes {
    param([Parameter(Mandatory)][string]$ArchivePath)
    $Zip = [System.IO.Compression.ZipFile]::Open(
        $ArchivePath,
        [System.IO.Compression.ZipArchiveMode]::Update
    )
    try {
        foreach ($Entry in $Zip.Entries) {
            $Mode = if ($Entry.FullName -eq 'harnessctl.exe') {
                [uint32]2179792896
            }
            else {
                [uint32]2175008768
            }
            $Entry.ExternalAttributes = ConvertTo-SignedAttributes -Value $Mode
        }
    }
    finally {
        $Zip.Dispose()
    }
}

function Set-ReleaseChecksum {
    param([Parameter(Mandatory)]$Fixture)
    $Digest = (Get-FileHash -LiteralPath $Fixture.ArchivePath -Algorithm SHA256).Hash.ToLowerInvariant()
    Set-Content -LiteralPath $Fixture.ChecksumPath -Value "$Digest  $($Fixture.Asset)" -Encoding ascii
}

function New-ReleaseFixture {
    param([Parameter(Mandatory)][string]$Root)
    $ReleaseRoot = Join-Path $Root 'release assets'
    $PayloadRoot = Join-Path $Root 'archive payload'
    New-Item -ItemType Directory -Path $ReleaseRoot, $PayloadRoot -Force | Out-Null
    $RuntimeBytes = [Text.Encoding]::UTF8.GetBytes('verified-windows-runtime')
    [IO.File]::WriteAllBytes((Join-Path $PayloadRoot 'harnessctl.exe'), $RuntimeBytes)
    [IO.File]::WriteAllText((Join-Path $PayloadRoot 'LICENSE'), "MIT fixture`n")
    $Asset = 'harnessctl-v0.2.0-x86_64-pc-windows-msvc.zip'
    $ArchivePath = Join-Path $ReleaseRoot $Asset
    [System.IO.Compression.ZipFile]::CreateFromDirectory($PayloadRoot, $ArchivePath)
    Set-RegularZipAttributes -ArchivePath $ArchivePath
    $Fixture = [pscustomobject]@{
        ReleaseRoot = $ReleaseRoot
        Asset = $Asset
        ArchivePath = $ArchivePath
        ChecksumPath = Join-Path $ReleaseRoot 'SHA256SUMS'
        RuntimeBytes = $RuntimeBytes
    }
    Set-ReleaseChecksum -Fixture $Fixture
    return $Fixture
}

function Start-ReleaseHttpFixture {
    param(
        [Parameter(Mandatory)][string]$ReleaseRoot,
        [Parameter(Mandatory)][string]$StateRoot
    )
    $PortPath = Join-Path $StateRoot ("http-port-" + [guid]::NewGuid())
    $Job = Start-Job -ArgumentList $ReleaseRoot, $PortPath -ScriptBlock {
        param($Root, $PortFile)
        $ErrorActionPreference = 'Stop'
        $Listener = [System.Net.Sockets.TcpListener]::new(
            [System.Net.IPAddress]::Loopback,
            0
        )
        $Listener.Start()
        try {
            $Port = ([System.Net.IPEndPoint]$Listener.LocalEndpoint).Port
            [IO.File]::WriteAllText($PortFile, [string]$Port)
            foreach ($RequestNumber in 1..2) {
                $Client = $Listener.AcceptTcpClient()
                try {
                    $Stream = $Client.GetStream()
                    $Reader = [IO.StreamReader]::new(
                        $Stream,
                        [Text.Encoding]::ASCII,
                        $false,
                        1024,
                        $true
                    )
                    try {
                        $RequestLine = $Reader.ReadLine()
                        do {
                            $HeaderLine = $Reader.ReadLine()
                        } while ($null -ne $HeaderLine -and $HeaderLine -ne '')
                    }
                    finally {
                        $Reader.Dispose()
                    }

                    $Parts = @($RequestLine -split ' ')
                    $Name = ''
                    if ($Parts.Count -ge 2) {
                        $RequestPath = $Parts[1].Split('?')[0].TrimStart([char]'/')
                        $Name = [Uri]::UnescapeDataString($RequestPath)
                    }
                    $SafeName = $Name -and $Name -notmatch '[/\\]' -and $Name -notin @('.', '..')
                    $Source = if ($SafeName) { Join-Path $Root $Name } else { '' }
                    if ($SafeName -and (Test-Path -LiteralPath $Source -PathType Leaf)) {
                        $Status = '200 OK'
                        $Body = [IO.File]::ReadAllBytes($Source)
                    }
                    else {
                        $Status = '404 Not Found'
                        $Body = [Text.Encoding]::UTF8.GetBytes('not found')
                    }
                    $Header = "HTTP/1.1 $Status`r`nContent-Length: $($Body.Length)`r`nConnection: close`r`n`r`n"
                    $HeaderBytes = [Text.Encoding]::ASCII.GetBytes($Header)
                    $Stream.Write($HeaderBytes, 0, $HeaderBytes.Length)
                    $Stream.Write($Body, 0, $Body.Length)
                    $Stream.Flush()
                }
                finally {
                    $Client.Dispose()
                }
            }
        }
        finally {
            $Listener.Stop()
        }
    }

    $Deadline = [DateTime]::UtcNow.AddSeconds(10)
    while (-not (Test-Path -LiteralPath $PortPath -PathType Leaf)) {
        if ($Job.State -in @('Completed', 'Failed', 'Stopped')) {
            $Details = Receive-Job -Job $Job -ErrorAction SilentlyContinue | Out-String
            Remove-Job -Job $Job -Force
            throw "Loopback HTTP fixture stopped before startup: $Details"
        }
        if ([DateTime]::UtcNow -ge $Deadline) {
            Stop-Job -Job $Job -ErrorAction SilentlyContinue
            Remove-Job -Job $Job -Force
            throw 'Loopback HTTP fixture startup timed out'
        }
        Start-Sleep -Milliseconds 25
    }

    return [pscustomobject]@{
        Job = $Job
        BaseUrl = 'http://127.0.0.1:' + (Get-Content -LiteralPath $PortPath -Raw)
    }
}

function Complete-ReleaseHttpFixture {
    param([Parameter(Mandatory)]$Server)
    $Completed = Wait-Job -Job $Server.Job -Timeout 10
    if ($null -eq $Completed) {
        Stop-Job -Job $Server.Job -ErrorAction SilentlyContinue
        Receive-Job -Job $Server.Job -ErrorAction SilentlyContinue | Out-Null
        Remove-Job -Job $Server.Job -Force
        throw 'Loopback HTTP fixture did not receive both release requests'
    }
    $State = $Server.Job.State
    $Details = Receive-Job -Job $Server.Job -ErrorAction SilentlyContinue | Out-String
    Remove-Job -Job $Server.Job -Force
    if ($State -ne 'Completed') {
        throw "Loopback HTTP fixture failed: $Details"
    }
}

function Get-InstalledRuntime {
    param([Parameter(Mandatory)][string]$CodexHome)
    return Join-Path $CodexHome 'skills/cached-subagent-harness/scripts/bin/harnessctl.exe'
}

$script:CargoInvocationCount = 0
$script:FakeBuildBytes = [Text.Encoding]::UTF8.GetBytes('locally-built-windows-runtime')
function cargo {
    $script:CargoInvocationCount += 1
    $Arguments = @($args)
    $ManifestIndex = [Array]::IndexOf($Arguments, '--manifest-path')
    if ($ManifestIndex -lt 0 -or $ManifestIndex + 1 -ge $Arguments.Count) {
        $global:LASTEXITCODE = 71
        return
    }
    $CrateRoot = Split-Path -Parent $Arguments[$ManifestIndex + 1]
    $OutputPath = Join-Path $CrateRoot 'target/release/harnessctl.exe'
    New-Item -ItemType Directory -Path (Split-Path -Parent $OutputPath) -Force | Out-Null
    [IO.File]::WriteAllBytes($OutputPath, $script:FakeBuildBytes)
    $global:LASTEXITCODE = 0
}

function Test-NoneSource {
    param([string]$Root)
    $CodexHome = Join-Path $Root 'none home'
    Invoke-HarnessInstall -RepoRoot $RepoRoot -CodexHome $CodexHome -BinarySource None
    $SkillRoot = Join-Path $CodexHome 'skills/cached-subagent-harness'
    Assert-True (Test-Path -LiteralPath (Join-Path $SkillRoot 'SKILL.md') -PathType Leaf) 'Skill was not installed'
    Assert-False (Test-Path -LiteralPath (Join-Path $SkillRoot 'scripts/bin/harnessctl.exe')) 'None source installed a Windows runtime'
    Assert-False (Test-Path -LiteralPath (Join-Path $SkillRoot 'scripts/bin/harnessctl')) 'None source copied a stale Unix runtime'
}

function Test-DownloadSuccess {
    param([string]$Root)
    $Fixture = New-ReleaseFixture -Root (Join-Path $Root 'download fixture')
    $CodexHome = Join-Path $Root 'download home'
    $Before = $script:CargoInvocationCount
    Invoke-HarnessInstall -RepoRoot $RepoRoot -CodexHome $CodexHome -BinarySource Download -ReleaseBaseUrl $Fixture.ReleaseRoot
    Assert-Equal $script:CargoInvocationCount $Before 'Download invoked Cargo'
    $Actual = [IO.File]::ReadAllBytes((Get-InstalledRuntime -CodexHome $CodexHome))
    Assert-Equal ([Convert]::ToBase64String($Actual)) ([Convert]::ToBase64String($Fixture.RuntimeBytes)) 'Downloaded runtime bytes differ'
}

function Test-HttpDownloadSuccess {
    param([string]$Root)
    $Fixture = New-ReleaseFixture -Root (Join-Path $Root 'HTTP fixture')
    $Server = Start-ReleaseHttpFixture -ReleaseRoot $Fixture.ReleaseRoot -StateRoot $Root
    $CodexHome = Join-Path $Root 'HTTP download home'
    $Before = $script:CargoInvocationCount
    try {
        Invoke-HarnessInstall `
            -RepoRoot $RepoRoot `
            -CodexHome $CodexHome `
            -BinarySource Download `
            -ReleaseBaseUrl $Server.BaseUrl
    }
    finally {
        Complete-ReleaseHttpFixture -Server $Server
    }
    Assert-Equal $script:CargoInvocationCount $Before 'HTTP Download invoked Cargo'
    $Actual = [IO.File]::ReadAllBytes((Get-InstalledRuntime -CodexHome $CodexHome))
    Assert-Equal ([Convert]::ToBase64String($Actual)) ([Convert]::ToBase64String($Fixture.RuntimeBytes)) 'HTTP runtime bytes differ'
}

function Test-ChecksumMismatch {
    param([string]$Root)
    $Fixture = New-ReleaseFixture -Root (Join-Path $Root 'mismatch fixture')
    Set-Content -LiteralPath $Fixture.ChecksumPath -Value "$('0' * 64)  $($Fixture.Asset)" -Encoding ascii
    $CodexHome = Join-Path $Root 'mismatch home'
    Assert-Throws {
        Invoke-HarnessInstall -RepoRoot $RepoRoot -CodexHome $CodexHome -BinarySource Download -ReleaseBaseUrl $Fixture.ReleaseRoot
    } 'checksum mismatch' 'Checksum mismatch'
    Assert-False (Test-Path -LiteralPath (Get-InstalledRuntime -CodexHome $CodexHome)) 'Checksum mismatch installed a runtime'
}

function Test-MissingAndDuplicateChecksum {
    param([string]$Root)
    $Fixture = New-ReleaseFixture -Root (Join-Path $Root 'checksum fixture')
    Set-Content -LiteralPath $Fixture.ChecksumPath -Value '' -Encoding ascii
    Assert-Throws {
        Invoke-HarnessInstall -RepoRoot $RepoRoot -CodexHome (Join-Path $Root 'missing checksum home') -BinarySource Download -ReleaseBaseUrl $Fixture.ReleaseRoot
    } 'checksum entry' 'Missing checksum'

    $Digest = (Get-FileHash -LiteralPath $Fixture.ArchivePath -Algorithm SHA256).Hash.ToLowerInvariant()
    Set-Content -LiteralPath $Fixture.ChecksumPath -Value @(
        "$Digest  $($Fixture.Asset)",
        "$Digest  $($Fixture.Asset)"
    ) -Encoding ascii
    Assert-Throws {
        Invoke-HarnessInstall -RepoRoot $RepoRoot -CodexHome (Join-Path $Root 'duplicate checksum home') -BinarySource Download -ReleaseBaseUrl $Fixture.ReleaseRoot
    } 'checksum entry' 'Duplicate checksum'
}

function Test-UnsafeZipMember {
    param([string]$Root)
    $Fixture = New-ReleaseFixture -Root (Join-Path $Root 'unsafe fixture')
    $Zip = [System.IO.Compression.ZipFile]::Open(
        $Fixture.ArchivePath,
        [System.IO.Compression.ZipArchiveMode]::Update
    )
    try {
        $Entry = $Zip.GetEntry('harnessctl.exe')
        $Entry.ExternalAttributes = ConvertTo-SignedAttributes -Value ([uint32]2717843456)
    }
    finally {
        $Zip.Dispose()
    }
    Set-ReleaseChecksum -Fixture $Fixture
    $CodexHome = Join-Path $Root 'unsafe home'
    Assert-Throws {
        Invoke-HarnessInstall -RepoRoot $RepoRoot -CodexHome $CodexHome -BinarySource Download -ReleaseBaseUrl $Fixture.ReleaseRoot
    } 'unsafe member' 'Symlink ZIP member'
    Assert-False (Test-Path -LiteralPath (Get-InstalledRuntime -CodexHome $CodexHome)) 'Unsafe ZIP installed a runtime'
}

function Test-ForcedDownloadNeverBuilds {
    param([string]$Root)
    $Before = $script:CargoInvocationCount
    Assert-Throws {
        Invoke-HarnessInstall -RepoRoot $RepoRoot -CodexHome (Join-Path $Root 'forced download home') -BinarySource Download -ReleaseBaseUrl (Join-Path $Root 'missing release')
    } 'download failed' 'Forced Download failure'
    Assert-Equal $script:CargoInvocationCount $Before 'Forced Download invoked Cargo'
}

function Test-AutoFallsBackToBuild {
    param([string]$Root)
    $CodexHome = Join-Path $Root 'auto home'
    $Before = $script:CargoInvocationCount
    Invoke-HarnessInstall -RepoRoot $RepoRoot -CodexHome $CodexHome -BinarySource Auto -ReleaseBaseUrl (Join-Path $Root 'missing release')
    Assert-Equal $script:CargoInvocationCount ($Before + 1) 'Auto did not invoke Cargo once'
    $Actual = [IO.File]::ReadAllBytes((Get-InstalledRuntime -CodexHome $CodexHome))
    Assert-Equal ([Convert]::ToBase64String($Actual)) ([Convert]::ToBase64String($script:FakeBuildBytes)) 'Auto fallback bytes differ'
}

function Test-BuildNeverDownloads {
    param([string]$Root)
    $CodexHome = Join-Path $Root 'build home'
    $Before = $script:CargoInvocationCount
    Invoke-HarnessInstall -RepoRoot $RepoRoot -CodexHome $CodexHome -BinarySource Build -ReleaseBaseUrl 'not a valid URL'
    Assert-Equal $script:CargoInvocationCount ($Before + 1) 'Build did not invoke Cargo once'
    Assert-True (Test-Path -LiteralPath (Get-InstalledRuntime -CodexHome $CodexHome) -PathType Leaf) 'Build did not install a runtime'
}

function Test-PathWithSpaces {
    param([string]$Root)
    $Fixture = New-ReleaseFixture -Root (Join-Path $Root 'fixture with spaces')
    $CodexHome = Join-Path $Root 'Codex Home With Spaces'
    Invoke-HarnessInstall -RepoRoot $RepoRoot -CodexHome $CodexHome -BinarySource Download -ReleaseBaseUrl $Fixture.ReleaseRoot
    Assert-True (Test-Path -LiteralPath (Get-InstalledRuntime -CodexHome $CodexHome) -PathType Leaf) 'Path with spaces failed'
}

function Test-ReplacementFailure {
    param([string]$Root)
    $Fixture = New-ReleaseFixture -Root (Join-Path $Root 'replacement fixture')
    $SkillRoot = Join-Path $Root 'replacement skill'
    $BinDir = Join-Path $SkillRoot 'scripts/bin'
    $Destination = Join-Path $BinDir 'harnessctl.exe'
    New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
    $PreservedBytes = [Text.Encoding]::UTF8.GetBytes('preserved-runtime')
    [IO.File]::WriteAllBytes($Destination, $PreservedBytes)

    $Output = @()
    $Caught = $null
    $Lock = [IO.File]::Open(
        $Destination,
        [IO.FileMode]::Open,
        [IO.FileAccess]::ReadWrite,
        [IO.FileShare]::None
    )
    try {
        try {
            $Output = @(Install-VerifiedRelease `
                -Version '0.2.0' `
                -BaseUrl $Fixture.ReleaseRoot `
                -SkillRoot $SkillRoot)
        }
        catch {
            $Caught = $_.Exception.Message
        }
    }
    finally {
        $Lock.Dispose()
    }

    Assert-True ($null -ne $Caught) 'Replacement failure did not throw'
    Assert-True ($Caught -like '*runtime replacement failed*') "Unexpected replacement error: $Caught"
    Assert-False (($Output -join "`n") -like '*Installed verified*') 'Replacement failure reported success'
    $Actual = [IO.File]::ReadAllBytes($Destination)
    Assert-Equal ([Convert]::ToBase64String($Actual)) ([Convert]::ToBase64String($PreservedBytes)) 'Replacement failure changed destination bytes'
    $Staged = @(Get-ChildItem -LiteralPath $BinDir -Filter '.harnessctl.*.exe' -Force)
    Assert-Equal $Staged.Count 0 'Replacement failure left a staged runtime'
}

Assert-Equal (Get-ReleaseTarget) 'x86_64-pc-windows-msvc' 'release target mismatch'
Assert-Equal (Get-PackageVersion -RepoRoot $RepoRoot) '0.2.0' 'package version mismatch'

$TempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("harness install tests " + [guid]::NewGuid())
New-Item -ItemType Directory -Path $TempRoot -Force | Out-Null
try {
    Test-NoneSource -Root $TempRoot
    Test-DownloadSuccess -Root $TempRoot
    Test-HttpDownloadSuccess -Root $TempRoot
    Test-ChecksumMismatch -Root $TempRoot
    Test-MissingAndDuplicateChecksum -Root $TempRoot
    Test-UnsafeZipMember -Root $TempRoot
    Test-ForcedDownloadNeverBuilds -Root $TempRoot
    Test-AutoFallsBackToBuild -Root $TempRoot
    Test-BuildNeverDownloads -Root $TempRoot
    Test-PathWithSpaces -Root $TempRoot
    Test-ReplacementFailure -Root $TempRoot
}
finally {
    Remove-Item -LiteralPath $TempRoot -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Output 'PowerShell installer behavior tests passed'
