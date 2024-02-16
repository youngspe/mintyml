import * as _mintyml from '../pkg'

export class MintymlConverter {
    xml
    indent: number | null

    constructor(options: { xml?: boolean, indent?: number | null }) {
        this.xml = options.xml ?? false
        this.indent = options.indent ?? null
    }

    convert(src: string): string {
        return _mintyml.convert(src, this.xml, this.indent ?? -1)
    }
}
