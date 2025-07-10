(self.webpackChunk_sequentech_ui_essentials =
    self.webpackChunk_sequentech_ui_essentials || []).push([
    [5051, 1677],
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
        "./src/components/CustomAutocompleteArrayInput/__stories__/CustomAutocompleteArrayInput.mdx": function (
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
                _CustomAutocompleteArrayInput_stories__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(
                    "./src/components/CustomAutocompleteArrayInput/__stories__/CustomAutocompleteArrayInput.stories.tsx"
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
                                    of: _CustomAutocompleteArrayInput_stories__WEBPACK_IMPORTED_MODULE_2__,
                                    title: "components/CustomAutocompleteArrayInput",
                                }
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _components.h1,
                                {
                                    id: "customautocompletearrayinput",
                                    children: "CustomAutocompleteArrayInput",
                                }
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(_components.p, {
                                children:
                                    "This custom component CustomAutocompleteArrayInput uses Material UI's Autocomplete and TextField to provide the same functionality as AutocompleteArrayInput from react-admin, but it ensures that the input field does not lose focus after entering a new value.",
                            }),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(_components.p, {
                                children:
                                    "A string with space separated words will be bulk converted to labels.",
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
                                        _CustomAutocompleteArrayInput_stories__WEBPACK_IMPORTED_MODULE_2__.Default,
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
        "./src/components/CustomAutocompleteArrayInput/__stories__/CustomAutocompleteArrayInput.stories.tsx": function (
            __unused_webpack_module,
            __webpack_exports__,
            __webpack_require__
        ) {
            __webpack_require__.r(__webpack_exports__),
                __webpack_require__.d(__webpack_exports__, {
                    Choices: function () {
                        return Choices
                    },
                    Default: function () {
                        return Default
                    },
                    Disabled: function () {
                        return Disabled
                    },
                    __namedExportsOrder: function () {
                        return __namedExportsOrder
                    },
                })
            var _Default$parameters,
                _Default$parameters2,
                _Default$parameters2$,
                _Disabled$parameters,
                _Disabled$parameters2,
                _Disabled$parameters3,
                _Choices$parameters,
                _Choices$parameters2,
                _Choices$parameters2$,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__ = __webpack_require__(
                    "../node_modules/@babel/runtime/helpers/esm/objectSpread2.js"
                ),
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectDestructuringEmpty_js__WEBPACK_IMPORTED_MODULE_4__ = __webpack_require__(
                    "../node_modules/@babel/runtime/helpers/esm/objectDestructuringEmpty.js"
                ),
                _CustomAutocompleteArrayInput__WEBPACK_IMPORTED_MODULE_1__ =
                    (__webpack_require__("../node_modules/react/index.js"),
                    __webpack_require__(
                        "./src/components/CustomAutocompleteArrayInput/CustomAutocompleteArrayInput.tsx"
                    )),
                storybook_addon_react_router_v6__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(
                    "./node_modules/storybook-addon-react-router-v6/dist/index.mjs"
                ),
                react_jsx_runtime__WEBPACK_IMPORTED_MODULE_3__ = __webpack_require__(
                    "../node_modules/react/jsx-runtime.js"
                ),
                meta = {
                    title: "components/CustomAutocompleteArrayInput",
                    component: _CustomAutocompleteArrayInput__WEBPACK_IMPORTED_MODULE_1__.Z,
                    decorators: [storybook_addon_react_router_v6__WEBPACK_IMPORTED_MODULE_2__.E],
                }
            __webpack_exports__.default = meta
            var Template = function Template(_ref) {
                    var args = Object.assign(
                        {},
                        ((0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectDestructuringEmpty_js__WEBPACK_IMPORTED_MODULE_4__.Z)(
                            _ref
                        ),
                        _ref)
                    )
                    return (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_3__.jsx)(
                        _CustomAutocompleteArrayInput__WEBPACK_IMPORTED_MODULE_1__.Z,
                        (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                            {},
                            args
                        )
                    )
                },
                Default = Template.bind({})
            Default.args = {label: "Name of the field", choices: []}
            var Disabled = Template.bind({})
            Disabled.args = {label: "Name of the field", choices: [], disabled: !0}
            var Choices = Template.bind({})
            ;(Choices.args = {
                label: "Name of the field",
                choices: [
                    {id: "uno", name: "uno"},
                    {id: "dos", name: "dos"},
                    {id: "tres", name: "tres"},
                ],
                defaultValue: ["uno", "dos", "tres"],
            }),
                (Default.parameters = (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                    (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                        {},
                        Default.parameters
                    ),
                    {},
                    {
                        docs: (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                            (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                                {},
                                null === (_Default$parameters = Default.parameters) ||
                                    void 0 === _Default$parameters
                                    ? void 0
                                    : _Default$parameters.docs
                            ),
                            {},
                            {
                                source: (0,
                                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                                    {
                                        originalSource:
                                            "({\n  ...args\n}) => <CustomAutocompleteArrayInput {...args} />",
                                    },
                                    null === (_Default$parameters2 = Default.parameters) ||
                                        void 0 === _Default$parameters2 ||
                                        null ===
                                            (_Default$parameters2$ = _Default$parameters2.docs) ||
                                        void 0 === _Default$parameters2$
                                        ? void 0
                                        : _Default$parameters2$.source
                                ),
                            }
                        ),
                    }
                )),
                (Disabled.parameters = (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                    (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                        {},
                        Disabled.parameters
                    ),
                    {},
                    {
                        docs: (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                            (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                                {},
                                null === (_Disabled$parameters = Disabled.parameters) ||
                                    void 0 === _Disabled$parameters
                                    ? void 0
                                    : _Disabled$parameters.docs
                            ),
                            {},
                            {
                                source: (0,
                                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                                    {
                                        originalSource:
                                            "({\n  ...args\n}) => <CustomAutocompleteArrayInput {...args} />",
                                    },
                                    null === (_Disabled$parameters2 = Disabled.parameters) ||
                                        void 0 === _Disabled$parameters2 ||
                                        null ===
                                            (_Disabled$parameters3 = _Disabled$parameters2.docs) ||
                                        void 0 === _Disabled$parameters3
                                        ? void 0
                                        : _Disabled$parameters3.source
                                ),
                            }
                        ),
                    }
                )),
                (Choices.parameters = (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                    (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                        {},
                        Choices.parameters
                    ),
                    {},
                    {
                        docs: (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                            (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                                {},
                                null === (_Choices$parameters = Choices.parameters) ||
                                    void 0 === _Choices$parameters
                                    ? void 0
                                    : _Choices$parameters.docs
                            ),
                            {},
                            {
                                source: (0,
                                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                                    {
                                        originalSource:
                                            "({\n  ...args\n}) => <CustomAutocompleteArrayInput {...args} />",
                                    },
                                    null === (_Choices$parameters2 = Choices.parameters) ||
                                        void 0 === _Choices$parameters2 ||
                                        null ===
                                            (_Choices$parameters2$ = _Choices$parameters2.docs) ||
                                        void 0 === _Choices$parameters2$
                                        ? void 0
                                        : _Choices$parameters2$.source
                                ),
                            }
                        ),
                    }
                ))
            var __namedExportsOrder = ["Default", "Disabled", "Choices"]
        },
    },
])
