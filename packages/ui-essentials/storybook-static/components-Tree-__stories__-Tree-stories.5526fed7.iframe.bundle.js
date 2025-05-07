(self.webpackChunk_sequentech_ui_essentials =
    self.webpackChunk_sequentech_ui_essentials || []).push([
    [5659],
    {
        "./src/components/Tree/__stories__/Tree.stories.tsx": function (
            __unused_webpack_module,
            __webpack_exports__,
            __webpack_require__
        ) {
            __webpack_require__.r(__webpack_exports__),
                __webpack_require__.d(__webpack_exports__, {
                    SimpleTree: function () {
                        return SimpleTree
                    },
                    TreeComponents: function () {
                        return TreeComponents
                    },
                    __namedExportsOrder: function () {
                        return __namedExportsOrder
                    },
                })
            var _templateObject,
                _SimpleTree$parameter,
                _SimpleTree$parameter2,
                _SimpleTree$parameter3,
                _TreeComponents$param,
                _TreeComponents$param2,
                _TreeComponents$param3,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__ = __webpack_require__(
                    "../node_modules/@babel/runtime/helpers/esm/objectSpread2.js"
                ),
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_taggedTemplateLiteral_js__WEBPACK_IMPORTED_MODULE_6__ = __webpack_require__(
                    "../node_modules/@babel/runtime/helpers/esm/taggedTemplateLiteral.js"
                ),
                _Tree__WEBPACK_IMPORTED_MODULE_1__ =
                    (__webpack_require__("../node_modules/react/index.js"),
                    __webpack_require__("./src/components/Tree/Tree.tsx")),
                _storybook_addon_viewport__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(
                    "../node_modules/@storybook/addon-viewport/dist/index.mjs"
                ),
                _fortawesome_free_solid_svg_icons__WEBPACK_IMPORTED_MODULE_7__ = __webpack_require__(
                    "../node_modules/@fortawesome/free-solid-svg-icons/index.mjs"
                ),
                _Icon_Icon__WEBPACK_IMPORTED_MODULE_3__ = __webpack_require__(
                    "./src/components/Icon/Icon.tsx"
                ),
                _mui_material_styles__WEBPACK_IMPORTED_MODULE_5__ = __webpack_require__(
                    "../node_modules/@mui/material/styles/styled.js"
                ),
                react_jsx_runtime__WEBPACK_IMPORTED_MODULE_4__ = __webpack_require__(
                    "../node_modules/react/jsx-runtime.js"
                ),
                StyledIcon = (0, _mui_material_styles__WEBPACK_IMPORTED_MODULE_5__.ZP)(
                    _Icon_Icon__WEBPACK_IMPORTED_MODULE_3__.Z
                )(
                    _templateObject ||
                        (_templateObject = (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_taggedTemplateLiteral_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                            ["\n    margin-right: 8px;\n"]
                        ))
                ),
                meta = {
                    title: "components/Tree",
                    component: _Tree__WEBPACK_IMPORTED_MODULE_1__.Z,
                    parameters: {
                        backgrounds: {default: "white"},
                        viewport: {
                            viewports: _storybook_addon_viewport__WEBPACK_IMPORTED_MODULE_2__.p,
                            defaultViewport: "iphone6",
                        },
                    },
                }
            __webpack_exports__.default = meta
            var SimpleTree = {
                    args: {
                        leaves: [
                            {
                                label: "Parent",
                                leaves: [
                                    {
                                        label: "Child 1",
                                        leaves: [{label: "SubChild A"}, {label: "SubChild B"}],
                                    },
                                    {label: "Child 2"},
                                ],
                            },
                            {label: "Parent 2"},
                        ],
                    },
                    parameters: {backgrounds: {default: "white"}, viewport: {disable: !0}},
                },
                TreeComponents = {
                    args: {
                        leaves: [
                            {
                                label: (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_4__.jsxs)(
                                    react_jsx_runtime__WEBPACK_IMPORTED_MODULE_4__.Fragment,
                                    {
                                        children: [
                                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_4__.jsx)(
                                                StyledIcon,
                                                {
                                                    icon:
                                                        _fortawesome_free_solid_svg_icons__WEBPACK_IMPORTED_MODULE_7__.glO,
                                                }
                                            ),
                                            (0,
                                            react_jsx_runtime__WEBPACK_IMPORTED_MODULE_4__.jsx)(
                                                "i",
                                                {children: "Parent"}
                                            ),
                                        ],
                                    }
                                ),
                                leaves: [
                                    {
                                        label: "Child 1",
                                        leaves: [{label: "SubChild A"}, {label: "SubChild B"}],
                                    },
                                    {label: "Child 2"},
                                ],
                            },
                        ],
                    },
                    parameters: {backgrounds: {default: "white"}, viewport: {disable: !0}},
                }
            ;(SimpleTree.parameters = (0,
            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                    {},
                    SimpleTree.parameters
                ),
                {},
                {
                    docs: (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                        (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                            {},
                            null === (_SimpleTree$parameter = SimpleTree.parameters) ||
                                void 0 === _SimpleTree$parameter
                                ? void 0
                                : _SimpleTree$parameter.docs
                        ),
                        {},
                        {
                            source: (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                                {
                                    originalSource:
                                        '{\n  args: {\n    leaves: [{\n      label: "Parent",\n      leaves: [{\n        label: "Child 1",\n        leaves: [{\n          label: "SubChild A"\n        }, {\n          label: "SubChild B"\n        }]\n      }, {\n        label: "Child 2"\n      }]\n    }, {\n      label: "Parent 2"\n    }]\n  },\n  parameters: {\n    backgrounds: {\n      default: "white"\n    },\n    viewport: {\n      disable: true\n    }\n  }\n}',
                                },
                                null === (_SimpleTree$parameter2 = SimpleTree.parameters) ||
                                    void 0 === _SimpleTree$parameter2 ||
                                    null ===
                                        (_SimpleTree$parameter3 = _SimpleTree$parameter2.docs) ||
                                    void 0 === _SimpleTree$parameter3
                                    ? void 0
                                    : _SimpleTree$parameter3.source
                            ),
                        }
                    ),
                }
            )),
                (TreeComponents.parameters = (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                    (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                        {},
                        TreeComponents.parameters
                    ),
                    {},
                    {
                        docs: (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                            (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                                {},
                                null === (_TreeComponents$param = TreeComponents.parameters) ||
                                    void 0 === _TreeComponents$param
                                    ? void 0
                                    : _TreeComponents$param.docs
                            ),
                            {},
                            {
                                source: (0,
                                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                                    {
                                        originalSource:
                                            '{\n  args: {\n    leaves: [{\n      label: <>\n                        <StyledIcon icon={faBank} />\n                        <i>Parent</i>\n                    </>,\n      leaves: [{\n        label: "Child 1",\n        leaves: [{\n          label: "SubChild A"\n        }, {\n          label: "SubChild B"\n        }]\n      }, {\n        label: "Child 2"\n      }]\n    }]\n  },\n  parameters: {\n    backgrounds: {\n      default: "white"\n    },\n    viewport: {\n      disable: true\n    }\n  }\n}',
                                    },
                                    null === (_TreeComponents$param2 = TreeComponents.parameters) ||
                                        void 0 === _TreeComponents$param2 ||
                                        null ===
                                            (_TreeComponents$param3 =
                                                _TreeComponents$param2.docs) ||
                                        void 0 === _TreeComponents$param3
                                        ? void 0
                                        : _TreeComponents$param3.source
                                ),
                            }
                        ),
                    }
                ))
            var __namedExportsOrder = ["SimpleTree", "TreeComponents"]
        },
    },
])
