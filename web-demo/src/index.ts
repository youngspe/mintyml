import { ConvertResponseMessage, ConvertRequestMessage } from './message'

import '../index.scss'

import * as ace from 'ace-builds'
import 'ace-builds/src-noconflict/theme-solarized_dark'
import 'ace-builds/src-noconflict/theme-solarized_light'
import 'ace-builds/src-noconflict/mode-html'
import 'ace-builds/src-noconflict/ext-searchbox'

function updateURL({ params, hash, push = false }: {
    params?: Record<string, string | null>,
    hash?: string | null,
    push?: boolean,
}) {
    const url = new URL(window.location.href)
    if (params) {
        for (const [key, value] of Object.entries(params)) {
            if (value == null) {
                url.searchParams.delete(key)
            } else {
                url.searchParams.set(key, value)
            }
        }
    }

    if (hash !== undefined) {
        if (hash == null) {
            url.hash = ''
        } else {
            url.hash = hash
        }
    }

    if (push) {
        history.pushState(null, '', url)
    } else {
        history.replaceState(null, '', url)
    }
}

async function loadExample(exampleName: string) {
    let res = fetch(`${document.baseURI}examples/${exampleName}.mty`, {
        headers: { 'Accept': 'text/plain' },
    })
    return await (await res).text()
}

const POST_SEND_DELAY = 3000
const POST_RECV_DELAY = 100

const storedTextInputKey = 'stored-text-input'

class Demo {
    private _worker?: Worker
    private _sent?: number
    private _dirty = false
    private _viewOutputContainer
    private _editor
    private _textOutput
    private _media
    private _exampleSelect
    private _resetButton
    private _isLoadingExample = true
    private _lastInput: string | null = null

    constructor(private _root: HTMLElement, private _exampleName: string, private _examples: string[]) {
        let host = (_root.getRootNode() ?? document) as DocumentFragment
        this._exampleSelect = host.getElementById('example-select') as HTMLSelectElement

        this._resetButton = host.getElementById('reset-button') as HTMLButtonElement
        this._viewOutputContainer = host.getElementById('view-out') as HTMLIFrameElement

        const viewOutDoc = this._viewOutputContainer.contentDocument
        if (viewOutDoc) {
            const frameStyle = viewOutDoc.createElement('style')
            frameStyle.innerText = `
            :root {
                inline-size: 100%;
            }
            body {
                inline-size: max-content;
                max-inline-size: 100%;
                margin-inline: auto;
                margin-block: var(--margin-block-wide);
            }
            `
            viewOutDoc.head.appendChild(frameStyle)
            const theme = document.createElement('link')
            theme.rel = "stylesheet"
            theme.href = `${document.baseURI}theme.css`
            viewOutDoc.head.appendChild(theme)
            const base = document.createElement('base')
            base.target = "_parent"
            viewOutDoc.head.appendChild(base)
        }

        {
            let options = {
                fontSize: parseFloat(getComputedStyle(document.documentElement).fontSize) * 0.75,
                fontFamily: 'monospace',
                tabSize: 2,
                printMargin: 80,
                wrap: true,
            } satisfies Partial<ace.Ace.EditorOptions>

            const editElement = host.getElementById('editor')!

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
                printMargin: false,
                mode: 'ace/mode/html',
                useWorker: false,
            })
        }
        this._media = window.matchMedia("(prefers-color-scheme: dark)")
    }

    private _updateExampleNames() {
        for (const name of this._examples) {
            let option = this._exampleSelect.options.namedItem(name)
            if (!option) {
                option = document.createElement('option')
                option.setAttribute('name', name)
                option.value = name
                this._exampleSelect.options.add(option)
            }
            option.text = localStorage.getItem(`${storedTextInputKey}/${name}`) !== null ? `${name}*` : name
        }
        this._exampleSelect.value = this._exampleName
    }

    private _updateDoc(src: string) {
        const contentDocument = this._viewOutputContainer.contentDocument
        if (!contentDocument) return
        const srcDom = new DOMParser().parseFromString(src, 'text/html')

        const outHead = contentDocument.head

        const base = outHead.querySelector('base')
        if (base) {
            base.href = window.location.href
        }

        for (const child of outHead.querySelectorAll('.-user-provided')) {
            child.remove()
        }

        const srcHeadChildren = srcDom.head.querySelectorAll('title,link,style')
        for (const child of srcHeadChildren) {
            child.classList.add('-user-provided')
        }

        outHead.append(...srcHeadChildren)

        contentDocument.querySelector('body')?.replaceWith(srcDom.body)

        if (this._shouldShowElementInHash) {
            Promise.resolve().then(() => this._showElementInHash())
        }
    }

    private _onSchemeChange = (e: MediaQueryListEvent) => {
        this._updateTheme(e.matches)
    }

    private async _loadExample(exampleName: string) {
        updateURL({ params: { example: exampleName }, hash: null })
        this._isLoadingExample = true
        this._editor.session.clearAnnotations()
        let exampleText = localStorage.getItem(
            `${storedTextInputKey}/${exampleName}`
        ) ?? await loadExample(exampleName)
        this._exampleName = exampleName
        this._editor.setValue(exampleText, 0)
        this._editor.clearSelection()
        this._update()
    }

    private _onExampleSelect = async (e: Event) => {
        this._loadExample(this._exampleSelect.value)
    }

    private _onResetClick = async () => {
        this._isLoadingExample = true
        this._clearModified()
        await this._loadExample(this._exampleName)
    }

    private _shouldShowElementInHash = true

    private _showElementInHash = () => {
        this._shouldShowElementInHash = false
        const hash = document.location.hash
        if (hash.length < 2) return
        const id = hash.slice(1)

        const target = this._viewOutputContainer
            .contentDocument
            ?.getElementById(id)

        if (target) {
            target.scrollIntoView()
        } else {
            this._shouldShowElementInHash = true
        }
    }

    private _markers: number[] = []

    init() {
        this._loadExample(this._exampleName)
        this._updateExampleNames()
        this._exampleSelect.addEventListener('input', this._onExampleSelect)
        this._resetButton.addEventListener('click', this._onResetClick)
        this._media.addEventListener('change', this._onSchemeChange)
        window.addEventListener('hashchange', this._showElementInHash)
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
            }
            this._sent = undefined

            if (this._dirty) {
                setTimeout(() => {
                    this._dirty = false
                    this._update()
                }, POST_RECV_DELAY)
            } else {
                this._isLoadingExample = false
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
            if (now - this._sent < POST_SEND_DELAY) {
                this._dirty = true
                return
            }
        }

        const input = this._editor.getValue()
        if (input == this._lastInput) return
        this._lastInput = input

        this._sent = now
        this._storeModified(input)
        worker.postMessage({ input } satisfies ConvertRequestMessage)
    }

    private _storeModified(input: string) {
        if (this._isLoadingExample) return
        const option = this._exampleSelect.options.namedItem(this._exampleName)
        if (option) { option.text = `${this._exampleName}*` }
        localStorage.setItem(`${storedTextInputKey}/${this._exampleName}`, input)
    }

    private _clearModified() {
        const option = this._exampleSelect.options.namedItem(this._exampleName)
        if (option) { option.text = this._exampleName }
        localStorage.removeItem(`${storedTextInputKey}/${this._exampleName}`)
    }

    deinit() {
        this._worker?.terminate()
        this._worker = undefined
        this._exampleSelect.removeEventListener('change', this._onExampleSelect)
        this._resetButton.removeEventListener('click', this._onResetClick)
        this._media.removeEventListener('change', this._onSchemeChange)
        this._editor.off('input', this._update)
        window.removeEventListener('hashchange', this._showElementInHash)
    }
}

if (matchMedia('(max-width: 480px').matches) {
    (document.querySelector('#text-in-wrapper details') as HTMLDetailsElement).open = false
}

const params = new URLSearchParams(window.location.search)

const exampleName = params.get('example') ?? 'intro'

new Demo(document.querySelector('demo-container')!, exampleName, [
    'intro',
    'table',
    'blockquote',
    'formatting',
]).init()
