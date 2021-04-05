export class Event implements globalThis.Event {
  readonly timeStamp = Date.now()
  readonly bubbles = true
  readonly cancelable = true
  readonly composed = false
  isTrusted = true

  // TODO: phasing
  eventPhase = 0
  cancelBubble = false
  cancelBubbleImmediately = false
  defaultPrevented = false

  target: EventTarget | null = null
  currentTarget: EventTarget | null = null

  constructor(public type: string, eventInit = undefined) {
    Object.assign(this, eventInit)
  }

  // deprecated, kept for WPT
  initEvent(type: string, bubbles?: boolean, cancelable?: boolean) {
    Object.assign(this, { type, bubbles, cancelable, cancelBubble: false, defaultPrevented: false })
  }

  preventDefault() {
    if (this.cancelable) {
      this.defaultPrevented = true
    }
  }

  stopPropagation() {
    this.cancelBubble = true
  }

  stopImmediatePropagation() {
    this.cancelBubbleImmediately = true
    this.stopPropagation()
  }

  get srcElement() {
    return this.target
  }

  get returnValue() {
    return !this.cancelBubble
  }

  set returnValue(v) {
    if (v === false) {
      this.preventDefault()
    }
  }

  static readonly NONE = 0
  static readonly CAPTURING_PHASE = 1
  static readonly AT_TARGET = 2
  static readonly BUBBLING_PHASE = 3

  // later
  composedPath

  NONE
  CAPTURING_PHASE
  AT_TARGET
  BUBBLING_PHASE
}
