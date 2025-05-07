(self.webpackChunk_sequentech_ui_essentials =
    self.webpackChunk_sequentech_ui_essentials || []).push([
    [2994, 2611],
    {
        "../node_modules/@babel/runtime/helpers/esm/objectDestructuringEmpty.js": function (
            __unused_webpack_module,
            __webpack_exports__,
            __webpack_require__
        ) {
            function _objectDestructuringEmpty(obj) {
                if (null == obj) throw new TypeError("Cannot destructure " + obj)
            }
            __webpack_require__.d(__webpack_exports__, {
                Z: function () {
                    return _objectDestructuringEmpty
                },
            })
        },
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
        "./src/stories/material/Paper/Paper.mdx": function (
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
                _Paper_stories__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(
                    "./src/stories/material/Paper/Paper.stories.tsx"
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
                                    of: _Paper_stories__WEBPACK_IMPORTED_MODULE_2__,
                                    title: "material/Button",
                                }
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _components.h1,
                                {id: "paper", children: "Paper"}
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(_components.p, {
                                children:
                                    "A Paper is the box at the top of every screen. It includes the logo, the\nsoftware version, language toggle and an optional button to log out.",
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
                                {of: _Paper_stories__WEBPACK_IMPORTED_MODULE_2__.Fixed}
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
        "./src/stories/material/Paper/Paper.stories.tsx": function (
            __unused_webpack_module,
            __webpack_exports__,
            __webpack_require__
        ) {
            __webpack_require__.r(__webpack_exports__),
                __webpack_require__.d(__webpack_exports__, {
                    Dashed: function () {
                        return Dashed
                    },
                    Fixed: function () {
                        return Fixed
                    },
                    Responsive: function () {
                        return Responsive
                    },
                    __namedExportsOrder: function () {
                        return __namedExportsOrder
                    },
                })
            var _Fixed$parameters,
                _Fixed$parameters2,
                _Fixed$parameters2$do,
                _Responsive$parameter,
                _Responsive$parameter2,
                _Responsive$parameter3,
                _Dashed$parameters,
                _Dashed$parameters2,
                _Dashed$parameters2$d,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__ = __webpack_require__(
                    "../node_modules/@babel/runtime/helpers/esm/objectSpread2.js"
                ),
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectDestructuringEmpty_js__WEBPACK_IMPORTED_MODULE_3__ = __webpack_require__(
                    "../node_modules/@babel/runtime/helpers/esm/objectDestructuringEmpty.js"
                ),
                _mui_material_Paper__WEBPACK_IMPORTED_MODULE_4__ =
                    (__webpack_require__("../node_modules/react/index.js"),
                    __webpack_require__("../node_modules/@mui/material/Paper/Paper.js")),
                _services_theme__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(
                    "./src/services/theme.ts"
                ),
                react_jsx_runtime__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(
                    "../node_modules/react/jsx-runtime.js"
                ),
                meta = {
                    title: "material/Paper",
                    component: function PaperType(_ref) {
                        var props = Object.assign(
                            {},
                            ((0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectDestructuringEmpty_js__WEBPACK_IMPORTED_MODULE_3__.Z)(
                                _ref
                            ),
                            _ref)
                        )
                        return (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_2__.jsx)(
                            _mui_material_Paper__WEBPACK_IMPORTED_MODULE_4__.Z,
                            (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                                {className: "fff"},
                                props
                            )
                        )
                    },
                    parameters: {backgrounds: {default: "light"}},
                    argTypes: {},
                }
            __webpack_exports__.default = meta
            var Fixed = {
                    args: {
                        variant: "fixed",
                        sx: {justifyContent: "center", alignItems: "center", display: "flex"},
                        children: (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_2__.jsx)(
                            react_jsx_runtime__WEBPACK_IMPORTED_MODULE_2__.Fragment,
                            {
                                children: (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_2__.jsx)(
                                    _mui_material_Paper__WEBPACK_IMPORTED_MODULE_4__.Z,
                                    {
                                        sx: {
                                            width: "10rem",
                                            height: "10rem",
                                            backgroundColor: "".concat(
                                                _services_theme__WEBPACK_IMPORTED_MODULE_1__.rS
                                                    .palette.customGrey.light,
                                                " !important"
                                            ),
                                        },
                                    }
                                ),
                            }
                        ),
                    },
                },
                Responsive = {
                    args: {
                        variant: "responsive",
                        sx: {justifyContent: "center", alignItems: "center", display: "flex"},
                        children: (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_2__.jsx)(
                            react_jsx_runtime__WEBPACK_IMPORTED_MODULE_2__.Fragment,
                            {
                                children: (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_2__.jsx)(
                                    _mui_material_Paper__WEBPACK_IMPORTED_MODULE_4__.Z,
                                    {
                                        sx: {
                                            width: "10rem",
                                            height: "10rem",
                                            backgroundColor: "".concat(
                                                _services_theme__WEBPACK_IMPORTED_MODULE_1__.rS
                                                    .palette.customGrey.light,
                                                " !important"
                                            ),
                                        },
                                    }
                                ),
                            }
                        ),
                    },
                },
                Dashed = {
                    args: {
                        variant: "dashed",
                        sx: {justifyContent: "center", alignItems: "center", display: "flex"},
                        children: (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_2__.jsx)(
                            react_jsx_runtime__WEBPACK_IMPORTED_MODULE_2__.Fragment,
                            {
                                children: (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_2__.jsx)(
                                    _mui_material_Paper__WEBPACK_IMPORTED_MODULE_4__.Z,
                                    {
                                        sx: {
                                            width: "10rem",
                                            height: "10rem",
                                            backgroundColor: "".concat(
                                                _services_theme__WEBPACK_IMPORTED_MODULE_1__.rS
                                                    .palette.customGrey.light,
                                                " !important"
                                            ),
                                        },
                                    }
                                ),
                            }
                        ),
                    },
                }
            ;(Fixed.parameters = (0,
            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                    {},
                    Fixed.parameters
                ),
                {},
                {
                    docs: (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                        (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                            {},
                            null === (_Fixed$parameters = Fixed.parameters) ||
                                void 0 === _Fixed$parameters
                                ? void 0
                                : _Fixed$parameters.docs
                        ),
                        {},
                        {
                            source: (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                                {
                                    originalSource:
                                        '{\n  args: {\n    variant: "fixed",\n    sx: {\n      justifyContent: "center",\n      alignItems: "center",\n      display: "flex"\n    },\n    children: <>\n                <Paper sx={{\n        width: "10rem",\n        height: "10rem",\n        backgroundColor: `${theme.palette.customGrey.light} !important`\n      }} />\n            </>\n  }\n}',
                                },
                                null === (_Fixed$parameters2 = Fixed.parameters) ||
                                    void 0 === _Fixed$parameters2 ||
                                    null === (_Fixed$parameters2$do = _Fixed$parameters2.docs) ||
                                    void 0 === _Fixed$parameters2$do
                                    ? void 0
                                    : _Fixed$parameters2$do.source
                            ),
                        }
                    ),
                }
            )),
                (Responsive.parameters = (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                    (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                        {},
                        Responsive.parameters
                    ),
                    {},
                    {
                        docs: (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                            (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                                {},
                                null === (_Responsive$parameter = Responsive.parameters) ||
                                    void 0 === _Responsive$parameter
                                    ? void 0
                                    : _Responsive$parameter.docs
                            ),
                            {},
                            {
                                source: (0,
                                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                                    {
                                        originalSource:
                                            '{\n  args: {\n    variant: "responsive",\n    sx: {\n      justifyContent: "center",\n      alignItems: "center",\n      display: "flex"\n    },\n    children: <>\n                <Paper sx={{\n        width: "10rem",\n        height: "10rem",\n        backgroundColor: `${theme.palette.customGrey.light} !important`\n      }} />\n            </>\n  }\n}',
                                    },
                                    null === (_Responsive$parameter2 = Responsive.parameters) ||
                                        void 0 === _Responsive$parameter2 ||
                                        null ===
                                            (_Responsive$parameter3 =
                                                _Responsive$parameter2.docs) ||
                                        void 0 === _Responsive$parameter3
                                        ? void 0
                                        : _Responsive$parameter3.source
                                ),
                            }
                        ),
                    }
                )),
                (Dashed.parameters = (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                    (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                        {},
                        Dashed.parameters
                    ),
                    {},
                    {
                        docs: (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                            (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                                {},
                                null === (_Dashed$parameters = Dashed.parameters) ||
                                    void 0 === _Dashed$parameters
                                    ? void 0
                                    : _Dashed$parameters.docs
                            ),
                            {},
                            {
                                source: (0,
                                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                                    {
                                        originalSource:
                                            '{\n  args: {\n    variant: "dashed",\n    sx: {\n      justifyContent: "center",\n      alignItems: "center",\n      display: "flex"\n    },\n    children: <>\n                <Paper sx={{\n        width: "10rem",\n        height: "10rem",\n        backgroundColor: `${theme.palette.customGrey.light} !important`\n      }} />\n            </>\n  }\n}',
                                    },
                                    null === (_Dashed$parameters2 = Dashed.parameters) ||
                                        void 0 === _Dashed$parameters2 ||
                                        null ===
                                            (_Dashed$parameters2$d = _Dashed$parameters2.docs) ||
                                        void 0 === _Dashed$parameters2$d
                                        ? void 0
                                        : _Dashed$parameters2$d.source
                                ),
                            }
                        ),
                    }
                ))
            var __namedExportsOrder = ["Fixed", "Responsive", "Dashed"]
        },
    },
])
