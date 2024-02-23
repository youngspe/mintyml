import { ConvertRequestMessage, ConvertResponseMessage } from './message'
import { MintymlConverter, MintymlError } from 'mintyml'

let converter: MintymlConverter
let mintyml: Promise<typeof import('mintyml')>

self.onmessage = async function (e: MessageEvent<ConvertRequestMessage>) {
    mintyml ??= import('mintyml')
    converter ??= new (await mintyml).MintymlConverter({ indent: 2 })

    try {
        let output = converter.convert(e.data.input)
        self.postMessage({ output } satisfies ConvertResponseMessage)
    } catch (error) {
        self.postMessage({ error: error as MintymlError } satisfies ConvertResponseMessage)
    }
}
