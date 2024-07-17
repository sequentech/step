import {NightwatchAPI} from "nightwatch"

type Props = {
    el: string
    text: string
    browser: NightwatchAPI
}

export const assertListItemText = async ({el, text, browser}: Props) => {
    const els = await browser.element.findAll(el)

    return browser.assert.textContains(els[els.length - 1], text)
}
