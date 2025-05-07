;(self.webpackChunk_sequentech_ui_essentials =
    self.webpackChunk_sequentech_ui_essentials || []).push([
    [1988, 3648],
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
        "./src/components/LanguageMenu/__stories__/LanguageMenu.mdx": function (
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
                _LanguageMenu_stories__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(
                    "./src/components/LanguageMenu/__stories__/LanguageMenu.stories.tsx"
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
                                    of: _LanguageMenu_stories__WEBPACK_IMPORTED_MODULE_2__,
                                    title: "components/LanguageMenu",
                                }
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _components.h1,
                                {id: "languagemenu", children: "LanguageMenu"}
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(_components.p, {
                                children:
                                    "A LanguageMenu is the box at the top of every screen. It includes the logo, the\nsoftware version, language toggle and an optional button to log out.",
                            }),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _components.h2,
                                {id: "desktop", children: "Desktop"}
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _storybook_blocks__WEBPACK_IMPORTED_MODULE_4__.Xz,
                                {
                                    of:
                                        _LanguageMenu_stories__WEBPACK_IMPORTED_MODULE_2__.CollapsedMenu,
                                }
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
        "./src/components/LanguageMenu/__stories__/LanguageMenu.stories.tsx": function (
            __unused_webpack_module,
            __webpack_exports__,
            __webpack_require__
        ) {
            __webpack_require__.r(__webpack_exports__),
                __webpack_require__.d(__webpack_exports__, {
                    CollapsedMenu: function () {
                        return CollapsedMenu
                    },
                    ExpandedMenu: function () {
                        return ExpandedMenu
                    },
                    __namedExportsOrder: function () {
                        return __namedExportsOrder
                    },
                })
            var _CollapsedMenu$parame,
                _CollapsedMenu$parame2,
                _CollapsedMenu$parame3,
                _ExpandedMenu$paramet,
                _ExpandedMenu$paramet2,
                _ExpandedMenu$paramet3,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_regeneratorRuntime_js__WEBPACK_IMPORTED_MODULE_7__ = __webpack_require__(
                    "../node_modules/@babel/runtime/helpers/esm/regeneratorRuntime.js"
                ),
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_asyncToGenerator_js__WEBPACK_IMPORTED_MODULE_6__ = __webpack_require__(
                    "../node_modules/@babel/runtime/helpers/esm/asyncToGenerator.js"
                ),
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__ = __webpack_require__(
                    "../node_modules/@babel/runtime/helpers/esm/objectSpread2.js"
                ),
                _LanguageMenu__WEBPACK_IMPORTED_MODULE_1__ =
                    (__webpack_require__("../node_modules/react/index.js"),
                    __webpack_require__("./src/components/LanguageMenu/LanguageMenu.tsx")),
                _mui_material_Box__WEBPACK_IMPORTED_MODULE_4__ = __webpack_require__(
                    "../node_modules/@mui/material/Box/Box.js"
                ),
                _storybook_testing_library__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(
                    "../node_modules/@storybook/testing-library/dist/esm/index.js"
                ),
                react_jsx_runtime__WEBPACK_IMPORTED_MODULE_3__ = __webpack_require__(
                    "../node_modules/react/jsx-runtime.js"
                )
            __webpack_exports__.default = {
                title: "components/LanguageMenu",
                component: _LanguageMenu__WEBPACK_IMPORTED_MODULE_1__.Z,
                parameters: {backgrounds: {default: "light"}},
                argTypes: {},
            }
            var Template = function Template(args) {
                    return (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_3__.jsx)(
                        _mui_material_Box__WEBPACK_IMPORTED_MODULE_4__.Z,
                        {
                            style: {display: "inline-flex", backgroundColor: "white"},
                            children: (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_3__.jsx)(
                                _LanguageMenu__WEBPACK_IMPORTED_MODULE_1__.Z,
                                (0,
                                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                                    {},
                                    args
                                )
                            ),
                        }
                    )
                },
                CollapsedMenu = Template.bind({})
            CollapsedMenu.args = {label: "LanguageMenu", languagesList: ["en", "es"]}
            var ExpandedMenu = Template.bind({})
            ;(ExpandedMenu.args = {label: "LanguageMenu", languagesList: ["en", "es"]}),
                (ExpandedMenu.play = (function () {
                    var _ref2 = (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_asyncToGenerator_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                        (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_regeneratorRuntime_js__WEBPACK_IMPORTED_MODULE_7__.Z)().mark(
                            function _callee(_ref) {
                                var canvasElement, canvas
                                return (0,
                                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_regeneratorRuntime_js__WEBPACK_IMPORTED_MODULE_7__.Z)().wrap(
                                    function _callee$(_context) {
                                        for (;;)
                                            switch ((_context.prev = _context.next)) {
                                                case 0:
                                                    return (
                                                        (canvasElement = _ref.canvasElement),
                                                        (canvas = (0,
                                                        _storybook_testing_library__WEBPACK_IMPORTED_MODULE_2__.uh)(
                                                            canvasElement
                                                        )),
                                                        (_context.next = 4),
                                                        _storybook_testing_library__WEBPACK_IMPORTED_MODULE_2__.mV.click(
                                                            canvas.getByTestId("lang-button-test")
                                                        )
                                                    )
                                                case 4:
                                                case "end":
                                                    return _context.stop()
                                            }
                                    },
                                    _callee
                                )
                            }
                        )
                    )
                    return function (_x) {
                        return _ref2.apply(this, arguments)
                    }
                })()),
                (CollapsedMenu.parameters = (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                    (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                        {},
                        CollapsedMenu.parameters
                    ),
                    {},
                    {
                        docs: (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                            (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                                {},
                                null === (_CollapsedMenu$parame = CollapsedMenu.parameters) ||
                                    void 0 === _CollapsedMenu$parame
                                    ? void 0
                                    : _CollapsedMenu$parame.docs
                            ),
                            {},
                            {
                                source: (0,
                                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                                    {
                                        originalSource:
                                            'args => <Box style={{\n  display: "inline-flex",\n  backgroundColor: "white"\n}}>\n        <LanguageMenu {...args} />\n    </Box>',
                                    },
                                    null === (_CollapsedMenu$parame2 = CollapsedMenu.parameters) ||
                                        void 0 === _CollapsedMenu$parame2 ||
                                        null ===
                                            (_CollapsedMenu$parame3 =
                                                _CollapsedMenu$parame2.docs) ||
                                        void 0 === _CollapsedMenu$parame3
                                        ? void 0
                                        : _CollapsedMenu$parame3.source
                                ),
                            }
                        ),
                    }
                )),
                (ExpandedMenu.parameters = (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                    (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                        {},
                        ExpandedMenu.parameters
                    ),
                    {},
                    {
                        docs: (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                            (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                                {},
                                null === (_ExpandedMenu$paramet = ExpandedMenu.parameters) ||
                                    void 0 === _ExpandedMenu$paramet
                                    ? void 0
                                    : _ExpandedMenu$paramet.docs
                            ),
                            {},
                            {
                                source: (0,
                                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                                    {
                                        originalSource:
                                            'args => <Box style={{\n  display: "inline-flex",\n  backgroundColor: "white"\n}}>\n        <LanguageMenu {...args} />\n    </Box>',
                                    },
                                    null === (_ExpandedMenu$paramet2 = ExpandedMenu.parameters) ||
                                        void 0 === _ExpandedMenu$paramet2 ||
                                        null ===
                                            (_ExpandedMenu$paramet3 =
                                                _ExpandedMenu$paramet2.docs) ||
                                        void 0 === _ExpandedMenu$paramet3
                                        ? void 0
                                        : _ExpandedMenu$paramet3.source
                                ),
                            }
                        ),
                    }
                ))
            var __namedExportsOrder = ["CollapsedMenu", "ExpandedMenu"]
        },
        "?d91c": function () {},
    },
])
