# Metadata (experimental)


<b>Note: These definitions are unstable and may change in the future</b>

The `--metadata` flag for `mintyml-cli` adds
additional attributes to the converted document.
Combined with `--xml`, an external tool can parse the document with metadata
to perform additional processing on the document.


## Data Types

bool
: A "true" or "false" value.

position

: The number of bytes (**not characters**) from the start of the source string
  to a particular location in the source string.

## Attributes

All root elements in the output are given
`xmlns:mty="tag:youngspe.github.io,2024:mintyml/metadata"`
to declare the `mty:` namespace for metadata tags and attributes.


### All nodes

`mty:start`
: type: position
: Starting location of the node

`mty:end`
: type: position
: Ending location of the node

### Elements

`mty:content-start`
: type: position
: Starting location of the element's content

`mty:content-end`
: type: position
: Ending location of the element's content

The start/end attributes may be omitted if the internally-represented ranges
defined for the node do not exist in the source document.

### Text nodes

These attributes are only relevant if `--metadata-elements` is enabled:

`mty:verbatim`
: type: bool
: If `true`, any escape sequences in the source of this text node are left intact.

`mty:raw`
: type: bool
: If `true`, the content of this text node are not escaped in the final HTML document,
  even if they are not valid in HTML text.
  If `--xml` is enabled, the content is escaped to form valid XML text even if this
  attribute is `true`.

`mty:multiline`
: type: bool
: If `true`, this text node was declared using multiline string syntax.

## Elements

If the `--metadata-elements` flag is enabled, XML elements will be created for non-element nodes
in order to apply metadata attributes.

`mty:text`
: This element is wrapped around each text node.

`mty:comment`
: This element replaces a comment node.
  It contains the original contents of the comment in the form of a text node.

`mty:element`
: This element is created for source elements that do not have an explicit or inferred tag name.
  Without `--metadata-elements`, these elements would be excluded from the output.

