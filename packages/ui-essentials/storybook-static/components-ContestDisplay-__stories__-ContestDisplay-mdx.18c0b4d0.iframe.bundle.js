;(self.webpackChunk_sequentech_ui_essentials =
    self.webpackChunk_sequentech_ui_essentials || []).push([
    [6489],
    {
        "../node_modules/@mdx-js/react/lib/index.js": function (
            __unused_webpack_module,
            __webpack_exports__,
            __webpack_require__
        ) {
            __webpack_require__.d(__webpack_exports__, {
                NF: function () {
                    return withMDXComponents
                },
                Zo: function () {
                    return MDXProvider
                },
                ah: function () {
                    return useMDXComponents
                },
                pC: function () {
                    return MDXContext
                },
            })
            var react__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(
                "../node_modules/react/index.js"
            )
            const MDXContext = react__WEBPACK_IMPORTED_MODULE_0__.createContext({})
            function withMDXComponents(Component) {
                return function boundMDXComponent(props) {
                    const allComponents = useMDXComponents(props.components)
                    return react__WEBPACK_IMPORTED_MODULE_0__.createElement(Component, {
                        ...props,
                        allComponents: allComponents,
                    })
                }
            }
            function useMDXComponents(components) {
                const contextComponents = react__WEBPACK_IMPORTED_MODULE_0__.useContext(MDXContext)
                return react__WEBPACK_IMPORTED_MODULE_0__.useMemo(
                    () =>
                        "function" == typeof components
                            ? components(contextComponents)
                            : {...contextComponents, ...components},
                    [contextComponents, components]
                )
            }
            const emptyObject = {}
            function MDXProvider({components, children, disableParentContext}) {
                let allComponents
                return (
                    (allComponents = disableParentContext
                        ? "function" == typeof components
                            ? components({})
                            : components || emptyObject
                        : useMDXComponents(components)),
                    react__WEBPACK_IMPORTED_MODULE_0__.createElement(
                        MDXContext.Provider,
                        {value: allComponents},
                        children
                    )
                )
            }
        },
        "./src/components/ContestDisplay/__stories__/ContestDisplay.mdx": function (
            __unused_webpack_module,
            __webpack_exports__,
            __webpack_require__
        ) {
            __webpack_require__.r(__webpack_exports__)
            __webpack_require__("../node_modules/react/index.js")
            var react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(
                    "../node_modules/react/jsx-runtime.js"
                ),
                _storybook_addon_essentials_docs_mdx_react_shim__WEBPACK_IMPORTED_MODULE_3__ = __webpack_require__(
                    "../node_modules/@mdx-js/react/lib/index.js"
                ),
                _storybook_blocks__WEBPACK_IMPORTED_MODULE_4__ = __webpack_require__(
                    "../node_modules/@storybook/blocks/dist/index.mjs"
                ),
                _ContestDisplay_stories__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(
                    "./src/components/ContestDisplay/__stories__/ContestDisplay.stories.tsx"
                )
            function _createMdxContent(props) {
                const _components = Object.assign(
                    {h1: "h1", p: "p", blockquote: "blockquote", strong: "strong", h2: "h2"},
                    (0,
                    _storybook_addon_essentials_docs_mdx_react_shim__WEBPACK_IMPORTED_MODULE_3__.ah)(),
                    props.components
                )
                return (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsxs)(
                    react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.Fragment,
                    {
                        children: [
                            (0,
                            react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _storybook_blocks__WEBPACK_IMPORTED_MODULE_4__.h_,
                                {title: "components/ContestDisplay"}
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _components.h1,
                                {id: "contestdisplay", children: "ContestDisplay"}
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(_components.p, {
                                children:
                                    "A ContestDisplay is a Contest in an election. It may be checkable or not.",
                            }),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsxs)(
                                _components.blockquote,
                                {
                                    children: [
                                        "\n",
                                        (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsxs)(
                                            _components.p,
                                            {
                                                children: [
                                                    (0,
                                                    react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                                        _components.strong,
                                                        {children: "Note:"}
                                                    ),
                                                    " This is a simplified version of the ContestDisplay component for Storybook documentation purposes. The actual component has more features and functionality.",
                                                ],
                                            }
                                        ),
                                        "\n",
                                    ],
                                }
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _components.h2,
                                {id: "primary", children: "Primary"}
                            ),
                            "\n",
                            (0,
                            react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _storybook_blocks__WEBPACK_IMPORTED_MODULE_4__.Xz,
                                {of: _ContestDisplay_stories__WEBPACK_IMPORTED_MODULE_2__.Primary}
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _components.h2,
                                {id: "vote", children: "Vote"}
                            ),
                            "\n",
                            (0,
                            react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _storybook_blocks__WEBPACK_IMPORTED_MODULE_4__.Xz,
                                {of: _ContestDisplay_stories__WEBPACK_IMPORTED_MODULE_2__.Vote}
                            ),
                        ],
                    }
                )
            }
            __webpack_exports__.default = function MDXContent(props = {}) {
                const {wrapper: MDXLayout} = Object.assign(
                    {},
                    (0,
                    _storybook_addon_essentials_docs_mdx_react_shim__WEBPACK_IMPORTED_MODULE_3__.ah)(),
                    props.components
                )
                return MDXLayout
                    ? (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                          MDXLayout,
                          Object.assign({}, props, {
                              children: (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                  _createMdxContent,
                                  props
                              ),
                          })
                      )
                    : _createMdxContent(props)
            }
        },
    },
])
