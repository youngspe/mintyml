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

        this._viewOutputContainer = host.querySelector('#view-out') as HTMLIFrameElement

        const viewOutDoc = this._viewOutputContainer.contentDocument
        if (viewOutDoc) {
            if (theme) {
                viewOutDoc.head.appendChild(theme.cloneNode(true))
            }
            const base = document.createElement('base')
            base.target = "_parent"
            viewOutDoc.head.appendChild(base)

            this._viewOutput = document.createElement("view-output")
            this._viewOutput.style.display = 'contents'

            viewOutDoc.body.append(this._viewOutput)

        }

        {
            let options = {
                fontSize: parseFloat(getComputedStyle(document.documentElement).fontSize) * 0.75,
                fontFamily: 'monospace',
                tabSize: 2,
                printMargin: false,
            } satisfies Partial<ace.Ace.EditorOptions>

            this._editor = ace.edit(host.getElementById('editor')!, {
                ...options,
                displayIndentGuides: true,
                useSoftTabs: true,
                showGutter: true,
            })
            this._textOutput = ace.edit(host.getElementById('viewer')!, {
                ...options,
                readOnly: true,
                selectionStyle: 'display: none',
                highlightActiveLine: false,
                highlightGutterLine: false,
                highlightIndentGuides: false,
                highlightSelectedWord: false,
                showGutter: false,
                mode: 'ace/mode/html',
                useWorker: false,
            })
        }

        this._media = window.matchMedia("(prefers-color-scheme: dark)")
    }

    private _onSchemeChange = (e: MediaQueryListEvent) => {
        this._updateTheme(e.matches)
    }

    private _markers: number[] = []

    init() {
        this._media.addEventListener('change', this._onSchemeChange)
        this._updateTheme(this._media.matches)

        this._editor.on('input', () => this._update())
        const worker = this._worker = new Worker(new URL('./worker.ts', import.meta.url))
        this._update()
        worker.onmessage = (e: MessageEvent<ConvertResponseMessage>) => {
            let session = this._editor.getSession()

            for (const marker of this._markers) {
                session.removeMarker(marker)
            }
            this._markers = []


            if ('output' in e.data) {
                session.clearAnnotations()
                this._textOutput.setValue(e.data.output, 0)
                this._textOutput.clearSelection()
                if (this._viewOutput) {
                    this._viewOutput.innerHTML = e.data.output
                }
            } else {
                const annotations: ace.Ace.Annotation[] = []
                if (e.data.error.syntax_errors) {
                    let doc = session.getDocument()
                    for (const error of e.data.error.syntax_errors) {
                        let start = doc.indexToPosition(error.start, 0)
                        let end = doc.indexToPosition(error.end, 0)
                        let range = new ace.Range(
                            start.row, start.column,
                            end.row, end.column,
                        )
                        this._markers.push(session.addMarker(range, 'syntax-error', 'text', true))
                        let msg = 'expected' in error ? `Expected ${error.expected.join(' | ')}`
                            : error.message
                        annotations.push({
                            type: 'error',
                            text: msg,
                            row: start.row,
                            column: start.column,
                        })
                    }
                }
                session.setAnnotations(annotations)

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
