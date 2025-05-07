;(self.webpackChunk_sequentech_ui_essentials =
    self.webpackChunk_sequentech_ui_essentials || []).push([
    [7729, 9250],
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
        "./src/components/IconButton/__stories__/IconButton.mdx": function (
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
                _IconButton_stories__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(
                    "./src/components/IconButton/__stories__/IconButton.stories.tsx"
                )
            function _createMdxContent(props) {
                const _components = Object.assign(
                    {h1: "h1", p: "p", h2: "h2"},
                    (0,
                    _storybook_addon_essentials_docs_mdx_react_shim__WEBPACK_IMPORTED_MODULE_3__.ah)(),
                    props.components
                )
                return (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsxs)(
                    react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.Fragment,
                    {
                        children: [
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _storybook_blocks__WEBPACK_IMPORTED_MODULE_4__.h_,
                                {
                                    of: _IconButton_stories__WEBPACK_IMPORTED_MODULE_2__,
                                    title: "components/IconButton",
                                }
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _components.h1,
                                {id: "iconbutton", children: "IconButton"}
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(_components.p, {
                                children:
                                    "A IconButton is the box at the top of every screen. It includes the logo, the\nsoftware version, language toggle and an optional button to log out.",
                            }),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _components.h2,
                                {id: "desktop", children: "Desktop"}
                            ),
                            "\n",
                            (0,
                            react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _storybook_blocks__WEBPACK_IMPORTED_MODULE_4__.Xz,
                                {of: _IconButton_stories__WEBPACK_IMPORTED_MODULE_2__.Primary}
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
        "./src/components/IconButton/__stories__/IconButton.stories.tsx": function (
            __unused_webpack_module,
            __webpack_exports__,
            __webpack_require__
        ) {
            __webpack_require__.r(__webpack_exports__),
                __webpack_require__.d(__webpack_exports__, {
                    Primary: function () {
                        return Primary
                    },
                    __namedExportsOrder: function () {
                        return __namedExportsOrder
                    },
                })
            var _Primary$parameters,
                _Primary$parameters2,
                _Primary$parameters2$,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__ = __webpack_require__(
                    "../node_modules/@babel/runtime/helpers/esm/objectSpread2.js"
                ),
                _fortawesome_free_solid_svg_icons__WEBPACK_IMPORTED_MODULE_4__ =
                    (__webpack_require__("../node_modules/react/index.js"),
                    __webpack_require__(
                        "../node_modules/@fortawesome/free-solid-svg-icons/index.mjs"
                    )),
                _IconButton__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(
                    "./src/components/IconButton/IconButton.tsx"
                ),
                _VerticalBox_VerticalBox__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(
                    "./src/components/VerticalBox/VerticalBox.tsx"
                ),
                react_jsx_runtime__WEBPACK_IMPORTED_MODULE_3__ = __webpack_require__(
                    "../node_modules/react/jsx-runtime.js"
                ),
                meta = {
                    title: "components/IconButton",
                    component: function IconButtonExample() {
                        return (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_3__.jsxs)(
                            _VerticalBox_VerticalBox__WEBPACK_IMPORTED_MODULE_2__.Z,
                            {
                                maxWidth: "32px",
                                children: [
                                    (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_3__.jsx)(
                                        _IconButton__WEBPACK_IMPORTED_MODULE_1__.Z,
                                        {
                                            icon:
                                                _fortawesome_free_solid_svg_icons__WEBPACK_IMPORTED_MODULE_4__.nYk,
                                        }
                                    ),
                                    (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_3__.jsx)(
                                        _IconButton__WEBPACK_IMPORTED_MODULE_1__.Z,
                                        {
                                            icon:
                                                _fortawesome_free_solid_svg_icons__WEBPACK_IMPORTED_MODULE_4__.nYk,
                                            variant: "info",
                                        }
                                    ),
                                    (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_3__.jsx)(
                                        _IconButton__WEBPACK_IMPORTED_MODULE_1__.Z,
                                        {
                                            icon:
                                                _fortawesome_free_solid_svg_icons__WEBPACK_IMPORTED_MODULE_4__.nYk,
                                            variant: "warning",
                                        }
                                    ),
                                    (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_3__.jsx)(
                                        _IconButton__WEBPACK_IMPORTED_MODULE_1__.Z,
                                        {
                                            icon:
                                                _fortawesome_free_solid_svg_icons__WEBPACK_IMPORTED_MODULE_4__.nYk,
                                            variant: "error",
                                        }
                                    ),
                                    (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_3__.jsx)(
                                        _IconButton__WEBPACK_IMPORTED_MODULE_1__.Z,
                                        {
                                            icon:
                                                _fortawesome_free_solid_svg_icons__WEBPACK_IMPORTED_MODULE_4__.nYk,
                                            variant: "success",
                                        }
                                    ),
                                ],
                            }
                        )
                    },
                    parameters: {backgrounds: {default: "white"}},
                }
            __webpack_exports__.default = meta
            var Primary = {args: {}, parameters: {viewport: {disable: !0}}}
            Primary.parameters = (0,
            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                    {},
                    Primary.parameters
                ),
                {},
                {
                    docs: (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                        (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                            {},
                            null === (_Primary$parameters = Primary.parameters) ||
                                void 0 === _Primary$parameters
                                ? void 0
                                : _Primary$parameters.docs
                        ),
                        {},
                        {
                            source: (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                                {
                                    originalSource:
                                        "{\n  // More on args: https://storybook.js.org/docs/react/writing-stories/args\n  args: {},\n  parameters: {\n    viewport: {\n      disable: true\n    }\n  }\n}",
                                },
                                null === (_Primary$parameters2 = Primary.parameters) ||
                                    void 0 === _Primary$parameters2 ||
                                    null === (_Primary$parameters2$ = _Primary$parameters2.docs) ||
                                    void 0 === _Primary$parameters2$
                                    ? void 0
                                    : _Primary$parameters2$.source
                            ),
                        }
                    ),
                }
            )
            var __namedExportsOrder = ["Primary"]
        },
    },
])
