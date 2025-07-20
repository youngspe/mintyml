import templateSrc from './collapsible.html'

const template = document.createElement('template')

template.innerHTML = templateSrc

class Collapsible extends HTMLElement {
    static observedAttributes = ['open']
    private _buttonSlot: HTMLSlotElement
    private _contentSlot: HTMLSlotElement

    constructor() {
        super()
        const shadowRoot = this.attachShadow({ mode: 'open' })
        shadowRoot.appendChild(template.content.cloneNode(true))
        const [buttonSlot, labelSlot, contentSlot] = shadowRoot.querySelectorAll('slot')
        const header = shadowRoot.getElementById('header')!

        buttonSlot.ariaExpanded = String(this.hasAttribute('open'))
        contentSlot.hidden = !this.hasAttribute('open')



        this._contentSlot = contentSlot
        this._buttonSlot = buttonSlot

        const toggleVisibility = () => {
            this.toggleAttribute('open')
        }

        header.addEventListener('click', toggleVisibility)
    }

    attributeChangedCallback(name: string, _oldValue: string, newValue: string) {
        if (name === 'open') {
            const isOpen = newValue !== null
            this._buttonSlot.ariaExpanded = String(isOpen)
            this._contentSlot.hidden = !isOpen
        }
    }
}

customElements.define('my-collapsible', Collapsible)
