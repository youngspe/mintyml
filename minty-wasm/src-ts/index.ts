import * as _mintyml from '../pkg'

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

    constructor(options: { xml?: boolean, indent?: number | null }) {
        this.xml = options.xml ?? false
        this.indent = options.indent ?? null
    }

    convert(src: string): string {
        try {
            return _mintyml.convert(src, this.xml, this.indent ?? -1)
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
