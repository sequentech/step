;(self.webpackChunk_sequentech_ui_essentials =
    self.webpackChunk_sequentech_ui_essentials || []).push([
    [9217],
    {
        "./src/components/QRCode/__stories__/QRCode.stories.tsx": function (
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
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(
                    "../node_modules/@babel/runtime/helpers/esm/objectSpread2.js"
                ),
                _QRCode__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(
                    "./src/components/QRCode/QRCode.tsx"
                ),
                _storybook_addon_viewport__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(
                    "../node_modules/@storybook/addon-viewport/dist/index.mjs"
                ),
                meta = {
                    title: "components/QRCode",
                    component: _QRCode__WEBPACK_IMPORTED_MODULE_0__.Z,
                    parameters: {
                        backgrounds: {default: "white"},
                        viewport: {
                            viewports: _storybook_addon_viewport__WEBPACK_IMPORTED_MODULE_1__.p,
                            defaultViewport: "iphone6",
                        },
                    },
                }
            __webpack_exports__.default = meta
            var Primary = {
                args: {value: "https://sequentech.io"},
                parameters: {viewport: {disable: !0}},
            }
            Primary.parameters = (0,
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
                                        '{\n  args: {\n    value: "https://sequentech.io"\n  },\n  parameters: {\n    viewport: {\n      disable: true\n    }\n  }\n}',
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
