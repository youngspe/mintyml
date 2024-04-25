$WSRoot = $PSScriptRoot

function Build-ReadmeDotMd {
    cargo run -p mintyml-cli -- `
        convert --special-tag underline=ins `
        --pretty --fragment `
        "$WSRoot/README.mty" `
        --out "$WSRoot/README.md"
}

function Build-ExampleIntro {
    $content = (Get-Content "$WSRoot/README.mty" `
        | ForEach-Object { $_ ? "  " + $_ : "" }
    ) -join "`n"

    $template = Get-Content -Raw "$WSRoot/doc-templates/readme.mty"

    $template -replace '<! content here !>', $content `
    | Out-File -NoNewline "$WSRoot/web-demo/public/examples/intro.mty"
}

function Build-CliReadme {
    function getHelp {
        (cargo run -qp mintyml-cli -- help @args
        | ForEach-Object { $_ ? "    " + $_ : "" }) -join "`n"
    }

    $template = Get-Content -Raw "$WSRoot/doc-templates/cli-readme.md"

    $template -replace '\{\{\s*help\s*(.*?)\s*}}', { Invoke-Expression "getHelp $($_.Groups[1])" } `
    | Out-File -NoNewline "$WSRoot/minty-cli/README.md"
}
