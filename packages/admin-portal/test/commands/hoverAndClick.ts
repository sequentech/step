exports.command = async function (
    el:
        | string
        | {
              hoverElement: string
              clickElement: string
          }
) {
    console.log({el})

    // if el is an object
    if (typeof el === "object") {
        // then it contains an element to hover and one to click
        this.moveToElement(el.hoverElement, 10, 10).click(el.clickElement)

        return this
    }

    this.moveToElement(el, 10, 10).click(el)

    return this
}
