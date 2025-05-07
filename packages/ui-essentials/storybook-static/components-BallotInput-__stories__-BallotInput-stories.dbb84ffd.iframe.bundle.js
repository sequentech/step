;(self.webpackChunk_sequentech_ui_essentials =
    self.webpackChunk_sequentech_ui_essentials || []).push([
    [2381],
    {
        "./src/components/BallotInput/__stories__/BallotInput.stories.tsx": function (
            __unused_webpack_module,
            __webpack_exports__,
            __webpack_require__
        ) {
            __webpack_require__.r(__webpack_exports__),
                __webpack_require__.d(__webpack_exports__, {
                    NoBack: function () {
                        return NoBack
                    },
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
                _NoBack$parameters,
                _NoBack$parameters2,
                _NoBack$parameters2$d,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__ = __webpack_require__(
                    "../node_modules/@babel/runtime/helpers/esm/objectSpread2.js"
                ),
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_slicedToArray_js__WEBPACK_IMPORTED_MODULE_4__ = __webpack_require__(
                    "../node_modules/@babel/runtime/helpers/esm/slicedToArray.js"
                ),
                react__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(
                    "../node_modules/react/index.js"
                ),
                _BallotInput__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(
                    "./src/components/BallotInput/BallotInput.tsx"
                ),
                _storybook_addon_viewport__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(
                    "../node_modules/@storybook/addon-viewport/dist/index.mjs"
                ),
                react_router_dom__WEBPACK_IMPORTED_MODULE_5__ = __webpack_require__(
                    "../node_modules/react-router-dom/node_modules/react-router/dist/index.js"
                ),
                react_jsx_runtime__WEBPACK_IMPORTED_MODULE_3__ = __webpack_require__(
                    "../node_modules/react/jsx-runtime.js"
                ),
                meta = {
                    title: "components/BallotInput",
                    component: _BallotInput__WEBPACK_IMPORTED_MODULE_1__.Z,
                    parameters: {
                        backgrounds: {default: "white"},
                        viewport: {
                            viewports: _storybook_addon_viewport__WEBPACK_IMPORTED_MODULE_2__.p,
                            defaultViewport: "iphone6",
                        },
                    },
                }
            __webpack_exports__.default = meta
            var BallotExample = function BallotExample(args) {
                    var _useState = (0, react__WEBPACK_IMPORTED_MODULE_0__.useState)(""),
                        _useState2 = (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_slicedToArray_js__WEBPACK_IMPORTED_MODULE_4__.Z)(
                            _useState,
                            2
                        ),
                        value = _useState2[0],
                        setValue = _useState2[1]
                    return (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_3__.jsx)(
                        react_router_dom__WEBPACK_IMPORTED_MODULE_5__.VA,
                        {
                            initialEntries: ["/tenant/1/event/2/election-chooser"],
                            children: (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_3__.jsx)(
                                _BallotInput__WEBPACK_IMPORTED_MODULE_1__.Z,
                                (0,
                                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                                    {
                                        value: value,
                                        doChange: function doChange(e) {
                                            return setValue(e.target.value)
                                        },
                                    },
                                    args
                                )
                            ),
                        }
                    )
                },
                Primary = {
                    render: function render(args) {
                        return (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_3__.jsx)(
                            BallotExample,
                            (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                                {},
                                args
                            )
                        )
                    },
                    args: {
                        title: "Ballot ID",
                        subTitle: "Enter your ballot identifier",
                        label: "Ballot ID",
                        error: "Invalid ballot ID",
                        placeholder: "e.g. 1234abcd",
                        captureEnterAction: function captureEnterAction(e) {
                            "Enter" === e.key && alert("Enter pressed!")
                        },
                        labelProps: {shrink: !0},
                        helpText: "Your ballot ID is provided on your voting card.",
                        dialogTitle: "What is a Ballot ID?",
                        dialogOk: "OK",
                        backButtonText: "Back",
                        ballotStyle: void 0,
                    },
                    parameters: {viewport: {disable: !0}},
                },
                NoBack = {
                    render: function render(args) {
                        return (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_3__.jsx)(
                            BallotExample,
                            (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                                {},
                                args
                            )
                        )
                    },
                    args: {
                        title: "Ballot ID",
                        subTitle: "Enter your ballot identifier",
                        label: "Ballot ID",
                        error: "Invalid ballot ID",
                        placeholder: "e.g. 1234abcd",
                        captureEnterAction: function captureEnterAction(e) {
                            "Enter" === e.key && alert("Enter pressed!")
                        },
                        labelProps: {shrink: !0},
                        helpText: "Your ballot ID is provided on your voting card.",
                        dialogTitle: "What is a Ballot ID?",
                        dialogOk: "OK",
                        ballotStyle: void 0,
                    },
                    parameters: {viewport: {disable: !0}},
                }
            ;(Primary.parameters = (0,
            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                    {},
                    Primary.parameters
                ),
                {},
                {
                    docs: (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                        (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                            {},
                            null === (_Primary$parameters = Primary.parameters) ||
                                void 0 === _Primary$parameters
                                ? void 0
                                : _Primary$parameters.docs
                        ),
                        {},
                        {
                            source: (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                                {
                                    originalSource:
                                        '{\n  render: args => <BallotExample {...args} />,\n  args: {\n    title: "Ballot ID",\n    subTitle: "Enter your ballot identifier",\n    label: "Ballot ID",\n    error: "Invalid ballot ID",\n    placeholder: "e.g. 1234abcd",\n    captureEnterAction: e => {\n      if (e.key === "Enter") alert("Enter pressed!");\n    },\n    labelProps: {\n      shrink: true\n    },\n    helpText: "Your ballot ID is provided on your voting card.",\n    dialogTitle: "What is a Ballot ID?",\n    dialogOk: "OK",\n    backButtonText: "Back",\n    ballotStyle: undefined\n  },\n  parameters: {\n    viewport: {\n      disable: true\n    }\n  }\n}',
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
                (NoBack.parameters = (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                    (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                        {},
                        NoBack.parameters
                    ),
                    {},
                    {
                        docs: (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                            (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                                {},
                                null === (_NoBack$parameters = NoBack.parameters) ||
                                    void 0 === _NoBack$parameters
                                    ? void 0
                                    : _NoBack$parameters.docs
                            ),
                            {},
                            {
                                source: (0,
                                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                                    {
                                        originalSource:
                                            '{\n  render: args => <BallotExample {...args} />,\n  args: {\n    title: "Ballot ID",\n    subTitle: "Enter your ballot identifier",\n    label: "Ballot ID",\n    error: "Invalid ballot ID",\n    placeholder: "e.g. 1234abcd",\n    captureEnterAction: e => {\n      if (e.key === "Enter") alert("Enter pressed!");\n    },\n    labelProps: {\n      shrink: true\n    },\n    helpText: "Your ballot ID is provided on your voting card.",\n    dialogTitle: "What is a Ballot ID?",\n    dialogOk: "OK",\n    // backButtonText: "Back",\n    ballotStyle: undefined\n  },\n  parameters: {\n    viewport: {\n      disable: true\n    }\n  }\n}',
                                    },
                                    null === (_NoBack$parameters2 = NoBack.parameters) ||
                                        void 0 === _NoBack$parameters2 ||
                                        null ===
                                            (_NoBack$parameters2$d = _NoBack$parameters2.docs) ||
                                        void 0 === _NoBack$parameters2$d
                                        ? void 0
                                        : _NoBack$parameters2$d.source
                                ),
                            }
                        ),
                    }
                ))
            var __namedExportsOrder = ["Primary", "NoBack"]
        },
    },
])
