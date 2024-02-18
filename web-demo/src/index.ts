import { ConvertResponseMessage, ConvertRequestMessage } from './message'

import * as ace from 'ace-builds'
import 'ace-builds/src-noconflict/theme-solarized_dark'
import 'ace-builds/src-noconflict/theme-solarized_light'
import 'ace-builds/src-noconflict/mode-html'


const POST_SEND_DELAY = 3000
const POST_RECV_DELAY = 100

class Demo {
    private _worker?: Worker
    private _sent?: number
    private _dirty = false
    private _viewOutputContainer
    private _viewOutput
    private _editor
    private _textOutput
    private _media

    constructor(private _root: HTMLElement) {
        let theme = document.getElementById('theme')
        let host = (_root.getRootNode() ?? document) as DocumentFragment

        this._viewOutputContainer = host.getElementById('view-out') as HTMLElement

        {
            const viewOutShadow = this._viewOutputContainer.attachShadow({ mode: 'open' })

            if (theme) {
                viewOutShadow.appendChild(theme.cloneNode(true))
            }

            this._viewOutput = document.createElement("view-output")
            this._viewOutput.style.display = 'contents'

            viewOutShadow.append(this._viewOutput)
        }

        let fontSize = parseFloat(getComputedStyle(document.documentElement).fontSize) * 0.8

        this._editor = ace.edit(host.getElementById('editor')!, {
            displayIndentGuides: true,
            useSoftTabs: true,
            tabSize: 2,
            fontFamily: 'monospace',
            showGutter: false,
            fontSize,
        })
        this._textOutput = ace.edit(host.getElementById('viewer')!, {
            readOnly: true,
            selectionStyle: 'display: none',
            highlightActiveLine: false,
            highlightGutterLine: false,
            highlightIndentGuides: false,
            highlightSelectedWord: false,
            showGutter: false,
            cursorStyle: 'smooth',
            fontFamily: 'monospace',
            fontSize,
            mode: 'ace/mode/html',
            useWorker: false,
        })

        this._media = window.matchMedia("(prefers-color-scheme: dark)")
    }

    private _onSchemeChange = (e: MediaQueryListEvent) => {
        this._updateTheme(e.matches)
    }

    init() {
        this._media.addEventListener('change', this._onSchemeChange)
        this._updateTheme(this._media.matches)

        this._editor.on('input', () => this._update())
        const worker = this._worker = new Worker(new URL('./worker.ts', import.meta.url))
        this._update()
        worker.onmessage = (e: MessageEvent<ConvertResponseMessage>) => {
            if ('output' in e.data) {
                this._textOutput.setValue(e.data.output, 0)
                this._textOutput.clearSelection()
                this._viewOutput.innerHTML = e.data.output
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

    _updateTheme(dark: boolean) {
        let theme: string
        if (dark) {
            theme = "ace/theme/solarized_dark"
        } else {
            theme = "ace/theme/solarized_light"
        }
        this._editor.setTheme(theme)
        this._textOutput.setTheme(theme)
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
            input: this._editor.getValue(),
        } satisfies ConvertRequestMessage)
    }

    deinit() {
        this._worker?.terminate()
        this._worker = undefined
        this._media.removeEventListener('change', this._onSchemeChange)
    }
}

const demo = new Demo(document.querySelector('demo-container')!)
demo.init()
