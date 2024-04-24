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

    $template = Get-Content -Raw "$WSRoot/example-doc/template.mty"

    $template -replace '<! content here !>',$content `
    | Out-File -NoNewline "$WSRoot/web-demo/public/examples/intro.mty"
}
