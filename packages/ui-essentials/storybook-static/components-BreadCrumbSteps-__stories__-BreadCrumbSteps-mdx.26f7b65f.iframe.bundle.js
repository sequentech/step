;(self.webpackChunk_sequentech_ui_essentials =
    self.webpackChunk_sequentech_ui_essentials || []).push([
    [3359, 1074],
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
        "./src/components/BreadCrumbSteps/__stories__/BreadCrumbSteps.mdx": function (
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
                _BreadCrumbSteps_stories__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(
                    "./src/components/BreadCrumbSteps/__stories__/BreadCrumbSteps.stories.tsx"
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
                                    of: _BreadCrumbSteps_stories__WEBPACK_IMPORTED_MODULE_2__,
                                    title: "components/BreadCrumbSteps",
                                }
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _components.h1,
                                {id: "breadcrumbsteps", children: "BreadCrumbSteps"}
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(_components.p, {
                                children:
                                    "A BreadCrumbSteps is the box at the top of every screen. It includes the logo, the\nsoftware version, language toggle and an optional button to log out.",
                            }),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _components.h2,
                                {id: "primary", children: "Primary"}
                            ),
                            "\n",
                            (0,
                            react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _storybook_blocks__WEBPACK_IMPORTED_MODULE_4__.Xz,
                                {of: _BreadCrumbSteps_stories__WEBPACK_IMPORTED_MODULE_2__.Primary}
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _components.h2,
                                {id: "warning", children: "Warning"}
                            ),
                            "\n",
                            (0,
                            react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _storybook_blocks__WEBPACK_IMPORTED_MODULE_4__.Xz,
                                {of: _BreadCrumbSteps_stories__WEBPACK_IMPORTED_MODULE_2__.Warning}
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _components.h2,
                                {id: "circle", children: "Circle"}
                            ),
                            "\n",
                            (0,
                            react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _storybook_blocks__WEBPACK_IMPORTED_MODULE_4__.Xz,
                                {of: _BreadCrumbSteps_stories__WEBPACK_IMPORTED_MODULE_2__.Circle}
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _components.h2,
                                {id: "color-previous-steps", children: "Color previous steps"}
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _storybook_blocks__WEBPACK_IMPORTED_MODULE_4__.Xz,
                                {
                                    of:
                                        _BreadCrumbSteps_stories__WEBPACK_IMPORTED_MODULE_2__.CircleColorPreviousSteps,
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
        "./src/components/BreadCrumbSteps/__stories__/BreadCrumbSteps.stories.tsx": function (
            __unused_webpack_module,
            __webpack_exports__,
            __webpack_require__
        ) {
            __webpack_require__.r(__webpack_exports__),
                __webpack_require__.d(__webpack_exports__, {
                    Circle: function () {
                        return Circle
                    },
                    CircleColorPreviousSteps: function () {
                        return CircleColorPreviousSteps
                    },
                    Primary: function () {
                        return Primary
                    },
                    Warning: function () {
                        return Warning
                    },
                    __namedExportsOrder: function () {
                        return __namedExportsOrder
                    },
                })
            var _Primary$parameters,
                _Primary$parameters2,
                _Primary$parameters2$,
                _Warning$parameters,
                _Warning$parameters2,
                _Warning$parameters2$,
                _Circle$parameters,
                _Circle$parameters2,
                _Circle$parameters2$d,
                _CircleColorPreviousS,
                _CircleColorPreviousS2,
                _CircleColorPreviousS3,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(
                    "../node_modules/@babel/runtime/helpers/esm/objectSpread2.js"
                ),
                _BreadCrumbSteps__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(
                    "./src/components/BreadCrumbSteps/BreadCrumbSteps.tsx"
                ),
                _storybook_addon_viewport__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(
                    "../node_modules/@storybook/addon-viewport/dist/index.mjs"
                ),
                meta = {
                    title: "components/BreadCrumbSteps",
                    component: _BreadCrumbSteps__WEBPACK_IMPORTED_MODULE_0__.Z,
                    parameters: {
                        backgrounds: {default: "white"},
                        viewport: {
                            viewports: _storybook_addon_viewport__WEBPACK_IMPORTED_MODULE_1__.p,
                            defaultViewport: "iphone6",
                        },
                    },
                }
            __webpack_exports__.default = meta
            var parameters = {viewport: {disable: !0}},
                Primary = {
                    args: {
                        labels: [
                            "breadcrumbSteps.import",
                            "breadcrumbSteps.verify",
                            "breadcrumbSteps.finish",
                        ],
                        selected: 1,
                    },
                    parameters: parameters,
                },
                Warning = {
                    args: {
                        labels: [
                            "breadcrumbSteps.import",
                            "breadcrumbSteps.verify",
                            "breadcrumbSteps.finish",
                        ],
                        selected: 2,
                        warning: !0,
                    },
                    parameters: parameters,
                },
                Circle = {
                    args: {
                        labels: [
                            "breadcrumbSteps.import",
                            "breadcrumbSteps.verify",
                            "breadcrumbSteps.finish",
                        ],
                        selected: 1,
                        variant: _BreadCrumbSteps__WEBPACK_IMPORTED_MODULE_0__.g.Circle,
                    },
                    parameters: parameters,
                },
                CircleColorPreviousSteps = {
                    args: {
                        labels: [
                            "breadcrumbSteps.import",
                            "breadcrumbSteps.verify",
                            "breadcrumbSteps.finish",
                        ],
                        selected: 1,
                        variant: _BreadCrumbSteps__WEBPACK_IMPORTED_MODULE_0__.g.Circle,
                        colorPreviousSteps: !0,
                    },
                    parameters: parameters,
                }
            ;(Primary.parameters = (0,
            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_2__.Z)(
                (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_2__.Z)(
                    {},
                    Primary.parameters
                ),
                {},
                {
                    docs: (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_2__.Z)(
                        (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_2__.Z)(
                            {},
                            null === (_Primary$parameters = Primary.parameters) ||
                                void 0 === _Primary$parameters
                                ? void 0
                                : _Primary$parameters.docs
                        ),
                        {},
                        {
                            source: (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_2__.Z)(
                                {
                                    originalSource:
                                        '{\n  args: {\n    labels: ["breadcrumbSteps.import", "breadcrumbSteps.verify", "breadcrumbSteps.finish"],\n    selected: 1\n  },\n  parameters\n}',
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
            )),
                (Warning.parameters = (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_2__.Z)(
                    (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_2__.Z)(
                        {},
                        Warning.parameters
                    ),
                    {},
                    {
                        docs: (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_2__.Z)(
                            (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_2__.Z)(
                                {},
                                null === (_Warning$parameters = Warning.parameters) ||
                                    void 0 === _Warning$parameters
                                    ? void 0
                                    : _Warning$parameters.docs
                            ),
                            {},
                            {
                                source: (0,
                                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_2__.Z)(
                                    {
                                        originalSource:
                                            '{\n  args: {\n    labels: ["breadcrumbSteps.import", "breadcrumbSteps.verify", "breadcrumbSteps.finish"],\n    selected: 2,\n    warning: true\n  },\n  parameters\n}',
                                    },
                                    null === (_Warning$parameters2 = Warning.parameters) ||
                                        void 0 === _Warning$parameters2 ||
                                        null ===
                                            (_Warning$parameters2$ = _Warning$parameters2.docs) ||
                                        void 0 === _Warning$parameters2$
                                        ? void 0
                                        : _Warning$parameters2$.source
                                ),
                            }
                        ),
                    }
                )),
                (Circle.parameters = (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_2__.Z)(
                    (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_2__.Z)(
                        {},
                        Circle.parameters
                    ),
                    {},
                    {
                        docs: (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_2__.Z)(
                            (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_2__.Z)(
                                {},
                                null === (_Circle$parameters = Circle.parameters) ||
                                    void 0 === _Circle$parameters
                                    ? void 0
                                    : _Circle$parameters.docs
                            ),
                            {},
                            {
                                source: (0,
                                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_2__.Z)(
                                    {
                                        originalSource:
                                            '{\n  args: {\n    labels: ["breadcrumbSteps.import", "breadcrumbSteps.verify", "breadcrumbSteps.finish"],\n    selected: 1,\n    variant: BreadCrumbStepsVariant.Circle\n  },\n  parameters\n}',
                                    },
                                    null === (_Circle$parameters2 = Circle.parameters) ||
                                        void 0 === _Circle$parameters2 ||
                                        null ===
                                            (_Circle$parameters2$d = _Circle$parameters2.docs) ||
                                        void 0 === _Circle$parameters2$d
                                        ? void 0
                                        : _Circle$parameters2$d.source
                                ),
                            }
                        ),
                    }
                )),
                (CircleColorPreviousSteps.parameters = (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_2__.Z)(
                    (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_2__.Z)(
                        {},
                        CircleColorPreviousSteps.parameters
                    ),
                    {},
                    {
                        docs: (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_2__.Z)(
                            (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_2__.Z)(
                                {},
                                null ===
                                    (_CircleColorPreviousS = CircleColorPreviousSteps.parameters) ||
                                    void 0 === _CircleColorPreviousS
                                    ? void 0
                                    : _CircleColorPreviousS.docs
                            ),
                            {},
                            {
                                source: (0,
                                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_2__.Z)(
                                    {
                                        originalSource:
                                            '{\n  args: {\n    labels: ["breadcrumbSteps.import", "breadcrumbSteps.verify", "breadcrumbSteps.finish"],\n    selected: 1,\n    variant: BreadCrumbStepsVariant.Circle,\n    colorPreviousSteps: true\n  },\n  parameters\n}',
                                    },
                                    null ===
                                        (_CircleColorPreviousS2 =
                                            CircleColorPreviousSteps.parameters) ||
                                        void 0 === _CircleColorPreviousS2 ||
                                        null ===
                                            (_CircleColorPreviousS3 =
                                                _CircleColorPreviousS2.docs) ||
                                        void 0 === _CircleColorPreviousS3
                                        ? void 0
                                        : _CircleColorPreviousS3.source
                                ),
                            }
                        ),
                    }
                ))
            var __namedExportsOrder = ["Primary", "Warning", "Circle", "CircleColorPreviousSteps"]
        },
    },
])
