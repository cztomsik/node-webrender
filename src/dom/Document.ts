// TODO: cyclic
import { Node } from './Node'

import { native } from '../native'
import {
  NodeList,
  Text,
  Comment,
  DocumentFragment,
  HTMLHtmlElement,
  HTMLHeadElement,
  HTMLLinkElement,
  HTMLBodyElement,
  HTMLStyleElement,
  HTMLScriptElement,
  HTMLDivElement,
  HTMLSpanElement,
  HTMLInputElement,
  HTMLIFrameElement,
  HTMLTextAreaElement,
  HTMLButtonElement,
  HTMLUnknownElement,
  HTMLAnchorElement,
  SVGElement,
  SVGSVGElement,
  SVGGElement,
  HTMLTableSectionElement,
  HTMLTableElement,
  HTMLTableCellElement,
  HTMLTableHeaderCellElement,
  HTMLTableRowElement,
  DOMImplementation,
} from './index'

import { StyleSheetList } from '../css/StyleSheetList'
import { UNSUPPORTED } from '../util'

import { GET_THE_PARENT } from '../events/EventTarget'
import { Event } from '../events/Event'

export class Document extends Node implements globalThis.Document {
  readonly ownerDocument
  readonly defaultView: Window & typeof globalThis | null = null
  readonly implementation = new DOMImplementation()
  readonly childNodes = new NodeList<ChildNode>()
  readonly compatMode = 'CSS1Compat'

  // TODO: getter, should be last focused (from symbol?) or body (which can be null sometimes)
  readonly activeElement: Element | null = null

  constructor() {
    // TS defines Node.ownerDocument as nullable but redefines it on every subclass except Document
    super(null as any)

    // non-standard, we are using parent.ownerDocument in Node.insertBefore()
    // and we are using doc.appendChild() during parsing
    // if it's ever a problem we could use child.ownerDocument
    this.ownerDocument = this

    initDocument(this)
  }

  get nodeType() {
    return Node.DOCUMENT_NODE
  }

  get nodeName() {
    return '#document'
  }

  get documentElement() {
    // chrome allows removing root & appending a new one
    return this.childNodes[0] ?? null
  }

  get head() {
    return this.documentElement?.childNodes.find(n => n.localName === 'head') ?? null
  }

  get body() {
    return this.documentElement?.childNodes.find(n => n.localName === 'body') ?? null
  }

  get title() {
    return this.head?.childNodes.find(n => n.localName === 'title')?.data ?? ''
  }

  set title(title) {
    const head = this.head || this.appendChild(this.createElement('head'))
    const titleEl = head.childNodes.find(n => n.localName === 'title') ?? head.appendChild(this.createElement('title'))

    titleEl.data = title
  }

  get location() {
    // DOMParser docs should have null (TS is wrong)
    return this.defaultView?.location ?? (null as any)
  }

  // TODO: basic custom elements (no shadow DOM)
  createElement(tagName: string, options?) {
    // happy-case
    // - tagName in lowercase means it's also localName
    // - simple comparison of interned strings
    // - ordered by likelihood
    switch (tagName) {
      case 'div': return new HTMLDivElement(this, tagName)
      case 'span': return new HTMLSpanElement(this, tagName)
      case 'a': return new HTMLAnchorElement(this, tagName)
      case 'button': return new HTMLButtonElement(this, tagName)
      case 'input': return new HTMLInputElement(this, tagName)
      case 'textarea': return new HTMLTextAreaElement(this, tagName)
      case 'table': return new HTMLTableElement(this, tagName)
      case 'thead': return new HTMLTableSectionElement(this, tagName)
      case 'tbody': return new HTMLTableSectionElement(this, tagName)
      case 'tr': return new HTMLTableRowElement(this, tagName)
      case 'td': return new HTMLTableCellElement(this, tagName)
      case 'th': return new HTMLTableHeaderCellElement(this, tagName)
      case 'style': return new HTMLStyleElement(this, tagName)
      case 'script': return new HTMLScriptElement(this, tagName)
      //case 'canvas': return new HTMLCanvasElement(this, tagName)
      case 'link': return new HTMLLinkElement(this, tagName)
      case 'iframe': return new HTMLIFrameElement(this, tagName)
      case 'head': return new HTMLHeadElement(this, tagName)
      case 'body': return new HTMLBodyElement(this, tagName)
      case 'html': return new HTMLHtmlElement(this, tagName)

      // otherwise try lowercase and eventually fall-back to HTMLUnknownElement
      default:
        if (typeof tagName !== 'string') {
          tagName = '' + tagName
        }

        const lower = tagName.toLowerCase()

        if (tagName === lower) {
          return new HTMLUnknownElement(this, tagName)
        }

        return this.createElement(lower as any)
    }
  }

  createElementNS(ns: string | null, tagName: string, options?): any {
    switch (ns) {
      case 'http://www.w3.org/2000/svg':
        switch (tagName) {
          case 'svg': return new SVGSVGElement(this, tagName)
          case 'g': return new SVGGElement(this, tagName)
          default: return new SVGElement(this, tagName)
        }

      default:
        return new HTMLUnknownElement(this, tagName)
    }
  }

  createTextNode(data: string): Text {
    return new Text(data, this)
  }

  createComment(data: string): Comment {
    return new Comment(data, this)
  }

  createDocumentFragment(): DocumentFragment {
    return new DocumentFragment(this)
  }

  elementFromPoint(x, y): Element | null {
    // TODO: find a better way to get that id (and to call native)
    let id = native.viewport_element_from_point(this['__VIEWPORT_ID'], x, y)

    return id && lookup(this, id)
  }

  hasFocus(): boolean {
    // TODO: not sure if it shouldn't also be !== body
    return !!this.activeElement
  }

  get isConnected(): boolean {
    return true
  }

  [GET_THE_PARENT]() {
    return this.defaultView as any
  }

  get styleSheets(): StyleSheetList {
    return new StyleSheetList(this.getElementsByTagName('style').map(s => s.sheet))
  }

  get forms() {
    return this.getElementsByTagName('form')
  }
  get images() {
    return this.getElementsByTagName('img')
  }
  get links() {
    return this.getElementsByTagName('link')
  }
  get scripts() {
    return this.getElementsByTagName('script')
  }

  getElementById(id) {
    return this.querySelector(`#${id}`)
  }

  createEvent(type) {
    // TODO: return appropriate subclass
    return new Event(type.toLowerCase()) as any
  }

  // intentionally left out (out-of-scope)
  clear = UNSUPPORTED
  close = UNSUPPORTED
  open = UNSUPPORTED
  write = UNSUPPORTED
  writeln = UNSUPPORTED
  adoptNode = UNSUPPORTED
  importNode = UNSUPPORTED
  createAttribute = UNSUPPORTED
  createAttributeNS = UNSUPPORTED

  // maybe later
  caretPositionFromPoint
  characterSet
  charset
  contentType
  cookie
  createCDATASection
  createExpression
  createNodeIterator
  createNSResolver
  createProcessingInstruction
  createRange
  createTreeWalker
  currentScript
  designMode
  dir
  doctype
  documentURI
  domain
  elementsFromPoint
  embeds
  evaluate
  execCommand
  exitFullscreen
  exitPointerLock
  fullscreenElement
  fullscreenEnabled
  getAnimations
  getElementsByName
  getSelection
  hidden
  inputEncoding
  lastModified
  origin
  plugins
  pointerLockElement
  queryCommandEnabled
  queryCommandIndeterm
  queryCommandState
  queryCommandSupported
  queryCommandValue
  readyState
  referrer
  scrollingElement
  timeline
  URL
  visibilityState

  // deprecated
  alinkColor
  all
  anchors
  applets
  bgColor
  fgColor
  fullscreen
  linkColor
  captureEvents
  caretRangeFromPoint
  releaseEvents
  vlinkColor
}

type Doc = Document

declare global {
  interface Document extends Doc {}
}

const DOC_ID = Symbol()
const REFS = Symbol()
const NODE_REGISTRY = Symbol()
const NODE_ID = Symbol()

const initDocument = (doc) => {
  doc[DOC_ID] = native.document_new()
  doc[REFS] = []
  doc[NODE_REGISTRY] = new FinalizationRegistry(id => native.document_drop_node(doc[DOC_ID], id))
  initNode(doc, doc, 0)

  DOCUMENT_REGISTRY.register(doc, doc[DOC_ID])
}

const initNode = (doc, node, id) => {
  node[NODE_ID] = id
  doc[REFS][id] = new WeakRef(node)
  doc[NODE_REGISTRY].register(node, id)
}

const lookup = (doc, id) => (id && doc[REFS][id]?.deref()) ?? null

// package-private
export const getDocId = (doc) => doc[DOC_ID]
export const initTextNode = (doc, node, cdata) => initNode(doc, node, native.document_create_text_node(doc[DOC_ID], cdata))
export const initComment = (doc, node, cdata) => initNode(doc, node, native.document_create_comment(doc[DOC_ID], cdata))
export const setCdata = (doc, node, cdata) => native.document_set_cdata(doc[DOC_ID], node[NODE_ID], cdata)
export const initElement = (doc, el, localName) => initNode(doc, el, native.document_create_element(doc[DOC_ID], localName))
export const getAttribute = (doc, el, k) => native.document_attribute(doc[DOC_ID], el[NODE_ID], k)
export const setAttribute = (doc, el, k, v) => native.document_set_attribute(doc[DOC_ID], el[NODE_ID], k, v)
export const removeAttribute = (doc, el, k) => native.document_remove_attribute(doc[DOC_ID], el[NODE_ID], k)
export const getAttributeNames = (doc, el) => native.document_attribute_names(doc[DOC_ID], el[NODE_ID])
export const getNodeRect = (doc, el) => {
  const [left, top, width, height] = native.viewport_node_rect(doc.__VIEWPORT_ID, el[NODE_ID])
  return { left, top, width, height }
}
export const setElementStyleProp = (doc, el, prop, val) => native.document_set_element_style_property(doc[DOC_ID], el[NODE_ID], prop, val)
export const insertChild = (doc, parent, child, index) => native.document_insert_child(doc[DOC_ID], parent[NODE_ID], child[NODE_ID], index)
export const removeChild = (doc, parent, child) => native.document_remove_child(doc[DOC_ID], parent[NODE_ID], child[NODE_ID])
export const matches = (doc, el, sel) => lookup(doc, native.document_matches(doc[DOC_ID], el[NODE_ID], sel))
export const querySelector = (doc, ctxNode, sel) => lookup(doc, native.document_query_selector(doc[DOC_ID], ctxNode[NODE_ID], sel))
export const querySelectorAll = (doc, ctxNode, sel) => native.document_query_selector_all(doc[DOC_ID], ctxNode[NODE_ID], sel).map(id => lookup(doc, id))

const DOCUMENT_REGISTRY = new FinalizationRegistry(id => native.document_drop(id))
