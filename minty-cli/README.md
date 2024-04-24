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
              Whether to recursively search subdirectories when searching a directory for source files. If specified, the search will be limited to `DEPTH` levels of nested subdirectories

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

          --fragment
              Convert a MinTyML fragment without wrapping it in `<html>` tags

          --special-tag <SPECIAL_TAG>
              Override the element types used when converting special tags.
              
              This argument may be used multiple times to allow multiple overrides. Additionally, multiple overrides can be specified per argument, separated by commas.
              
              Example: --special_tag underline=ins,strike=del

              Possible values:
              - strong=...:               <# strong #> (default: 'strong')
              - emphasis=...:             </ emphasis /> (default: 'em')
              - underline=...:            <_ underline _> (default: 'u')
              - strike=...:               <~ strike ~> (default: 's')
              - quote=...:                <" quote "> (default: 'q')
              - code=...:                 <` code `> (default: 'code')
              - code-block-container=...: ``` code block ``` (default: 'pre')