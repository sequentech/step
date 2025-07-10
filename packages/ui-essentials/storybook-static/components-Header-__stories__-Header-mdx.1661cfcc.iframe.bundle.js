(self.webpackChunk_sequentech_ui_essentials =
    self.webpackChunk_sequentech_ui_essentials || []).push([
    [9124, 779],
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
        "./src/components/Header/__stories__/Header.mdx": function (
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
                _Header_stories__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(
                    "./src/components/Header/__stories__/Header.stories.tsx"
                )
            function _createMdxContent(props) {
                const _components = Object.assign(
                    {h1: "h1", p: "p", h2: "h2", h3: "h3", h4: "h4"},
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
                                    of: _Header_stories__WEBPACK_IMPORTED_MODULE_2__,
                                    title: "components/Header",
                                }
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _components.h1,
                                {id: "header", children: "Header"}
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(_components.p, {
                                children:
                                    "A Header is the box at the top of every screen. It includes the logo, the\nsoftware version, language toggle and an optional button to log out.",
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
                                {of: _Header_stories__WEBPACK_IMPORTED_MODULE_2__.Primary}
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _components.h3,
                                {id: "with-user-profile", children: "With user profile"}
                            ),
                            "\n",
                            (0,
                            react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _storybook_blocks__WEBPACK_IMPORTED_MODULE_4__.Xz,
                                {of: _Header_stories__WEBPACK_IMPORTED_MODULE_2__.WithUserProfile}
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _components.h4,
                                {id: "with-long-display-name", children: "With long display name"}
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _storybook_blocks__WEBPACK_IMPORTED_MODULE_4__.Xz,
                                {
                                    of:
                                        _Header_stories__WEBPACK_IMPORTED_MODULE_2__.WithUserProfileLong,
                                }
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _components.h2,
                                {id: "mobile", children: "Mobile"}
                            ),
                            "\n",
                            (0,
                            react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _storybook_blocks__WEBPACK_IMPORTED_MODULE_4__.Xz,
                                {of: _Header_stories__WEBPACK_IMPORTED_MODULE_2__.PrimaryMobile}
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _components.h2,
                                {
                                    id: "hidden-user-profile-show-logout-button",
                                    children: "Hidden User Profile (Show Logout Button)",
                                }
                            ),
                            "\n",
                            (0,
                            react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _storybook_blocks__WEBPACK_IMPORTED_MODULE_4__.Xz,
                                {of: _Header_stories__WEBPACK_IMPORTED_MODULE_2__.HiddenUserProfile}
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
        "./src/components/Header/__stories__/Header.stories.tsx": function (
            __unused_webpack_module,
            __webpack_exports__,
            __webpack_require__
        ) {
            __webpack_require__.r(__webpack_exports__),
                __webpack_require__.d(__webpack_exports__, {
                    HiddenUserProfile: function () {
                        return HiddenUserProfile
                    },
                    Primary: function () {
                        return Primary
                    },
                    PrimaryMobile: function () {
                        return PrimaryMobile
                    },
                    WithUserProfile: function () {
                        return WithUserProfile
                    },
                    WithUserProfileLong: function () {
                        return WithUserProfileLong
                    },
                    __namedExportsOrder: function () {
                        return __namedExportsOrder
                    },
                })
            var _Primary$parameters,
                _Primary$parameters2,
                _Primary$parameters2$,
                _PrimaryMobile$parame,
                _PrimaryMobile$parame2,
                _PrimaryMobile$parame3,
                _WithUserProfile$para,
                _WithUserProfile$para2,
                _WithUserProfile$para3,
                _WithUserProfileLong$,
                _WithUserProfileLong$2,
                _WithUserProfileLong$3,
                _HiddenUserProfile$pa,
                _HiddenUserProfile$pa2,
                _HiddenUserProfile$pa3,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(
                    "../node_modules/@babel/runtime/helpers/esm/objectSpread2.js"
                ),
                _Header__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(
                    "./src/components/Header/Header.tsx"
                ),
                _storybook_addon_viewport__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(
                    "../node_modules/@storybook/addon-viewport/dist/index.mjs"
                ),
                meta = {
                    title: "components/Header",
                    component: _Header__WEBPACK_IMPORTED_MODULE_0__.ZP,
                    parameters: {
                        backgrounds: {default: "white"},
                        viewport: {
                            viewports: _storybook_addon_viewport__WEBPACK_IMPORTED_MODULE_1__.p,
                            defaultViewport: "iphone6",
                        },
                    },
                }
            __webpack_exports__.default = meta
            var Primary = {args: {}, parameters: {viewport: {disable: !0}}},
                PrimaryMobile = {
                    args: {logoutFn: function logoutFn() {}},
                    parameters: {viewport: {defaultViewport: "iphone6"}},
                },
                WithUserProfile = {
                    args: {
                        userProfile: {
                            email: "john@sequentech.io",
                            username: "John Doe",
                            openLink: function openLink() {
                                alert("rouge")
                            },
                        },
                        logoutFn: function logoutFn() {
                            alert("logging out")
                        },
                    },
                    parameters: {viewport: {disable: !0}},
                },
                WithUserProfileLong = {
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
                    },
                    parameters: {viewport: {disable: !0}},
                },
                HiddenUserProfile = {
                    args: {
                        userProfile: {
                            email: "john@sequentech.io",
                            username: "John Doe",
                            openLink: function openLink() {
                                alert("rouge")
                            },
                        },
                        logoutFn: function logoutFn() {
                            alert("logging out")
                        },
                        errorVariant: _Header__WEBPACK_IMPORTED_MODULE_0__.Uj.HIDE_PROFILE,
                    },
                    parameters: {viewport: {disable: !0}},
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
                                        "{\n  // More on args: https://storybook.js.org/docs/react/writing-stories/args\n  args: {},\n  parameters: {\n    viewport: {\n      disable: true\n    }\n  }\n}",
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
                (PrimaryMobile.parameters = (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_2__.Z)(
                    (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_2__.Z)(
                        {},
                        PrimaryMobile.parameters
                    ),
                    {},
                    {
                        docs: (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_2__.Z)(
                            (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_2__.Z)(
                                {},
                                null === (_PrimaryMobile$parame = PrimaryMobile.parameters) ||
                                    void 0 === _PrimaryMobile$parame
                                    ? void 0
                                    : _PrimaryMobile$parame.docs
                            ),
                            {},
                            {
                                source: (0,
                                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_2__.Z)(
                                    {
                                        originalSource:
                                            '{\n  // More on args: https://storybook.js.org/docs/react/writing-stories/args\n  args: {\n    logoutFn: () => {}\n  },\n  parameters: {\n    viewport: {\n      defaultViewport: "iphone6"\n    }\n  }\n}',
                                    },
                                    null === (_PrimaryMobile$parame2 = PrimaryMobile.parameters) ||
                                        void 0 === _PrimaryMobile$parame2 ||
                                        null ===
                                            (_PrimaryMobile$parame3 =
                                                _PrimaryMobile$parame2.docs) ||
                                        void 0 === _PrimaryMobile$parame3
                                        ? void 0
                                        : _PrimaryMobile$parame3.source
                                ),
                            }
                        ),
                    }
                )),
                (WithUserProfile.parameters = (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_2__.Z)(
                    (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_2__.Z)(
                        {},
                        WithUserProfile.parameters
                    ),
                    {},
                    {
                        docs: (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_2__.Z)(
                            (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_2__.Z)(
                                {},
                                null === (_WithUserProfile$para = WithUserProfile.parameters) ||
                                    void 0 === _WithUserProfile$para
                                    ? void 0
                                    : _WithUserProfile$para.docs
                            ),
                            {},
                            {
                                source: (0,
                                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_2__.Z)(
                                    {
                                        originalSource:
                                            '{\n  args: {\n    userProfile: {\n      email: "john@sequentech.io",\n      username: "John Doe",\n      openLink() {\n        alert("rouge");\n      }\n    },\n    logoutFn() {\n      alert("logging out");\n    }\n  },\n  parameters: {\n    viewport: {\n      disable: true\n    }\n  }\n}',
                                    },
                                    null ===
                                        (_WithUserProfile$para2 = WithUserProfile.parameters) ||
                                        void 0 === _WithUserProfile$para2 ||
                                        null ===
                                            (_WithUserProfile$para3 =
                                                _WithUserProfile$para2.docs) ||
                                        void 0 === _WithUserProfile$para3
                                        ? void 0
                                        : _WithUserProfile$para3.source
                                ),
                            }
                        ),
                    }
                )),
                (WithUserProfileLong.parameters = (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_2__.Z)(
                    (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_2__.Z)(
                        {},
                        WithUserProfileLong.parameters
                    ),
                    {},
                    {
                        docs: (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_2__.Z)(
                            (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_2__.Z)(
                                {},
                                null === (_WithUserProfileLong$ = WithUserProfileLong.parameters) ||
                                    void 0 === _WithUserProfileLong$
                                    ? void 0
                                    : _WithUserProfileLong$.docs
                            ),
                            {},
                            {
                                source: (0,
                                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_2__.Z)(
                                    {
                                        originalSource:
                                            '{\n  args: {\n    userProfile: {\n      email: "johnhasaverysupersuperduperverysuperduperlongname@sequentech.io",\n      username: "John has a very super super duper very super duper long name",\n      openLink() {\n        alert("rouge");\n      }\n    },\n    logoutFn() {\n      alert("logging out");\n    }\n  },\n  parameters: {\n    viewport: {\n      disable: true\n    }\n  }\n}',
                                    },
                                    null ===
                                        (_WithUserProfileLong$2 = WithUserProfileLong.parameters) ||
                                        void 0 === _WithUserProfileLong$2 ||
                                        null ===
                                            (_WithUserProfileLong$3 =
                                                _WithUserProfileLong$2.docs) ||
                                        void 0 === _WithUserProfileLong$3
                                        ? void 0
                                        : _WithUserProfileLong$3.source
                                ),
                            }
                        ),
                    }
                )),
                (HiddenUserProfile.parameters = (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_2__.Z)(
                    (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_2__.Z)(
                        {},
                        HiddenUserProfile.parameters
                    ),
                    {},
                    {
                        docs: (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_2__.Z)(
                            (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_2__.Z)(
                                {},
                                null === (_HiddenUserProfile$pa = HiddenUserProfile.parameters) ||
                                    void 0 === _HiddenUserProfile$pa
                                    ? void 0
                                    : _HiddenUserProfile$pa.docs
                            ),
                            {},
                            {
                                source: (0,
                                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_2__.Z)(
                                    {
                                        originalSource:
                                            '{\n  args: {\n    userProfile: {\n      email: "john@sequentech.io",\n      username: "John Doe",\n      openLink() {\n        alert("rouge");\n      }\n    },\n    logoutFn() {\n      alert("logging out");\n    },\n    errorVariant: HeaderErrorVariant.HIDE_PROFILE\n  },\n  parameters: {\n    viewport: {\n      disable: true\n    }\n  }\n}',
                                    },
                                    null ===
                                        (_HiddenUserProfile$pa2 = HiddenUserProfile.parameters) ||
                                        void 0 === _HiddenUserProfile$pa2 ||
                                        null ===
                                            (_HiddenUserProfile$pa3 =
                                                _HiddenUserProfile$pa2.docs) ||
                                        void 0 === _HiddenUserProfile$pa3
                                        ? void 0
                                        : _HiddenUserProfile$pa3.source
                                ),
                            }
                        ),
                    }
                ))
            var __namedExportsOrder = [
                "Primary",
                "PrimaryMobile",
                "WithUserProfile",
                "WithUserProfileLong",
                "HiddenUserProfile",
            ]
        },
    },
])
