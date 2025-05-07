;(self.webpackChunk_sequentech_ui_essentials =
    self.webpackChunk_sequentech_ui_essentials || []).push([
    [9026, 5012],
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
        "./src/components/SelectElection/__stories__/SelectElection.mdx": function (
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
                _SelectElection_stories__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(
                    "./src/components/SelectElection/__stories__/SelectElection.stories.tsx"
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
                                    of: _SelectElection_stories__WEBPACK_IMPORTED_MODULE_2__,
                                    title: "components/SelectElection",
                                }
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _components.h1,
                                {id: "selectelection", children: "SelectElection"}
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(_components.p, {
                                children:
                                    "A SelectElection is an election listed in a list of election.",
                            }),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _components.h2,
                                {id: "open-voted", children: "Open Voted"}
                            ),
                            "\n",
                            (0,
                            react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _storybook_blocks__WEBPACK_IMPORTED_MODULE_4__.Xz,
                                {of: _SelectElection_stories__WEBPACK_IMPORTED_MODULE_2__.OpenVoted}
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _components.h2,
                                {id: "closed-not-voted", children: "Closed Not Voted"}
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _storybook_blocks__WEBPACK_IMPORTED_MODULE_4__.Xz,
                                {
                                    of:
                                        _SelectElection_stories__WEBPACK_IMPORTED_MODULE_2__.ClosedNotVoted,
                                }
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _components.h2,
                                {id: "on-hover", children: "On Hover"}
                            ),
                            "\n",
                            (0,
                            react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _storybook_blocks__WEBPACK_IMPORTED_MODULE_4__.Xz,
                                {of: _SelectElection_stories__WEBPACK_IMPORTED_MODULE_2__.OnHover}
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _components.h2,
                                {id: "on-active", children: "On Active"}
                            ),
                            "\n",
                            (0,
                            react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _storybook_blocks__WEBPACK_IMPORTED_MODULE_4__.Xz,
                                {of: _SelectElection_stories__WEBPACK_IMPORTED_MODULE_2__.OnActive}
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _components.h2,
                                {id: "on-focus", children: "On Focus"}
                            ),
                            "\n",
                            (0,
                            react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _storybook_blocks__WEBPACK_IMPORTED_MODULE_4__.Xz,
                                {of: _SelectElection_stories__WEBPACK_IMPORTED_MODULE_2__.OnFocus}
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _components.h2,
                                {id: "with-ballot-locator", children: "With Ballot locator"}
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _storybook_blocks__WEBPACK_IMPORTED_MODULE_4__.Xz,
                                {
                                    of:
                                        _SelectElection_stories__WEBPACK_IMPORTED_MODULE_2__.DisplayBallotLocator,
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
        "./src/components/SelectElection/__stories__/SelectElection.stories.tsx": function (
            __unused_webpack_module,
            __webpack_exports__,
            __webpack_require__
        ) {
            __webpack_require__.r(__webpack_exports__),
                __webpack_require__.d(__webpack_exports__, {
                    ClosedNotVoted: function () {
                        return ClosedNotVoted
                    },
                    DisplayBallotLocator: function () {
                        return DisplayBallotLocator
                    },
                    OnActive: function () {
                        return OnActive
                    },
                    OnFocus: function () {
                        return OnFocus
                    },
                    OnHover: function () {
                        return OnHover
                    },
                    OpenVoted: function () {
                        return OpenVoted
                    },
                    __namedExportsOrder: function () {
                        return __namedExportsOrder
                    },
                })
            var _OpenVoted$parameters,
                _OpenVoted$parameters2,
                _OpenVoted$parameters3,
                _OnHover$parameters,
                _OnHover$parameters2,
                _OnHover$parameters2$,
                _OnActive$parameters,
                _OnActive$parameters2,
                _OnActive$parameters3,
                _OnFocus$parameters,
                _OnFocus$parameters2,
                _OnFocus$parameters2$,
                _ClosedNotVoted$param,
                _ClosedNotVoted$param2,
                _ClosedNotVoted$param3,
                _DisplayBallotLocator,
                _DisplayBallotLocator2,
                _DisplayBallotLocator3,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__ = __webpack_require__(
                    "../node_modules/@babel/runtime/helpers/esm/objectSpread2.js"
                ),
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectWithoutProperties_js__WEBPACK_IMPORTED_MODULE_4__ = __webpack_require__(
                    "../node_modules/@babel/runtime/helpers/esm/objectWithoutProperties.js"
                ),
                _SelectElection__WEBPACK_IMPORTED_MODULE_1__ =
                    (__webpack_require__("../node_modules/react/index.js"),
                    __webpack_require__("./src/components/SelectElection/SelectElection.tsx")),
                _storybook_addon_viewport__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(
                    "../node_modules/@storybook/addon-viewport/dist/index.mjs"
                ),
                _mui_material__WEBPACK_IMPORTED_MODULE_5__ = __webpack_require__(
                    "../node_modules/@mui/material/Box/Box.js"
                ),
                react_jsx_runtime__WEBPACK_IMPORTED_MODULE_3__ = __webpack_require__(
                    "../node_modules/react/jsx-runtime.js"
                ),
                _excluded = ["className"],
                meta = {
                    title: "components/SelectElection",
                    component: function SelectElectionWrapper(_ref) {
                        var className = _ref.className,
                            props = (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectWithoutProperties_js__WEBPACK_IMPORTED_MODULE_4__.Z)(
                                _ref,
                                _excluded
                            )
                        return (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_3__.jsx)(
                            _mui_material__WEBPACK_IMPORTED_MODULE_5__.Z,
                            {
                                className: className,
                                children: (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_3__.jsx)(
                                    _SelectElection__WEBPACK_IMPORTED_MODULE_1__.Z,
                                    (0,
                                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                                        {},
                                        props
                                    )
                                ),
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
            var OpenVoted = {
                    args: {
                        isActive: !0,
                        isOpen: !0,
                        title: "Executive Board",
                        electionHomeUrl: "/election/34570007/public/home",
                        hasVoted: !0,
                        electionDates: {first_started_at: "2025-10-29T14:00:00.000Z"},
                    },
                    parameters: {backgrounds: {default: "white"}, viewport: {disable: !0}},
                },
                OnHover = {
                    args: {
                        isActive: !0,
                        isOpen: !0,
                        title: "Executive Board",
                        electionHomeUrl: "/election/34570007/public/home",
                        hasVoted: !0,
                        className: "hover",
                        electionDates: {first_started_at: "2025-10-29T14:00:00.000Z"},
                    },
                    parameters: {
                        pseudo: {hover: [".hover"], active: [".active"], focus: [".focus"]},
                        viewport: {disable: !0},
                    },
                },
                OnActive = {
                    args: {
                        isActive: !0,
                        isOpen: !0,
                        title: "Executive Board",
                        electionHomeUrl: "/election/34570007/public/home",
                        hasVoted: !0,
                        className: "active",
                        electionDates: {first_started_at: "2025-10-29T14:00:00.000Z"},
                    },
                    parameters: {
                        pseudo: {hover: [".hover"], active: [".active"], focus: [".focus"]},
                        viewport: {disable: !0},
                    },
                },
                OnFocus = {
                    args: {
                        isActive: !0,
                        isOpen: !0,
                        title: "Executive Board",
                        electionHomeUrl: "/election/34570007/public/home",
                        hasVoted: !0,
                        className: "focus",
                        electionDates: {first_started_at: "2025-10-29T14:00:00.000Z"},
                    },
                    parameters: {
                        pseudo: {hover: [".hover"], active: [".active"], focus: [".focus"]},
                        viewport: {disable: !0},
                    },
                },
                ClosedNotVoted = {
                    args: {
                        isActive: !1,
                        isOpen: !1,
                        title: "Executive Board",
                        electionHomeUrl: "/election/34570007/public/home",
                        hasVoted: !1,
                        electionDates: {first_started_at: "2025-10-29T14:00:00.000Z"},
                    },
                    parameters: {viewport: {disable: !0}},
                },
                DisplayBallotLocator = {
                    args: {
                        isActive: !0,
                        isOpen: !0,
                        title: "Executive Board",
                        electionHomeUrl: "/election/34570007/public/home",
                        hasVoted: !1,
                        onClickBallotLocator: function onClickBallotLocator() {
                            console.log("Clicked to locate the ballot")
                        },
                        electionDates: {first_started_at: "2025-10-29T14:00:00.000Z"},
                    },
                    parameters: {viewport: {disable: !0}},
                }
            ;(OpenVoted.parameters = (0,
            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                    {},
                    OpenVoted.parameters
                ),
                {},
                {
                    docs: (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                        (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                            {},
                            null === (_OpenVoted$parameters = OpenVoted.parameters) ||
                                void 0 === _OpenVoted$parameters
                                ? void 0
                                : _OpenVoted$parameters.docs
                        ),
                        {},
                        {
                            source: (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                                {
                                    originalSource:
                                        '{\n  args: {\n    isActive: true,\n    isOpen: true,\n    title: "Executive Board",\n    electionHomeUrl: "/election/34570007/public/home",\n    hasVoted: true,\n    electionDates: {\n      first_started_at: "2025-10-29T14:00:00.000Z"\n    }\n  },\n  parameters: {\n    backgrounds: {\n      default: "white"\n    },\n    viewport: {\n      disable: true\n    }\n  }\n}',
                                },
                                null === (_OpenVoted$parameters2 = OpenVoted.parameters) ||
                                    void 0 === _OpenVoted$parameters2 ||
                                    null ===
                                        (_OpenVoted$parameters3 = _OpenVoted$parameters2.docs) ||
                                    void 0 === _OpenVoted$parameters3
                                    ? void 0
                                    : _OpenVoted$parameters3.source
                            ),
                        }
                    ),
                }
            )),
                (OnHover.parameters = (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                    (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                        {},
                        OnHover.parameters
                    ),
                    {},
                    {
                        docs: (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                            (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                                {},
                                null === (_OnHover$parameters = OnHover.parameters) ||
                                    void 0 === _OnHover$parameters
                                    ? void 0
                                    : _OnHover$parameters.docs
                            ),
                            {},
                            {
                                source: (0,
                                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                                    {
                                        originalSource:
                                            '{\n  args: {\n    isActive: true,\n    isOpen: true,\n    title: "Executive Board",\n    electionHomeUrl: "/election/34570007/public/home",\n    hasVoted: true,\n    className: "hover",\n    electionDates: {\n      first_started_at: "2025-10-29T14:00:00.000Z"\n    }\n  },\n  parameters: {\n    pseudo: {\n      hover: [".hover"],\n      active: [".active"],\n      focus: [".focus"]\n    },\n    viewport: {\n      disable: true\n    }\n  }\n}',
                                    },
                                    null === (_OnHover$parameters2 = OnHover.parameters) ||
                                        void 0 === _OnHover$parameters2 ||
                                        null ===
                                            (_OnHover$parameters2$ = _OnHover$parameters2.docs) ||
                                        void 0 === _OnHover$parameters2$
                                        ? void 0
                                        : _OnHover$parameters2$.source
                                ),
                            }
                        ),
                    }
                )),
                (OnActive.parameters = (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                    (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                        {},
                        OnActive.parameters
                    ),
                    {},
                    {
                        docs: (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                            (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                                {},
                                null === (_OnActive$parameters = OnActive.parameters) ||
                                    void 0 === _OnActive$parameters
                                    ? void 0
                                    : _OnActive$parameters.docs
                            ),
                            {},
                            {
                                source: (0,
                                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                                    {
                                        originalSource:
                                            '{\n  args: {\n    isActive: true,\n    isOpen: true,\n    title: "Executive Board",\n    electionHomeUrl: "/election/34570007/public/home",\n    hasVoted: true,\n    className: "active",\n    electionDates: {\n      first_started_at: "2025-10-29T14:00:00.000Z"\n    }\n  },\n  parameters: {\n    pseudo: {\n      hover: [".hover"],\n      active: [".active"],\n      focus: [".focus"]\n    },\n    viewport: {\n      disable: true\n    }\n  }\n}',
                                    },
                                    null === (_OnActive$parameters2 = OnActive.parameters) ||
                                        void 0 === _OnActive$parameters2 ||
                                        null ===
                                            (_OnActive$parameters3 = _OnActive$parameters2.docs) ||
                                        void 0 === _OnActive$parameters3
                                        ? void 0
                                        : _OnActive$parameters3.source
                                ),
                            }
                        ),
                    }
                )),
                (OnFocus.parameters = (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                    (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                        {},
                        OnFocus.parameters
                    ),
                    {},
                    {
                        docs: (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                            (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                                {},
                                null === (_OnFocus$parameters = OnFocus.parameters) ||
                                    void 0 === _OnFocus$parameters
                                    ? void 0
                                    : _OnFocus$parameters.docs
                            ),
                            {},
                            {
                                source: (0,
                                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                                    {
                                        originalSource:
                                            '{\n  args: {\n    isActive: true,\n    isOpen: true,\n    title: "Executive Board",\n    electionHomeUrl: "/election/34570007/public/home",\n    hasVoted: true,\n    className: "focus",\n    electionDates: {\n      first_started_at: "2025-10-29T14:00:00.000Z"\n    }\n  },\n  parameters: {\n    pseudo: {\n      hover: [".hover"],\n      active: [".active"],\n      focus: [".focus"]\n    },\n    viewport: {\n      disable: true\n    }\n  }\n}',
                                    },
                                    null === (_OnFocus$parameters2 = OnFocus.parameters) ||
                                        void 0 === _OnFocus$parameters2 ||
                                        null ===
                                            (_OnFocus$parameters2$ = _OnFocus$parameters2.docs) ||
                                        void 0 === _OnFocus$parameters2$
                                        ? void 0
                                        : _OnFocus$parameters2$.source
                                ),
                            }
                        ),
                    }
                )),
                (ClosedNotVoted.parameters = (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                    (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                        {},
                        ClosedNotVoted.parameters
                    ),
                    {},
                    {
                        docs: (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                            (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                                {},
                                null === (_ClosedNotVoted$param = ClosedNotVoted.parameters) ||
                                    void 0 === _ClosedNotVoted$param
                                    ? void 0
                                    : _ClosedNotVoted$param.docs
                            ),
                            {},
                            {
                                source: (0,
                                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                                    {
                                        originalSource:
                                            '{\n  args: {\n    isActive: false,\n    isOpen: false,\n    title: "Executive Board",\n    electionHomeUrl: "/election/34570007/public/home",\n    hasVoted: false,\n    electionDates: {\n      first_started_at: "2025-10-29T14:00:00.000Z"\n    }\n  },\n  parameters: {\n    viewport: {\n      disable: true\n    }\n  }\n}',
                                    },
                                    null === (_ClosedNotVoted$param2 = ClosedNotVoted.parameters) ||
                                        void 0 === _ClosedNotVoted$param2 ||
                                        null ===
                                            (_ClosedNotVoted$param3 =
                                                _ClosedNotVoted$param2.docs) ||
                                        void 0 === _ClosedNotVoted$param3
                                        ? void 0
                                        : _ClosedNotVoted$param3.source
                                ),
                            }
                        ),
                    }
                )),
                (DisplayBallotLocator.parameters = (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                    (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                        {},
                        DisplayBallotLocator.parameters
                    ),
                    {},
                    {
                        docs: (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                            (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                                {},
                                null ===
                                    (_DisplayBallotLocator = DisplayBallotLocator.parameters) ||
                                    void 0 === _DisplayBallotLocator
                                    ? void 0
                                    : _DisplayBallotLocator.docs
                            ),
                            {},
                            {
                                source: (0,
                                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                                    {
                                        originalSource:
                                            '{\n  args: {\n    isActive: true,\n    isOpen: true,\n    title: "Executive Board",\n    electionHomeUrl: "/election/34570007/public/home",\n    hasVoted: false,\n    onClickBallotLocator() {\n      console.log("Clicked to locate the ballot");\n    },\n    electionDates: {\n      first_started_at: "2025-10-29T14:00:00.000Z"\n    }\n  },\n  parameters: {\n    viewport: {\n      disable: true\n    }\n  }\n}',
                                    },
                                    null ===
                                        (_DisplayBallotLocator2 =
                                            DisplayBallotLocator.parameters) ||
                                        void 0 === _DisplayBallotLocator2 ||
                                        null ===
                                            (_DisplayBallotLocator3 =
                                                _DisplayBallotLocator2.docs) ||
                                        void 0 === _DisplayBallotLocator3
                                        ? void 0
                                        : _DisplayBallotLocator3.source
                                ),
                            }
                        ),
                    }
                ))
            var __namedExportsOrder = [
                "OpenVoted",
                "OnHover",
                "OnActive",
                "OnFocus",
                "ClosedNotVoted",
                "DisplayBallotLocator",
            ]
        },
    },
])
