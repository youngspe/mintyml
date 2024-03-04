declare function require(s: string): any

let _mintyml: Promise<typeof import('../pkg-web')>

if (globalThis.window || !eval(`typeof require === 'function'`)) {
    _mintyml = require('../pkg-web')
} else {
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
