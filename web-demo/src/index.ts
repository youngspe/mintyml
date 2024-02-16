import { ConvertResponseMessage, ConvertRequestMessage } from './message'

const POST_SEND_DELAY = 3000
const POST_RECV_DELAY = 100

customElements.define('demo-container', class DemoContainerElement extends HTMLElement {
    private _worker?: Worker
    private _sent?: number
    private _dirty = false
    private _input
    private _textOutput
    private _viewOutput

    constructor() {
        super()

        let template = document.getElementById('demo-container-template') as HTMLTemplateElement
        let theme = document.getElementById('theme')

        let shadowRoot = this.attachShadow({ mode: 'open' })
        shadowRoot.append(template.content.cloneNode(true))
        if (theme) {
            shadowRoot.append(theme.cloneNode(false))
        }
        this._input = shadowRoot.getElementById('text-in') as HTMLTextAreaElement
        this._textOutput = shadowRoot.getElementById('text-out') as HTMLElement
        this._viewOutput = shadowRoot.getElementById('view-out') as HTMLIFrameElement
        this._input.onchange = this._input.oninput = () => this._update()
    }

    connectedCallback() {
        const worker = this._worker = new Worker(new URL('./worker.ts', import.meta.url))
        this._update()
        worker.onmessage = (e: MessageEvent<ConvertResponseMessage>) => {
            if ('output' in e.data) {
                this._textOutput.innerText = e.data.output
                const doc = this._viewOutput.contentDocument
                if (doc) {
                    doc.body.innerHTML = e.data.output
                }
            } else {
                console.error(e.data.error)
            }
            this._sent = undefined

            if (this._dirty) {
                setTimeout(() => {
                    this._dirty = false
                    this._update()
                }, POST_RECV_DELAY)
            }
        }
    }


    _update() {
        if (this._dirty) return
        const worker = this._worker
        if (!worker) return
        const now = Date.now()

        if (this._sent != null) {
            this._dirty = true
            if (now - this._sent < POST_SEND_DELAY) return
        }

        this._sent = now
        worker.postMessage({
            input: this._input.value,
        } satisfies ConvertRequestMessage)
    }

    disconnectedCallback() {
        this._worker?.terminate()
        this._worker = undefined
    }
})
