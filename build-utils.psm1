using namespace System
using namespace System.Collections.Generic
$ErrorActionPreference = 'Stop'
$WSRoot = $PSScriptRoot

function Test-ExitCode {
    $command = $args[0]
    $argList = $args[1..$args.Length]
    if ($command) {
        $global:LASTEXITCODE = 0
        try {
            & $command @argList
            if ($LASTEXITCODE -ne 0) {
                Write-Error "$command failed with exit code $LASTEXITCODE"
            }
        }
        catch {
            Write-Error $_
        }
    }
    else {
        if ($LASTEXITCODE -ne 0) {
            Write-Error "command failed with exit code $LASTEXITCODE"
        }
    }
}

function Use-Location(
    [string] $Path = $null,
    [scriptblock] $Body
) {
    Push-Location -Path:$Path -StackName "Use-Location"
    try {
        $input | & $Body
    }
    finally {
        Pop-Location -StackName "Use-Location"
    }
}

function Get-Version {
    Get-Content "$WSRoot/Cargo.toml" -Raw `
    | Select-String '(?m)^\s*version\s*=\s*"(?<version>.*?)"\s*$' `
    | ForEach-Object { $_.Matches[0].Groups['version'].Value }
}

function Get-VersionIncrement(
    [ValidateNotNullOrWhiteSpace()][string] $Version
) {
    $parts = $Version -split '\.'
    $parts[-1] = ([int]$parts[-1]) + 1
    $parts -join '.'
}

function Set-Version([string] $Version) {
    $src = (Get-Content "$WSRoot/Cargo.toml" -Raw)
    $out = $src -replace '(?m)(?<=^\s*version\s*=\s*").*?(?="\s*$)', $Version

    $out > "$WSRoot/Cargo.toml"
}

$packageNames = 'mintyml', 'minty-cli', 'minty-wasm'

class WSState {
    [string] $OldTag
    [string] $NewVersion
    [List[string]] $Packages = @()
    [string[]] $Targets = @()
}

function Get-NewVersion([WSState] $State) {
    $oldVersion = Get-Version
    $State.OldTag = git show-ref -s --tags "v$oldVersion"

    if ($State.OldTag) {
        $State.NewVersion = Get-VersionIncrement
        Write-Host (
            "::notice title=Version::Tag 'v$oldVersion' already exists; " +
            "Incrementing to v$($State.NewVersion)"
        )
    }
    else {
        Write-Host "::notice title=Version::Using 'v$oldVersion' from manifest file"
        $State.NewVersion = $oldVersion
    }
    $State.NewVersion
}

function Get-OutOfDatePackages([WSState] $State) {
    & {
        if (-not $State.OldTag) {
            return $packageNames
        }

        $changes = git diff --name-only $State.OldTag HEAD

        if ($changes -like 'mintyml/*') {
            # Update all packages if the core package changed
            return $packageNames
        }

        if ($changes -like 'minty-cli/*') {
            'minty-cli'
        }

        if ($changes -like 'minty-wasm/*') {
            'minty-wasm'
        }

    } | ForEach-Object { $State.Packages.Add($_) }
}

function Build-NodeManifest([WSState] $State = $null) {
    $version = $State.NewVersion ?? (Get-Version)
    $src = Get-Content "$WSRoot/minty-wasm/package.template.json" -Raw
    $out = $src -creplace '\{\{VERSION\}\}', $version
    $out > "$WSRoot/minty-wasm/package.json"
}

function Update-Version([WSState] $State) {
    $version = Get-NewVersion $State
    $packages = Get-OutOfDatePackages $State

    if (-not $packages) {
        return
    }

    if ($State.OldTag) {
        Set-Version $version
    }

    cargo update --workspace --offline
}

function Publish-Packages([switch] $Publish) {
    $State = [WSState]::new()
    Update-Version $State

    if (-not $State.Packages) {
        Write-Host "::notice title=No packages to publish::"
        return
    }

    Write-Host "::notice title=Packages will be published::$($State.Packages -join ', ')"

    $dryRun = $Publish ? @() : @('--dry-run')
    $successCount = 0

    if ("mintyml" -in $State.Packages) {
        Write-Host "Publishing mintyml..."
        try {
            Test-ExitCode cargo publish -q -p mintyml @dryRun --allow-dirty
            $successCount += 1
        }
        catch {
            Write-Host "::notice::mintyml failed to publish"
        }
    }
    else {
        Write-Host "Skipping mintyml"
    }

    if ("minty-cli" -in $State.Packages) {
        Write-Host "Publishing minty-cli..."
        try {
            Test-ExitCode cargo publish -q -p mintyml-cli @dryRun --allow-dirty
            $successCount += 1
        }
        catch {
            Write-Host "::notice::minty-cli failed to publish"
        }
    }
    else {
        Write-Host "Skipping minty-cli"
    }

    if ("minty-wasm" -in $State.Packages) {
        Write-Host "Publishing minty-wasm..."
        just -f "$WSRoot/justfile" build-node
        Push-Location "$WSRoot/minty-wasm"
        try {
            Build-NodeManifest $State
            Test-ExitCode npm publish @dryRun
            $successCount += 1
        }
        catch {
            Write-Host "::notice::minty-wasm failed to publish"
        }
        finally {
            Pop-Location
        }
    }
    else {
        Write-Host "Skipping minty-wasm"
    }

    if ($successCount -eq 0) {
        return
    }

    Sync-Changes $State
}

function Sync-Changes([WSState] $State, [switch] $Publish) {
    $tagName = "v$($State.NewVersion)"
    $dryRun = $Publish ? @() : @('--dry-run')

    if ($State.OldTag) {
        Write-Host "Committing version bump..."
        Test-ExitCode git add .
        Test-ExitCode git commit -m "Increment to $tagName"
        Test-ExitCode git pull --rebase --strategy-option=theirs
        Write-Host "Pushing version bump...."
        Test-ExitCode git push @dryRun
    }
    Write-Host "Making tag..."
    Test-ExitCode git tag --force $tagName -m ""
    Write-Host "Pushing tag..."
    Test-ExitCode git push @dryRun origin "refs/tags/$tagName"
}

function Build-Release {
    [CmdletBinding()]
    param ($Version = $null)
    $Version ??= Get-Version

    $targets = Get-Content "$WSRoot/release-targets.txt" `
    | ForEach-Object Trim `
    | Where-Object { $_ -and $_ -notlike '#*' }

    [List[Exception]] $Errors = @()

    $outDir = "$WSRoot/target-release"

    $targets | ForEach-Object {
        $target = $_
        try {
            Write-Host "Building $target ..."
            Test-ExitCode rustup target add $target
            Test-ExitCode cross build -q --release -p mintyml-cli --target $target

            $file = Get-ChildItem `
                "$WSRoot/target/$target/release/mintyml-cli*" `
                -File -Include 'mintyml-cli', 'mintyml-cli.exe'

            $outName = "mintyml-cli-$target-v$Version"

            New-Item -ItemType Directory -Force "$outDir/$outName" > $null
            Copy-Item -Path $file -Destination "$outDir/$outName/"

            Test-ExitCode tar -czf "$outDir/$outName.tgz" `
                -C $outDir $outName
        }
        catch {
            Write-Host "::error::Target '$target' Failed: $_"
            $Errors.Add($_.Exception)
        }
    } | Write-Host

    if ($Errors) {
        throw [AggregateException]::new($Errors)
    }
}

function Publish-Release {
    [CmdletBinding()]
    param($Version = $null, [switch] $Publish)
    if (-not $Version) {
        $Version = Get-Version
    }
    $tagName = "v$Version"
    gh release view $tagName *> $null
    if ($?) {
        Write-Host "::notice title=Skipping release::Release $tagName already exists"
        return
    }

    Build-Release $Version

    if ($Publish) {
        $assets = Get-ChildItem "$WSRoot/target-release/*.tgz" | ForEach-Object FullName
        Write-Host "Creating release..."
        Test-ExitCode gh release create --latest $tagName @assets
    }
}

Export-ModuleMember Publish-Packages, Build-NodeManifest, Build-Release, Publish-Release
