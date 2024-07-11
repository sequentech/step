exports.command = async function (id) {
    const menu = await this.element(id).moveTo()
    this.click(menu)

    return this
}
