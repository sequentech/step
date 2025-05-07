;(self.webpackChunk_sequentech_ui_essentials =
    self.webpackChunk_sequentech_ui_essentials || []).push([
    [1714],
    {
        "./src/components/ProfileMenu/__stories__/ProfileMenu.stories.tsx": function (
            __unused_webpack_module,
            __webpack_exports__,
            __webpack_require__
        ) {
            __webpack_require__.r(__webpack_exports__),
                __webpack_require__.d(__webpack_exports__, {
                    CountdownOnly: function () {
                        return CountdownOnly
                    },
                    CountdownWithAlert: function () {
                        return CountdownWithAlert
                    },
                    NoCountdown: function () {
                        return NoCountdown
                    },
                    __namedExportsOrder: function () {
                        return __namedExportsOrder
                    },
                })
            var _CountdownWithAlert$p,
                _CountdownWithAlert$p2,
                _CountdownWithAlert$p3,
                _CountdownOnly$parame,
                _CountdownOnly$parame2,
                _CountdownOnly$parame3,
                _NoCountdown$paramete,
                _NoCountdown$paramete2,
                _NoCountdown$paramete3,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__ = __webpack_require__(
                    "../node_modules/@babel/runtime/helpers/esm/objectSpread2.js"
                ),
                _storybook_addon_viewport__WEBPACK_IMPORTED_MODULE_1__ =
                    (__webpack_require__("../node_modules/react/index.js"),
                    __webpack_require__(
                        "../node_modules/@storybook/addon-viewport/dist/index.mjs"
                    )),
                _ProfileMenu__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(
                    "./src/components/ProfileMenu/ProfileMenu.tsx"
                ),
                _mui_material__WEBPACK_IMPORTED_MODULE_5__ = __webpack_require__(
                    "../node_modules/@mui/material/Box/Box.js"
                ),
                _sequentech_ui_core__WEBPACK_IMPORTED_MODULE_3__ = __webpack_require__(
                    "../ui-core/dist/index.js"
                ),
                react_jsx_runtime__WEBPACK_IMPORTED_MODULE_4__ = __webpack_require__(
                    "../node_modules/react/jsx-runtime.js"
                ),
                meta = {
                    title: "components/ProfileMenu",
                    component: _ProfileMenu__WEBPACK_IMPORTED_MODULE_2__.D,
                    decorators: [
                        function (Story) {
                            return (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_4__.jsx)(
                                _mui_material__WEBPACK_IMPORTED_MODULE_5__.Z,
                                {
                                    sx: {
                                        width: "100%",
                                        flexDirection: "row",
                                        justifyContent: "flex-end",
                                        display: "flex",
                                        alignItems: "flex-end",
                                    },
                                    children: (0,
                                    react_jsx_runtime__WEBPACK_IMPORTED_MODULE_4__.jsx)(Story, {}),
                                }
                            )
                        },
                    ],
                    parameters: {
                        backgrounds: {default: "white"},
                        viewport: {
                            viewports: _storybook_addon_viewport__WEBPACK_IMPORTED_MODULE_1__.p,
                            defaultViewport: "iphone6",
                        },
                    },
                }
            __webpack_exports__.default = meta
            var CountdownWithAlert = {
                    args: {
                        userProfile: {
                            email:
                                "johnhasaverysupersuperduperverysuperduperlongname@sequentech.io",
                            username:
                                "John has a very super super duper very super duper long name",
                            openLink: function openLink() {
                                alert("rouge")
                            },
                        },
                        logoutFn: function logoutFn() {
                            alert("logging out")
                        },
                        setOpenModal: function setOpenModal() {
                            return alert("open log out modal")
                        },
                        handleOpenTimeModal: function handleOpenTimeModal() {
                            return alert("open time modal")
                        },
                        expiry: {
                            endTime: new Date(Date.now() + 6e4),
                            countdown:
                                _sequentech_ui_core__WEBPACK_IMPORTED_MODULE_3__
                                    .EVotingPortalCountdownPolicy.COUNTDOWN_WITH_ALERT,
                            countdownAt: 60,
                            alertAt: 30,
                        },
                        setTimeLeftDialogText: function setTimeLeftDialogText(v) {
                            return console.log({v: v})
                        },
                    },
                    parameters: {viewport: {disable: !0}},
                },
                CountdownOnly = {
                    args: {
                        userProfile: {
                            email:
                                "johnhasaverysupersuperduperverysuperduperlongname@sequentech.io",
                            username:
                                "John has a very super super duper very super duper long name",
                            openLink: function openLink() {
                                alert("rouge")
                            },
                        },
                        logoutFn: function logoutFn() {
                            alert("logging out")
                        },
                        setOpenModal: function setOpenModal() {
                            return alert("open log out modal")
                        },
                        handleOpenTimeModal: function handleOpenTimeModal() {
                            return alert("open time modal")
                        },
                        expiry: {
                            endTime: new Date(Date.now() + 3e4),
                            countdown:
                                _sequentech_ui_core__WEBPACK_IMPORTED_MODULE_3__
                                    .EVotingPortalCountdownPolicy.COUNTDOWN,
                            countdownAt: 30,
                            alertAt: 10,
                            duration: 60,
                        },
                        setTimeLeftDialogText: function setTimeLeftDialogText(v) {
                            return console.log({v: v})
                        },
                    },
                    parameters: {viewport: {disable: !0}},
                },
                NoCountdown = {
                    args: {
                        userProfile: {
                            email:
                                "johnhasaverysupersuperduperverysuperduperlongname@sequentech.io",
                            username:
                                "John has a very super super duper very super duper long name",
                            openLink: function openLink() {
                                alert("rouge")
                            },
                        },
                        logoutFn: function logoutFn() {
                            alert("logging out")
                        },
                        setOpenModal: function setOpenModal() {
                            return alert("open log out modal")
                        },
                        handleOpenTimeModal: function handleOpenTimeModal() {
                            return alert("open time modal")
                        },
                        setTimeLeftDialogText: function setTimeLeftDialogText(v) {
                            return console.log({v: v})
                        },
                    },
                    parameters: {viewport: {disable: !0}},
                }
            ;(CountdownWithAlert.parameters = (0,
            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                    {},
                    CountdownWithAlert.parameters
                ),
                {},
                {
                    docs: (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                        (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                            {},
                            null === (_CountdownWithAlert$p = CountdownWithAlert.parameters) ||
                                void 0 === _CountdownWithAlert$p
                                ? void 0
                                : _CountdownWithAlert$p.docs
                        ),
                        {},
                        {
                            source: (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                                {
                                    originalSource:
                                        '{\n  // More on args: https://storybook.js.org/docs/react/writing-stories/args\n  args: {\n    userProfile: {\n      email: "johnhasaverysupersuperduperverysuperduperlongname@sequentech.io",\n      username: "John has a very super super duper very super duper long name",\n      openLink() {\n        alert("rouge");\n      }\n    },\n    logoutFn() {\n      alert("logging out");\n    },\n    setOpenModal: () => alert("open log out modal"),\n    handleOpenTimeModal: () => alert("open time modal"),\n    expiry: {\n      endTime: new Date(Date.now() + 60000),\n      //current time plus 2 minutes\n      countdown: EVotingPortalCountdownPolicy.COUNTDOWN_WITH_ALERT,\n      countdownAt: 60,\n      alertAt: 30\n    },\n    setTimeLeftDialogText: (v: string) => console.log({\n      v\n    })\n  },\n  parameters: {\n    viewport: {\n      disable: true\n    }\n  }\n}',
                                },
                                null === (_CountdownWithAlert$p2 = CountdownWithAlert.parameters) ||
                                    void 0 === _CountdownWithAlert$p2 ||
                                    null ===
                                        (_CountdownWithAlert$p3 = _CountdownWithAlert$p2.docs) ||
                                    void 0 === _CountdownWithAlert$p3
                                    ? void 0
                                    : _CountdownWithAlert$p3.source
                            ),
                        }
                    ),
                }
            )),
                (CountdownOnly.parameters = (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                    (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                        {},
                        CountdownOnly.parameters
                    ),
                    {},
                    {
                        docs: (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                            (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                                {},
                                null === (_CountdownOnly$parame = CountdownOnly.parameters) ||
                                    void 0 === _CountdownOnly$parame
                                    ? void 0
                                    : _CountdownOnly$parame.docs
                            ),
                            {},
                            {
                                source: (0,
                                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                                    {
                                        originalSource:
                                            '{\n  // More on args: https://storybook.js.org/docs/react/writing-stories/args\n  args: {\n    userProfile: {\n      email: "johnhasaverysupersuperduperverysuperduperlongname@sequentech.io",\n      username: "John has a very super super duper very super duper long name",\n      openLink() {\n        alert("rouge");\n      }\n    },\n    logoutFn() {\n      alert("logging out");\n    },\n    setOpenModal: () => alert("open log out modal"),\n    handleOpenTimeModal: () => alert("open time modal"),\n    expiry: {\n      endTime: new Date(Date.now() + 30000),\n      //current time plus 2 minutes\n      countdown: EVotingPortalCountdownPolicy.COUNTDOWN,\n      countdownAt: 30,\n      alertAt: 10,\n      duration: 60\n    },\n    setTimeLeftDialogText: (v: string) => console.log({\n      v\n    })\n  },\n  parameters: {\n    viewport: {\n      disable: true\n    }\n  }\n}',
                                    },
                                    null === (_CountdownOnly$parame2 = CountdownOnly.parameters) ||
                                        void 0 === _CountdownOnly$parame2 ||
                                        null ===
                                            (_CountdownOnly$parame3 =
                                                _CountdownOnly$parame2.docs) ||
                                        void 0 === _CountdownOnly$parame3
                                        ? void 0
                                        : _CountdownOnly$parame3.source
                                ),
                            }
                        ),
                    }
                )),
                (NoCountdown.parameters = (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                    (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                        {},
                        NoCountdown.parameters
                    ),
                    {},
                    {
                        docs: (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                            (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                                {},
                                null === (_NoCountdown$paramete = NoCountdown.parameters) ||
                                    void 0 === _NoCountdown$paramete
                                    ? void 0
                                    : _NoCountdown$paramete.docs
                            ),
                            {},
                            {
                                source: (0,
                                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                                    {
                                        originalSource:
                                            '{\n  // More on args: https://storybook.js.org/docs/react/writing-stories/args\n  args: {\n    userProfile: {\n      email: "johnhasaverysupersuperduperverysuperduperlongname@sequentech.io",\n      username: "John has a very super super duper very super duper long name",\n      openLink() {\n        alert("rouge");\n      }\n    },\n    logoutFn() {\n      alert("logging out");\n    },\n    setOpenModal: () => alert("open log out modal"),\n    handleOpenTimeModal: () => alert("open time modal"),\n    setTimeLeftDialogText: (v: string) => console.log({\n      v\n    })\n  },\n  parameters: {\n    viewport: {\n      disable: true\n    }\n  }\n}',
                                    },
                                    null === (_NoCountdown$paramete2 = NoCountdown.parameters) ||
                                        void 0 === _NoCountdown$paramete2 ||
                                        null ===
                                            (_NoCountdown$paramete3 =
                                                _NoCountdown$paramete2.docs) ||
                                        void 0 === _NoCountdown$paramete3
                                        ? void 0
                                        : _NoCountdown$paramete3.source
                                ),
                            }
                        ),
                    }
                ))
            var __namedExportsOrder = ["CountdownWithAlert", "CountdownOnly", "NoCountdown"]
        },
    },
])
