import { Element } from './index'
import { CSSStyleDeclaration } from '../css/CSSStyleDeclaration'
import { setElementStyleProp, getNodeRect } from './Document'

export abstract class HTMLElement extends Element implements globalThis.HTMLElement {
  #style

  get style() {
    if (this.#style === undefined) {
      this.#style = new CSSStyleDeclaration(null, (prop, value) =>
        setElementStyleProp(this.ownerDocument, this, prop, value)
      )
    }

    return this.#style
  }

  get tagName() {
    return this.localName.toUpperCase()
  }

  click() {
    this._fire('click')
  }

  blur() {
    if (this.ownerDocument.activeElement !== this) {
      return
    }

    this._fire('blur')
    // TODO: should be in Document
    ;(this.ownerDocument as any).activeElement = null
  }

  focus() {
    if (this.ownerDocument.activeElement === this) {
      return
    }

    if (this.ownerDocument.activeElement instanceof HTMLElement) {
      this.ownerDocument.activeElement.blur()
    }

    // TODO: should be in Document
    ;(this.ownerDocument as any).activeElement = this
    this._fire('focus')
  }

  get offsetParent() {
    return this.parentElement
  }

  get offsetLeft() {
    return getNodeRect(this.ownerDocument, this).left
  }

  get offsetTop() {
    return getNodeRect(this.ownerDocument, this).top
  }

  get offsetWidth() {
    return getNodeRect(this.ownerDocument, this).width
  }

  get offsetHeight() {
    return getNodeRect(this.ownerDocument, this).height
  }

  // later
  enterKeyHint
  accessKey
  accessKeyLabel
  autocapitalize
  autofocus
  contentEditable
  dataset
  dir
  draggable
  hidden
  inputMode
  isContentEditable
  lang
  spellcheck
  tabIndex
  title
  translate
}
