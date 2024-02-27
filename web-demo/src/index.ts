import { ConvertResponseMessage, ConvertRequestMessage } from './message'

import * as ace from 'ace-builds'
import 'ace-builds/src-noconflict/theme-solarized_dark'
import 'ace-builds/src-noconflict/theme-solarized_light'
import 'ace-builds/src-noconflict/mode-html'
import 'ace-builds/src-noconflict/ext-searchbox'


const POST_SEND_DELAY = 3000
const POST_RECV_DELAY = 100

const storedTextInputKey = 'stored-text-input'

function updateQuery(key: string, value: string | null, push: boolean = false) {
    const url = new URL(window.location.href)
    if (value == null) {
        url.searchParams.delete(key)
    } else {
        url.searchParams.set(key, value)
    }

    if (push) {
        history.pushState(null, '', url)
    } else {
        history.replaceState(null, '', url)
    }
}

class Demo {
    private _worker?: Worker
    private _sent?: number
    private _dirty = false
    private _viewOutputContainer
    private _editor
    private _textOutput
    private _media
    private _exampleSelect

    constructor(private _root: HTMLElement, example: string) {
        let host = (_root.getRootNode() ?? document) as DocumentFragment
        this._exampleSelect = host.getElementById('view-out') as HTMLSelectElement
        this._viewOutputContainer = host.getElementById('view-out') as HTMLIFrameElement

        this._viewOutputContainer = host.querySelector('#view-out') as HTMLIFrameElement

        const viewOutDoc = this._viewOutputContainer.contentDocument
        if (viewOutDoc) {
            const theme = document.getElementById('theme')
            if (theme) {
                viewOutDoc.head.appendChild(theme.cloneNode(true))
            }
            const base = document.createElement('base')
            base.target = "_parent"
            viewOutDoc.head.appendChild(base)
        }

        {
            let options = {
                fontSize: parseFloat(getComputedStyle(document.documentElement).fontSize) * 0.75,
                fontFamily: 'monospace',
                tabSize: 2,
                printMargin: false,
            } satisfies Partial<ace.Ace.EditorOptions>

            const editElement = host.getElementById('editor')!

            editElement.textContent = example

            this._editor = ace.edit(editElement, {
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

    private _updateDoc(src: string) {
        const template = document.createElement('template')
        const srcDom = new DOMParser().parseFromString(src, 'text/html')
        template.content.replaceChildren()

        const outHead = this._viewOutputContainer.contentDocument!.head
        for (const child of outHead.querySelectorAll('.-user-provided')) {
            child.remove()
        }

        const srcHeadChildren = srcDom.head.querySelectorAll('title,link,style')
        for (const child of srcHeadChildren) {
            child.classList.add('-user-provided')
        }

        outHead.append(...srcHeadChildren)

        this._viewOutputContainer.contentDocument?.querySelector('body')?.replaceWith(srcDom.body)
    }

    private _onSchemeChange = (e: MediaQueryListEvent) => {
        this._updateTheme(e.matches)
    }

    private _onExampleSelect = (e: Event) => {

    }

    private _markers: number[] = []

    init() {
        this._exampleSelect.addEventListener('change', this._onExampleSelect)
        this._media.addEventListener('change', this._onSchemeChange)
        this._updateTheme(this._media.matches)

        this._editor.on('input', this._update)
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
                this._updateDoc(e.data.output)
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

    private _update = () => {
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
        this._exampleSelect.removeEventListener('change', this._onExampleSelect)
        this._media.removeEventListener('change', this._onSchemeChange)
        this._editor.off('input', this._update)
    }
}

const params = new URLSearchParams(window.location.search)

let exampleName = params.get('example') ?? 'intro'

fetch(`/${exampleName}.mty`, { headers: { 'Accept': 'application/json' } }).then(async res => {
    const example = await res.text()

    const demo = new Demo(document.querySelector('demo-container')!, example)
demo.init()
})

