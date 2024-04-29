declare function require(s: string): any

let _mintyml: Promise<typeof import('../pkg-node/minty_wasm.d.ts')>

// We need to know if we're running in a browser (bundled with e.g. WebPack)
// or in node.js. If we're in a browser, we import from 'pkg-web' which imports the
// .wasm file for webpack to bundle.
// In node, we assume there's no bundler and import from 'pkg-node' which the loads the file via 'fs'
const isBrowser = eval(`
    this === this.window
 || this === this.self
 || typeof require !== 'function'
`)

if (isBrowser) {
    _mintyml = require('../pkg-web/minty_wasm.js')
} else {
    // Use eval so WebPack doesn't think it's a dependency
    _mintyml = eval('require')('../pkg-node/minty_wasm.js')
}

/** Represents a failed MinTyML operation. */
export interface MintymlError {
    /** Message describing the error. */
    message: string
    /** The specific syntax errors that caused the failure. */
    syntaxErrors?: MintymlSyntaxError[],
}

/** Base type for any MinTyML syntax error. */
export interface MintymlBaseSyntaxError {
    /** Message describing the syntax error. */
    message: string
    /** The source text causing the syntax error. */
    actual: string
    /** The location within the source string where the error begins. */
    start: number
    /** The location within the source string where the error ends. */
    end: number
}

/** Represents a scenario where the source could not be parsed into a tree. */
export interface MintymlParsingError extends MintymlBaseSyntaxError {
    /** A list of strings describing what was expected. */
    expected: string[]
}

/** Describes a syntax error that caused a failed MinTyML operation. */
export type MintymlSyntaxError = MintymlBaseSyntaxError | MintymlParsingError

export interface MintymlConverterOptions {
    /**
     * If true, produce XHTML5 rather than HTML.
     * @default false
     */
    xml: boolean
    /**
     * If specified, the converted HTML will be pretty-printed with the given number
     * of spaces in each indentation level.
     * If null, the converted HTML will not be pretty-printed.
     *
     * @default null
     */
    indent: number | null
    /**
     * If true, the output will be a complete HTML page with an `<html>` tag containing
     * a `<body>` and `<head>` tag.
     *
     * Otherwise, keep the input's element structure intact.
     *
     * @default false
     */
    completePage: boolean

    specialTags: Partial<{
        emphasis: string | null
        strong: string | null
        underline: string | null
        strike: string | null
        quote: string | null
        code: string | null
        codeBlockContainer: string | null
    } | null> | null

    /** If provided, parsing metadata will be added to the output. */
    metadata: boolean | Partial<{
        /**
         * Generate elements for nodes that don't correspond directly to HTML elements,
         * like comments and text segments.
         */
        elements: boolean
    }> | null
}

/** Converts MinTyML source to HTML. */
export class MintymlConverter implements MintymlConverterOptions {
    xml; indent; completePage; specialTags; metadata

    constructor(options: Partial<MintymlConverterOptions> = {}) {
        this.xml = options.xml ?? false
        this.indent = options.indent ?? null
        this.completePage = options.completePage ?? false
        this.specialTags = options.specialTags ?? null
        this.metadata = options.metadata ?? null
    }

    /** Converts the given MinTyML string to HTML. */
    async convert(src: string): Promise<string> {
        const mintyml = await _mintyml
        try {
            return mintyml.convert(src, this.xml, this.indent ?? -1, this.completePage, this.specialTags, this.metadata)
        } catch (e) {
            const err = e as MintymlError

            if (err.syntaxErrors) {
                err.message = err.syntaxErrors.map(e => {
                    if ('expected' in e) {
                        return `Unexpected '${e.actual}'; expected ${e.expected.join(' | ')}`
                    } else {
                        return `Unexpected '${e.actual}'`
                    }
                }).join('\n')
            }
            throw e
        }
    }
}
