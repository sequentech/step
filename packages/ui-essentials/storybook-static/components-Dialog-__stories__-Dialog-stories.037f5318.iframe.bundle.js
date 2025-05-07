;(self.webpackChunk_sequentech_ui_essentials =
    self.webpackChunk_sequentech_ui_essentials || []).push([
    [4792],
    {
        "./src/components/Dialog/__stories__/Dialog.stories.tsx": function (
            __unused_webpack_module,
            __webpack_exports__,
            __webpack_require__
        ) {
            __webpack_require__.r(__webpack_exports__),
                __webpack_require__.d(__webpack_exports__, {
                    Action: function () {
                        return Action
                    },
                    ActionMobile: function () {
                        return ActionMobile
                    },
                    Info: function () {
                        return Info
                    },
                    InfoMobile: function () {
                        return InfoMobile
                    },
                    Warning: function () {
                        return Warning
                    },
                    WarningMobile: function () {
                        return WarningMobile
                    },
                    __namedExportsOrder: function () {
                        return __namedExportsOrder
                    },
                })
            var _Info$parameters,
                _Info$parameters2,
                _Info$parameters2$doc,
                _InfoMobile$parameter,
                _InfoMobile$parameter2,
                _InfoMobile$parameter3,
                _Warning$parameters,
                _Warning$parameters2,
                _Warning$parameters2$,
                _WarningMobile$parame,
                _WarningMobile$parame2,
                _WarningMobile$parame3,
                _Action$parameters,
                _Action$parameters2,
                _Action$parameters2$d,
                _ActionMobile$paramet,
                _ActionMobile$paramet2,
                _ActionMobile$paramet3,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_7__ = __webpack_require__(
                    "../node_modules/@babel/runtime/helpers/esm/objectSpread2.js"
                ),
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_slicedToArray_js__WEBPACK_IMPORTED_MODULE_5__ = __webpack_require__(
                    "../node_modules/@babel/runtime/helpers/esm/slicedToArray.js"
                ),
                react__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(
                    "../node_modules/react/index.js"
                ),
                _Dialog__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(
                    "./src/components/Dialog/Dialog.tsx"
                ),
                _storybook_addon_viewport__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(
                    "../node_modules/@storybook/addon-viewport/dist/index.mjs"
                ),
                _mui_material__WEBPACK_IMPORTED_MODULE_6__ = __webpack_require__(
                    "../node_modules/@mui/material/Button/Button.js"
                ),
                react_i18next__WEBPACK_IMPORTED_MODULE_4__ = __webpack_require__(
                    "../node_modules/react-i18next/dist/es/useTranslation.js"
                ),
                react_jsx_runtime__WEBPACK_IMPORTED_MODULE_3__ = __webpack_require__(
                    "../node_modules/react/jsx-runtime.js"
                ),
                meta = {
                    title: "components/Dialog",
                    component: function DialogExample(_ref) {
                        var variant = _ref.variant,
                            close = _ref.close,
                            t = (0, react_i18next__WEBPACK_IMPORTED_MODULE_4__.$)().t,
                            _useState = (0, react__WEBPACK_IMPORTED_MODULE_0__.useState)(!0),
                            _useState2 = (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_slicedToArray_js__WEBPACK_IMPORTED_MODULE_5__.Z)(
                                _useState,
                                2
                            ),
                            open = _useState2[0],
                            setOpen = _useState2[1]
                        return (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_3__.jsxs)(
                            react_jsx_runtime__WEBPACK_IMPORTED_MODULE_3__.Fragment,
                            {
                                children: [
                                    (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_3__.jsx)(
                                        _mui_material__WEBPACK_IMPORTED_MODULE_6__.Z,
                                        {
                                            onClick: function onClick() {
                                                return setOpen(!0)
                                            },
                                            children: t("stories.openDialog"),
                                        }
                                    ),
                                    (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_3__.jsx)(
                                        _Dialog__WEBPACK_IMPORTED_MODULE_1__.Z,
                                        {
                                            handleClose: function handleClose() {
                                                return setOpen(!1)
                                            },
                                            open: open,
                                            title: t("ballotSelectionsScreen.statusModal.title"),
                                            ok: t("ballotSelectionsScreen.statusModal.ok"),
                                            cancel: close ? t("logout.modal.close") : void 0,
                                            variant: variant,
                                            children: (0,
                                            react_jsx_runtime__WEBPACK_IMPORTED_MODULE_3__.jsx)(
                                                "p",
                                                {
                                                    children: t(
                                                        "ballotSelectionsScreen.statusModal.content"
                                                    ),
                                                }
                                            ),
                                        }
                                    ),
                                ],
                            }
                        )
                    },
                    parameters: {
                        backgrounds: {default: "white"},
                        viewport: {
                            viewports: _storybook_addon_viewport__WEBPACK_IMPORTED_MODULE_2__.p,
                            defaultViewport: "iphone6",
                        },
                    },
                }
            __webpack_exports__.default = meta
            var Info = {args: {variant: "info"}, parameters: {viewport: {disable: !0}}},
                InfoMobile = {
                    args: {variant: "info"},
                    parameters: {viewport: {defaultViewport: "iphone6"}},
                },
                Warning = {
                    args: {variant: "warning", close: !0},
                    parameters: {viewport: {disable: !0}},
                },
                WarningMobile = {
                    args: {variant: "warning", close: !0},
                    parameters: {viewport: {defaultViewport: "iphone6"}},
                },
                Action = {
                    args: {variant: "action", close: !0},
                    parameters: {viewport: {disable: !0}},
                },
                ActionMobile = {
                    args: {variant: "action", close: !0},
                    parameters: {viewport: {defaultViewport: "iphone6"}},
                }
            ;(Info.parameters = (0,
            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_7__.Z)(
                (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_7__.Z)(
                    {},
                    Info.parameters
                ),
                {},
                {
                    docs: (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_7__.Z)(
                        (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_7__.Z)(
                            {},
                            null === (_Info$parameters = Info.parameters) ||
                                void 0 === _Info$parameters
                                ? void 0
                                : _Info$parameters.docs
                        ),
                        {},
                        {
                            source: (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_7__.Z)(
                                {
                                    originalSource:
                                        '{\n  // More on args: https://storybook.js.org/docs/react/writing-stories/args\n  args: {\n    variant: "info"\n  },\n  parameters: {\n    viewport: {\n      disable: true\n    }\n  }\n}',
                                },
                                null === (_Info$parameters2 = Info.parameters) ||
                                    void 0 === _Info$parameters2 ||
                                    null === (_Info$parameters2$doc = _Info$parameters2.docs) ||
                                    void 0 === _Info$parameters2$doc
                                    ? void 0
                                    : _Info$parameters2$doc.source
                            ),
                        }
                    ),
                }
            )),
                (InfoMobile.parameters = (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_7__.Z)(
                    (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_7__.Z)(
                        {},
                        InfoMobile.parameters
                    ),
                    {},
                    {
                        docs: (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_7__.Z)(
                            (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_7__.Z)(
                                {},
                                null === (_InfoMobile$parameter = InfoMobile.parameters) ||
                                    void 0 === _InfoMobile$parameter
                                    ? void 0
                                    : _InfoMobile$parameter.docs
                            ),
                            {},
                            {
                                source: (0,
                                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_7__.Z)(
                                    {
                                        originalSource:
                                            '{\n  // More on args: https://storybook.js.org/docs/react/writing-stories/args\n  args: {\n    variant: "info"\n  },\n  parameters: {\n    viewport: {\n      defaultViewport: "iphone6"\n    }\n  }\n}',
                                    },
                                    null === (_InfoMobile$parameter2 = InfoMobile.parameters) ||
                                        void 0 === _InfoMobile$parameter2 ||
                                        null ===
                                            (_InfoMobile$parameter3 =
                                                _InfoMobile$parameter2.docs) ||
                                        void 0 === _InfoMobile$parameter3
                                        ? void 0
                                        : _InfoMobile$parameter3.source
                                ),
                            }
                        ),
                    }
                )),
                (Warning.parameters = (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_7__.Z)(
                    (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_7__.Z)(
                        {},
                        Warning.parameters
                    ),
                    {},
                    {
                        docs: (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_7__.Z)(
                            (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_7__.Z)(
                                {},
                                null === (_Warning$parameters = Warning.parameters) ||
                                    void 0 === _Warning$parameters
                                    ? void 0
                                    : _Warning$parameters.docs
                            ),
                            {},
                            {
                                source: (0,
                                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_7__.Z)(
                                    {
                                        originalSource:
                                            '{\n  // More on args: https://storybook.js.org/docs/react/writing-stories/args\n  args: {\n    variant: "warning",\n    close: true\n  },\n  parameters: {\n    viewport: {\n      disable: true\n    }\n  }\n}',
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
                (WarningMobile.parameters = (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_7__.Z)(
                    (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_7__.Z)(
                        {},
                        WarningMobile.parameters
                    ),
                    {},
                    {
                        docs: (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_7__.Z)(
                            (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_7__.Z)(
                                {},
                                null === (_WarningMobile$parame = WarningMobile.parameters) ||
                                    void 0 === _WarningMobile$parame
                                    ? void 0
                                    : _WarningMobile$parame.docs
                            ),
                            {},
                            {
                                source: (0,
                                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_7__.Z)(
                                    {
                                        originalSource:
                                            '{\n  // More on args: https://storybook.js.org/docs/react/writing-stories/args\n  args: {\n    variant: "warning",\n    close: true\n  },\n  parameters: {\n    viewport: {\n      defaultViewport: "iphone6"\n    }\n  }\n}',
                                    },
                                    null === (_WarningMobile$parame2 = WarningMobile.parameters) ||
                                        void 0 === _WarningMobile$parame2 ||
                                        null ===
                                            (_WarningMobile$parame3 =
                                                _WarningMobile$parame2.docs) ||
                                        void 0 === _WarningMobile$parame3
                                        ? void 0
                                        : _WarningMobile$parame3.source
                                ),
                            }
                        ),
                    }
                )),
                (Action.parameters = (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_7__.Z)(
                    (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_7__.Z)(
                        {},
                        Action.parameters
                    ),
                    {},
                    {
                        docs: (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_7__.Z)(
                            (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_7__.Z)(
                                {},
                                null === (_Action$parameters = Action.parameters) ||
                                    void 0 === _Action$parameters
                                    ? void 0
                                    : _Action$parameters.docs
                            ),
                            {},
                            {
                                source: (0,
                                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_7__.Z)(
                                    {
                                        originalSource:
                                            '{\n  // More on args: https://storybook.js.org/docs/react/writing-stories/args\n  args: {\n    variant: "action",\n    close: true\n  },\n  parameters: {\n    viewport: {\n      disable: true\n    }\n  }\n}',
                                    },
                                    null === (_Action$parameters2 = Action.parameters) ||
                                        void 0 === _Action$parameters2 ||
                                        null ===
                                            (_Action$parameters2$d = _Action$parameters2.docs) ||
                                        void 0 === _Action$parameters2$d
                                        ? void 0
                                        : _Action$parameters2$d.source
                                ),
                            }
                        ),
                    }
                )),
                (ActionMobile.parameters = (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_7__.Z)(
                    (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_7__.Z)(
                        {},
                        ActionMobile.parameters
                    ),
                    {},
                    {
                        docs: (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_7__.Z)(
                            (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_7__.Z)(
                                {},
                                null === (_ActionMobile$paramet = ActionMobile.parameters) ||
                                    void 0 === _ActionMobile$paramet
                                    ? void 0
                                    : _ActionMobile$paramet.docs
                            ),
                            {},
                            {
                                source: (0,
                                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_7__.Z)(
                                    {
                                        originalSource:
                                            '{\n  // More on args: https://storybook.js.org/docs/react/writing-stories/args\n  args: {\n    variant: "action",\n    close: true\n  },\n  parameters: {\n    viewport: {\n      defaultViewport: "iphone6"\n    }\n  }\n}',
                                    },
                                    null === (_ActionMobile$paramet2 = ActionMobile.parameters) ||
                                        void 0 === _ActionMobile$paramet2 ||
                                        null ===
                                            (_ActionMobile$paramet3 =
                                                _ActionMobile$paramet2.docs) ||
                                        void 0 === _ActionMobile$paramet3
                                        ? void 0
                                        : _ActionMobile$paramet3.source
                                ),
                            }
                        ),
                    }
                ))
            var __namedExportsOrder = [
                "Info",
                "InfoMobile",
                "Warning",
                "WarningMobile",
                "Action",
                "ActionMobile",
            ]
        },
    },
])
