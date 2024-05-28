using namespace System
using namespace System.Collections.Generic
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true
$WSRoot = $PSScriptRoot

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

function Get-Tag([string] $Version = (Get-Version)) {
    "v$Version"
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

    $out | Out-File -NoNewline "$WSRoot/Cargo.toml"
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

    $State.OldTag = git fetch --quiet --tags origin "v$oldVersion" `
        && git show-ref -s --tags "v$oldVersion"

    if ($State.OldTag) {
        $State.NewVersion = Get-VersionIncrement $oldVersion
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

    } | ForEach-Object { $State.Packages.Add($_); $_ }
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
        Write-Host "Updating version in manifest..."
        Set-Version $version
    }
    else {
        Write-Host "Manifest remains unchanged"
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
            cargo publish -q -p mintyml @dryRun --allow-dirty
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
            cargo publish -q -p mintyml-cli @dryRun --allow-dirty
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
            npm publish @dryRun
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

    Sync-Changes -TagName "v$($State.NewVersion)" -Bump ([bool]$State.OldTag)  -Publish:$Publish
}

function Sync-Changes([string] $TagName, [bool] $Bump, [switch] $Publish) {
    $dryRun = $Publish ? @() : @('--dry-run')

    if ($Bump) {
        Write-Host "Committing version bump..."
        git add .
        git commit -m "Increment to $TagName"
        git pull --rebase --strategy-option=theirs
        Write-Host "Pushing version bump...."
        git push @dryRun
    }
    Write-Host "Making tag..."
    git tag --force $TagName -m ""
    Write-Host "Pushing tag..."
    git push @dryRun origin "refs/tags/$TagName"
}

function Get-Targets([string] $File) {
    Get-Content $File
    | ForEach-Object Trim `
    | Where-Object { $_ -and $_ -notlike '#*' }
}

function Build-Release {
    [CmdletBinding()]
    param (
        [string] $Tag = (Get-Tag),
        [string] $TargetFile,
        [List[string]] $Targets = (Get-Targets $TargetFile)
    )

    [List[Exception]] $Errors = @()

    $outDir = "$WSRoot/target-release"

    $Targets | ForEach-Object {
        $target = $_
        try {
            Write-Host "Building $target ..."
            rustup target add $target
            cross build -q --release -p mintyml-cli --target $target

            $file = Get-ChildItem `
                "$WSRoot/target/$target/release/mintyml-cli*" `
                -File -Include 'mintyml-cli', 'mintyml-cli.exe'

            $outName = "mintyml-cli-$target-$Tag"

            New-Item -ItemType Directory -Force "$outDir/$outName" > $null
            Copy-Item -Path $file -Destination "$outDir/$outName/"

            tar -czf "$outDir/$outName.tgz" -C $outDir $outName
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

function New-Release {
    [CmdletBinding()]
    param(
        [string] $Tag = (Get-Tag)
    )

    Write-Host "Creating release..."
    gh release create $Tag --draft=true --generate-notes
}

function Update-Release {
    [CmdletBinding()]
    param(
        [string] $Tag = (Get-Tag),
        [string] $TargetFile,
        [List[string]] $Targets = (Get-Targets $TargetFile),
        [switch] $Publish
    )

    $existingAssets = gh release view $Tag --json 'assets' --jq '[.assets[].name]'
    $Targets = $Targets | Where-Object { -not $existingAssets.Contains($_) }

    Build-Release -Tag $Tag -Targets $Targets

    if ($Publish) {
        $assets = Get-ChildItem "$WSRoot/target-release/*.tgz" | ForEach-Object FullName
        Write-Host "Uploading assets..."
        gh release upload $Tag @assets
    }
}

function Publish-Release {
    [CmdletBinding()]
    param(
        [string] $Tag = (Get-Tag)
    )

    Write-Host "Publishing release..."
    gh release edit $Tag --draft=false --latest
}


function Start-ReleaseWorkflow {
    [CmdletBinding()] param()

    Write-Host "Scheduling release..."
    gh workflow run release-cli.yml -f "tag=$(Get-Tag)"
}

Export-ModuleMember `
    Publish-Packages, Build-NodeManifest, Sync-Changes, Start-ReleaseWorkflow, `
    Build-Release, New-Release, Update-Release, Publish-Release

