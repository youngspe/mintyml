## Installation

Download the binary from [Github](https://github.com/youngspe/mintyml/releases/latest)
or install with [`cargo install mintyml-cli`](https://crates.io/crates/mintyml-cli)

## Help Text

    Processes MinTyML, a minimalist alternative syntax for HTML.

    For more information, see https://youngspe.github.io/mintyml and https://github.com/youngspe/mintyml

    Usage: mintyml-cli <COMMAND>

    Commands:
      convert  Convert MinTyML to HTML
      help     Print this message or the help of the given subcommand(s)

    Options:
      -h, --help
              Print help (see a summary with '-h')

### `convert` Command

    Convert MinTyML to HTML

    Usage: mintyml-cli convert [OPTIONS] <--stdin|--dir <SRC_DIR>|SRC_FILES>

    Options:
      -h, --help
              Print help (see a summary with '-h')

    Input Source:
          --stdin
              Read MinTyML source from stdin

      -d, --dir <SRC_DIR>
              Search for MinTyML files in the given directory

      -r, --recurse [<DEPTH>]
              Whether to recursively search subdirectories when searching a directory for source files.
              If specified, the search will be limited to `DEPTH` levels of nested subdirectories

      [SRC_FILES]...
              Convert the specified MinTyML file(s)

    Output Destination:
      -o, --out <OUT>
              Write the converted HTML to the given filename or directory

          --stdout
              Write the converted HTML to stdout

    Output Options:
      -x, --xml
              Produce XHTML5 instead of HTML

      -p, --pretty
              Produce HTML with line breaks and indentation for readability

          --indent <INDENT>
              Number of spaces for each indentation level when `--pretty` is enabled
              
              [default: 2]

          --complete-page[=<ENABLE>]
              Make a complete HTML page by wrapping the contents in `<html>` tags.
              
              * If the source document already has an `html` element at the top level, no changes will
              be made.
              
              * If the source document has a `body` element at the top level, no changes will be made
              beyond wrapping the document in `<html>` tags.
              
              * Otherwise, a `head` element will be created containing all top-level elements that
              belong in `head` (e.g. `title`, `meta`, `style`), and a `body` element will be created
              containing all other top-level elements.
              
              [default: true]
              
              [possible values: true, false]

          --fragment
              Convert a MinTyML fragment without wrapping it in `<html>` tags. Equivalent to
              `--complete-page=false`

          --special-tag <SPECIAL_TAG>
              Override the element types used when converting special tags.
              
              This argument may be used multiple times to allow multiple overrides. Additionally,
              multiple overrides can be specified per argument, separated by commas.
              
              Example: --special_tag underline=ins,strike=del

              Possible values:
              - strong=...:               <# strong #> (default: 'strong')
              - emphasis=...:             </ emphasis /> (default: 'em')
              - underline=...:            <_ underline _> (default: 'u')
              - strike=...:               <~ strike ~> (default: 's')
              - quote=...:                <" quote "> (default: 'q')
              - code=...:                 <` code `> (default: 'code')
              - code-block-container=...: ``` code block ``` (default: 'pre')

          --metadata[=<ENABLE>]
              If enabled, parsing metadata will be added to the output
              
              [possible values: true, false]

          --metadata-elements[=<ENABLE>]
              Generate elements for nodes that don't correspond directly to HTML elements, like comments
              and text segments. Implies `--metadata`
              
              [possible values: true, false]
