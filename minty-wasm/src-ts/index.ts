declare function require(s: string): any

let _mintyml: Promise<typeof import('../pkg-web')>

// We need to know if we're running in a browser (bundled with e.g. WebPack)
// or in node.js. If we're in a browser, we import from 'pkg-web' which imports the
// .wasm file for webpack to bundle.
// In node, we assume there's no bundler and import from 'pkg-node' which the loads the file via 'fs'
const isBrowser = new Function(`
    return this === this.window
 || this === this.self
 || typeof require !== 'function'
`)()

if (isBrowser) {
    _mintyml = require('../pkg-web/minty_wasm.js')
} else {
    // Use eval so WebPack doesn't think it's a dependency
    _mintyml = eval('require')('../pkg-node/minty_wasm.js')
}

export interface MintymlError {
    message: string
    syntax_errors?: MintymlSyntaxError[],
}
export interface MintymlBaseSyntaxError {
    message: string
    actual: String
    start: number
    end: number
}
export interface MintymlParsingError extends MintymlBaseSyntaxError {
    expected: string[]
}
export type MintymlSyntaxError = MintymlBaseSyntaxError | MintymlParsingError

export class MintymlConverter {
    xml
    indent: number | null
    completePage: boolean

    constructor(options: { xml?: boolean, indent?: number | null, completePage?: boolean } = {}) {
        this.xml = options.xml ?? false
        this.indent = options.indent ?? null
        this.completePage = options.completePage ?? false
    }

    async convert(src: string): Promise<string> {
        const mintyml = await _mintyml
        try {
            return mintyml.convert(src, this.xml, this.indent ?? -1, this.completePage)
        } catch (e) {
            const err = e as MintymlError

            if (err.syntax_errors) {
                err.message = err.syntax_errors.map(e => {
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
