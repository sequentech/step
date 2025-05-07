(self.webpackChunk_sequentech_ui_essentials =
    self.webpackChunk_sequentech_ui_essentials || []).push([
    [6784, 9751],
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
        "./src/components/Candidate/__stories__/Candidate.mdx": function (
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
                _Candidate_stories__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(
                    "./src/components/Candidate/__stories__/Candidate.stories.tsx"
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
                                    of: _Candidate_stories__WEBPACK_IMPORTED_MODULE_2__,
                                    title: "components/Candidate",
                                }
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _components.h1,
                                {id: "candidate", children: "Candidate"}
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(_components.p, {
                                children:
                                    "A Candidate is an option in an election. It may be checkable or not.",
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
                                {of: _Candidate_stories__WEBPACK_IMPORTED_MODULE_2__.Primary}
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _components.h2,
                                {id: "read-only", children: "Read Only"}
                            ),
                            "\n",
                            (0,
                            react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _storybook_blocks__WEBPACK_IMPORTED_MODULE_4__.Xz,
                                {of: _Candidate_stories__WEBPACK_IMPORTED_MODULE_2__.ReadOnly}
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
                                {of: _Candidate_stories__WEBPACK_IMPORTED_MODULE_2__.Hover}
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _components.h2,
                                {id: "on-click", children: "On Click"}
                            ),
                            "\n",
                            (0,
                            react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _storybook_blocks__WEBPACK_IMPORTED_MODULE_4__.Xz,
                                {of: _Candidate_stories__WEBPACK_IMPORTED_MODULE_2__.Active}
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _components.h2,
                                {id: "no-image", children: "No Image"}
                            ),
                            "\n",
                            (0,
                            react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _storybook_blocks__WEBPACK_IMPORTED_MODULE_4__.Xz,
                                {of: _Candidate_stories__WEBPACK_IMPORTED_MODULE_2__.NoImage}
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _components.h2,
                                {id: "no-description", children: "No Description"}
                            ),
                            "\n",
                            (0,
                            react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _storybook_blocks__WEBPACK_IMPORTED_MODULE_4__.Xz,
                                {of: _Candidate_stories__WEBPACK_IMPORTED_MODULE_2__.NoDescription}
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _components.h2,
                                {id: "only-title", children: "Only Title"}
                            ),
                            "\n",
                            (0,
                            react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _storybook_blocks__WEBPACK_IMPORTED_MODULE_4__.Xz,
                                {of: _Candidate_stories__WEBPACK_IMPORTED_MODULE_2__.OnlyTitle}
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _components.h2,
                                {id: "long-description", children: "Long Description"}
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _storybook_blocks__WEBPACK_IMPORTED_MODULE_4__.Xz,
                                {
                                    of:
                                        _Candidate_stories__WEBPACK_IMPORTED_MODULE_2__.LongDescription,
                                }
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _components.h2,
                                {id: "long-title", children: "Long Title"}
                            ),
                            "\n",
                            (0,
                            react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _storybook_blocks__WEBPACK_IMPORTED_MODULE_4__.Xz,
                                {of: _Candidate_stories__WEBPACK_IMPORTED_MODULE_2__.LongTitle}
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _components.h2,
                                {id: "with-html", children: "With HTML"}
                            ),
                            "\n",
                            (0,
                            react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _storybook_blocks__WEBPACK_IMPORTED_MODULE_4__.Xz,
                                {of: _Candidate_stories__WEBPACK_IMPORTED_MODULE_2__.WithHtml}
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _components.h2,
                                {id: "write-in-simple", children: "Write In (Simple)"}
                            ),
                            "\n",
                            (0,
                            react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _storybook_blocks__WEBPACK_IMPORTED_MODULE_4__.Xz,
                                {of: _Candidate_stories__WEBPACK_IMPORTED_MODULE_2__.WriteInSimple}
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _components.h2,
                                {id: "write-in-fields", children: "Write In (Fields)"}
                            ),
                            "\n",
                            (0,
                            react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _storybook_blocks__WEBPACK_IMPORTED_MODULE_4__.Xz,
                                {of: _Candidate_stories__WEBPACK_IMPORTED_MODULE_2__.WriteInFields}
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _components.h2,
                                {id: "write-in-invalid", children: "Write In (Invalid)"}
                            ),
                            "\n",
                            (0,
                            react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _storybook_blocks__WEBPACK_IMPORTED_MODULE_4__.Xz,
                                {of: _Candidate_stories__WEBPACK_IMPORTED_MODULE_2__.WriteInInvalid}
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _components.h2,
                                {id: "invalid-vote", children: "Invalid Vote"}
                            ),
                            "\n",
                            (0,
                            react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _storybook_blocks__WEBPACK_IMPORTED_MODULE_4__.Xz,
                                {of: _Candidate_stories__WEBPACK_IMPORTED_MODULE_2__.InvalidVote}
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
        "./src/components/Candidate/__stories__/Candidate.stories.tsx": function (
            __unused_webpack_module,
            __webpack_exports__,
            __webpack_require__
        ) {
            __webpack_require__.r(__webpack_exports__),
                __webpack_require__.d(__webpack_exports__, {
                    Active: function () {
                        return Active
                    },
                    Hover: function () {
                        return Hover
                    },
                    InvalidVote: function () {
                        return InvalidVote
                    },
                    LongDescription: function () {
                        return LongDescription
                    },
                    LongTitle: function () {
                        return LongTitle
                    },
                    NoDescription: function () {
                        return NoDescription
                    },
                    NoImage: function () {
                        return NoImage
                    },
                    OnlyTitle: function () {
                        return OnlyTitle
                    },
                    Primary: function () {
                        return Primary
                    },
                    ReadOnly: function () {
                        return ReadOnly
                    },
                    WithHtml: function () {
                        return WithHtml
                    },
                    WriteInFields: function () {
                        return WriteInFields
                    },
                    WriteInInvalid: function () {
                        return WriteInInvalid
                    },
                    WriteInSimple: function () {
                        return WriteInSimple
                    },
                    __namedExportsOrder: function () {
                        return __namedExportsOrder
                    },
                })
            var _Primary$parameters,
                _Primary$parameters2,
                _Primary$parameters2$,
                _ReadOnly$parameters,
                _ReadOnly$parameters2,
                _ReadOnly$parameters3,
                _NoImage$parameters,
                _NoImage$parameters2,
                _NoImage$parameters2$,
                _NoDescription$parame,
                _NoDescription$parame2,
                _NoDescription$parame3,
                _OnlyTitle$parameters,
                _OnlyTitle$parameters2,
                _OnlyTitle$parameters3,
                _LongDescription$para,
                _LongDescription$para2,
                _LongDescription$para3,
                _LongTitle$parameters,
                _LongTitle$parameters2,
                _LongTitle$parameters3,
                _WithHtml$parameters,
                _WithHtml$parameters2,
                _WithHtml$parameters3,
                _Hover$parameters,
                _Hover$parameters2,
                _Hover$parameters2$do,
                _Active$parameters,
                _Active$parameters2,
                _Active$parameters2$d,
                _WriteInSimple$parame,
                _WriteInSimple$parame2,
                _WriteInSimple$parame3,
                _WriteInInvalid$param,
                _WriteInInvalid$param2,
                _WriteInInvalid$param3,
                _WriteInFields$parame,
                _WriteInFields$parame2,
                _WriteInFields$parame3,
                _InvalidVote$paramete,
                _InvalidVote$paramete2,
                _InvalidVote$paramete3,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__ = __webpack_require__(
                    "../node_modules/@babel/runtime/helpers/esm/objectSpread2.js"
                ),
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectWithoutProperties_js__WEBPACK_IMPORTED_MODULE_6__ = __webpack_require__(
                    "../node_modules/@babel/runtime/helpers/esm/objectWithoutProperties.js"
                ),
                _Candidate__WEBPACK_IMPORTED_MODULE_1__ =
                    (__webpack_require__("../node_modules/react/index.js"),
                    __webpack_require__("./src/components/Candidate/Candidate.tsx")),
                _storybook_addon_viewport__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(
                    "../node_modules/@storybook/addon-viewport/dist/index.mjs"
                ),
                _mui_material__WEBPACK_IMPORTED_MODULE_7__ = __webpack_require__(
                    "../node_modules/@mui/material/Box/Box.js"
                ),
                mui_image__WEBPACK_IMPORTED_MODULE_3__ = __webpack_require__(
                    "../node_modules/mui-image/es/index.js"
                ),
                _public_example_candidate_jpg__WEBPACK_IMPORTED_MODULE_4__ = __webpack_require__(
                    "./public/example_candidate.jpg"
                ),
                react_jsx_runtime__WEBPACK_IMPORTED_MODULE_5__ = __webpack_require__(
                    "../node_modules/react/jsx-runtime.js"
                ),
                _excluded = ["className"],
                meta = {
                    title: "components/Candidate",
                    component: function CandidateWrapper(_ref) {
                        var className = _ref.className,
                            props = (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectWithoutProperties_js__WEBPACK_IMPORTED_MODULE_6__.Z)(
                                _ref,
                                _excluded
                            )
                        return (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_5__.jsx)(
                            _mui_material__WEBPACK_IMPORTED_MODULE_7__.Z,
                            {
                                className: className,
                                children: (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_5__.jsx)(
                                    _Candidate__WEBPACK_IMPORTED_MODULE_1__.Z,
                                    (0,
                                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
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
            var Primary = {
                    args: {
                        children: (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_5__.jsx)(
                            mui_image__WEBPACK_IMPORTED_MODULE_3__.Z,
                            {
                                src: _public_example_candidate_jpg__WEBPACK_IMPORTED_MODULE_4__,
                                duration: 100,
                            }
                        ),
                        title: "Micky Mouse",
                        description: "Candidate Description",
                        isActive: !0,
                        checked: !0,
                        url: "https://google.com",
                    },
                    parameters: {viewport: {disable: !0}},
                },
                ReadOnly = {
                    args: {
                        children: (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_5__.jsx)(
                            mui_image__WEBPACK_IMPORTED_MODULE_3__.Z,
                            {
                                src: _public_example_candidate_jpg__WEBPACK_IMPORTED_MODULE_4__,
                                duration: 100,
                            }
                        ),
                        title: "Micky Mouse",
                        description: "Candidate Description",
                        isActive: !1,
                        checked: !0,
                        url: "https://google.com",
                    },
                    parameters: {viewport: {disable: !0}},
                },
                NoImage = {
                    args: {
                        title: "Micky Mouse",
                        description: "Candidate Description",
                        isActive: !0,
                        url: "https://google.com",
                    },
                    parameters: {viewport: {disable: !0}},
                },
                NoDescription = {
                    args: {
                        children: (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_5__.jsx)(
                            mui_image__WEBPACK_IMPORTED_MODULE_3__.Z,
                            {
                                src: _public_example_candidate_jpg__WEBPACK_IMPORTED_MODULE_4__,
                                duration: 100,
                            }
                        ),
                        title: "Micky Mouse",
                        isActive: !0,
                        url: "https://google.com",
                        checked: !1,
                    },
                    parameters: {viewport: {disable: !0}},
                },
                OnlyTitle = {
                    args: {title: "Micky Mouse", isActive: !0, checked: !1},
                    parameters: {viewport: {disable: !0}},
                },
                LongDescription = {
                    args: {
                        children: (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_5__.jsx)(
                            mui_image__WEBPACK_IMPORTED_MODULE_3__.Z,
                            {
                                src: _public_example_candidate_jpg__WEBPACK_IMPORTED_MODULE_4__,
                                duration: 100,
                            }
                        ),
                        title: "Micky Mouse",
                        description:
                            "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.",
                    },
                    parameters: {viewport: {disable: !0}},
                },
                LongTitle = {
                    args: {
                        children: (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_5__.jsx)(
                            mui_image__WEBPACK_IMPORTED_MODULE_3__.Z,
                            {
                                src: _public_example_candidate_jpg__WEBPACK_IMPORTED_MODULE_4__,
                                duration: 100,
                            }
                        ),
                        title:
                            "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.",
                        description: "Candidate Description",
                        isActive: !0,
                        url: "https://google.com",
                    },
                    parameters: {viewport: {disable: !0}},
                },
                WithHtml = {
                    args: {
                        children: (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_5__.jsx)(
                            mui_image__WEBPACK_IMPORTED_MODULE_3__.Z,
                            {
                                src: _public_example_candidate_jpg__WEBPACK_IMPORTED_MODULE_4__,
                                duration: 100,
                            }
                        ),
                        title: (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_5__.jsxs)(
                            react_jsx_runtime__WEBPACK_IMPORTED_MODULE_5__.Fragment,
                            {
                                children: [
                                    "Micky ",
                                    (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_5__.jsx)("b", {
                                        children: "Mouse",
                                    }),
                                ],
                            }
                        ),
                        description: (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_5__.jsxs)(
                            react_jsx_runtime__WEBPACK_IMPORTED_MODULE_5__.Fragment,
                            {
                                children: [
                                    "Candidate ",
                                    (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_5__.jsx)("b", {
                                        children: "description",
                                    }),
                                ],
                            }
                        ),
                        isActive: !0,
                        url: "https://google.com",
                    },
                    parameters: {viewport: {disable: !0}},
                },
                Hover = {
                    args: {
                        children: (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_5__.jsx)(
                            mui_image__WEBPACK_IMPORTED_MODULE_3__.Z,
                            {
                                src: _public_example_candidate_jpg__WEBPACK_IMPORTED_MODULE_4__,
                                duration: 100,
                            }
                        ),
                        title: "Micky Mouse",
                        description: "Candidate Description",
                        className: "hover",
                        isActive: !0,
                        url: "https://google.com",
                    },
                    parameters: {
                        pseudo: {hover: [".hover"], active: [".active"], focus: [".focus"]},
                        viewport: {disable: !0},
                    },
                },
                Active = {
                    args: {
                        children: (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_5__.jsx)(
                            mui_image__WEBPACK_IMPORTED_MODULE_3__.Z,
                            {
                                src: _public_example_candidate_jpg__WEBPACK_IMPORTED_MODULE_4__,
                                duration: 100,
                            }
                        ),
                        title: "Micky Mouse",
                        description: "Candidate Description",
                        className: "hover active",
                        isActive: !0,
                        url: "https://google.com",
                    },
                    parameters: {
                        pseudo: {hover: [".hover"], active: [".active"], focus: [".focus"]},
                        viewport: {disable: !0},
                    },
                },
                WriteInSimple = {
                    args: {title: "", description: "", isActive: !0, isWriteIn: !0},
                    parameters: {viewport: {disable: !0}},
                },
                WriteInInvalid = {
                    args: {
                        title: "",
                        description: "",
                        isActive: !0,
                        isWriteIn: !0,
                        writeInValue: "John Connor",
                        isInvalidWriteIn: !0,
                    },
                    parameters: {viewport: {disable: !0}},
                },
                WriteInFields = {
                    args: {title: "", description: "", isActive: !0, isWriteIn: !0},
                    parameters: {viewport: {disable: !0}},
                },
                InvalidVote = {
                    args: {title: "Micky Mouse", isActive: !0, isInvalidVote: !0, checked: !1},
                    parameters: {viewport: {disable: !0}},
                }
            ;(Primary.parameters = (0,
            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                    {},
                    Primary.parameters
                ),
                {},
                {
                    docs: (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                        (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                            {},
                            null === (_Primary$parameters = Primary.parameters) ||
                                void 0 === _Primary$parameters
                                ? void 0
                                : _Primary$parameters.docs
                        ),
                        {},
                        {
                            source: (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                                {
                                    originalSource:
                                        '{\n  args: {\n    children: <Image src={CandidateImg} duration={100} />,\n    title: "Micky Mouse",\n    description: "Candidate Description",\n    isActive: true,\n    checked: true,\n    url: "https://google.com"\n  },\n  parameters: {\n    viewport: {\n      disable: true\n    }\n  }\n}',
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
                (ReadOnly.parameters = (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                    (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                        {},
                        ReadOnly.parameters
                    ),
                    {},
                    {
                        docs: (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                            (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                                {},
                                null === (_ReadOnly$parameters = ReadOnly.parameters) ||
                                    void 0 === _ReadOnly$parameters
                                    ? void 0
                                    : _ReadOnly$parameters.docs
                            ),
                            {},
                            {
                                source: (0,
                                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                                    {
                                        originalSource:
                                            '{\n  args: {\n    children: <Image src={CandidateImg} duration={100} />,\n    title: "Micky Mouse",\n    description: "Candidate Description",\n    isActive: false,\n    checked: true,\n    url: "https://google.com"\n  },\n  parameters: {\n    viewport: {\n      disable: true\n    }\n  }\n}',
                                    },
                                    null === (_ReadOnly$parameters2 = ReadOnly.parameters) ||
                                        void 0 === _ReadOnly$parameters2 ||
                                        null ===
                                            (_ReadOnly$parameters3 = _ReadOnly$parameters2.docs) ||
                                        void 0 === _ReadOnly$parameters3
                                        ? void 0
                                        : _ReadOnly$parameters3.source
                                ),
                            }
                        ),
                    }
                )),
                (NoImage.parameters = (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                    (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                        {},
                        NoImage.parameters
                    ),
                    {},
                    {
                        docs: (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                            (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                                {},
                                null === (_NoImage$parameters = NoImage.parameters) ||
                                    void 0 === _NoImage$parameters
                                    ? void 0
                                    : _NoImage$parameters.docs
                            ),
                            {},
                            {
                                source: (0,
                                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                                    {
                                        originalSource:
                                            '{\n  args: {\n    title: "Micky Mouse",\n    description: "Candidate Description",\n    isActive: true,\n    url: "https://google.com"\n  },\n  parameters: {\n    viewport: {\n      disable: true\n    }\n  }\n}',
                                    },
                                    null === (_NoImage$parameters2 = NoImage.parameters) ||
                                        void 0 === _NoImage$parameters2 ||
                                        null ===
                                            (_NoImage$parameters2$ = _NoImage$parameters2.docs) ||
                                        void 0 === _NoImage$parameters2$
                                        ? void 0
                                        : _NoImage$parameters2$.source
                                ),
                            }
                        ),
                    }
                )),
                (NoDescription.parameters = (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                    (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                        {},
                        NoDescription.parameters
                    ),
                    {},
                    {
                        docs: (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                            (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                                {},
                                null === (_NoDescription$parame = NoDescription.parameters) ||
                                    void 0 === _NoDescription$parame
                                    ? void 0
                                    : _NoDescription$parame.docs
                            ),
                            {},
                            {
                                source: (0,
                                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                                    {
                                        originalSource:
                                            '{\n  args: {\n    children: <Image src={CandidateImg} duration={100} />,\n    title: "Micky Mouse",\n    isActive: true,\n    url: "https://google.com",\n    checked: false\n  },\n  parameters: {\n    viewport: {\n      disable: true\n    }\n  }\n}',
                                    },
                                    null === (_NoDescription$parame2 = NoDescription.parameters) ||
                                        void 0 === _NoDescription$parame2 ||
                                        null ===
                                            (_NoDescription$parame3 =
                                                _NoDescription$parame2.docs) ||
                                        void 0 === _NoDescription$parame3
                                        ? void 0
                                        : _NoDescription$parame3.source
                                ),
                            }
                        ),
                    }
                )),
                (OnlyTitle.parameters = (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                    (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                        {},
                        OnlyTitle.parameters
                    ),
                    {},
                    {
                        docs: (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                            (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                                {},
                                null === (_OnlyTitle$parameters = OnlyTitle.parameters) ||
                                    void 0 === _OnlyTitle$parameters
                                    ? void 0
                                    : _OnlyTitle$parameters.docs
                            ),
                            {},
                            {
                                source: (0,
                                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                                    {
                                        originalSource:
                                            '{\n  args: {\n    title: "Micky Mouse",\n    isActive: true,\n    checked: false\n  },\n  parameters: {\n    viewport: {\n      disable: true\n    }\n  }\n}',
                                    },
                                    null === (_OnlyTitle$parameters2 = OnlyTitle.parameters) ||
                                        void 0 === _OnlyTitle$parameters2 ||
                                        null ===
                                            (_OnlyTitle$parameters3 =
                                                _OnlyTitle$parameters2.docs) ||
                                        void 0 === _OnlyTitle$parameters3
                                        ? void 0
                                        : _OnlyTitle$parameters3.source
                                ),
                            }
                        ),
                    }
                )),
                (LongDescription.parameters = (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                    (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                        {},
                        LongDescription.parameters
                    ),
                    {},
                    {
                        docs: (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                            (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                                {},
                                null === (_LongDescription$para = LongDescription.parameters) ||
                                    void 0 === _LongDescription$para
                                    ? void 0
                                    : _LongDescription$para.docs
                            ),
                            {},
                            {
                                source: (0,
                                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                                    {
                                        originalSource:
                                            '{\n  args: {\n    children: <Image src={CandidateImg} duration={100} />,\n    title: "Micky Mouse",\n    description: "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat."\n  },\n  parameters: {\n    viewport: {\n      disable: true\n    }\n  }\n}',
                                    },
                                    null ===
                                        (_LongDescription$para2 = LongDescription.parameters) ||
                                        void 0 === _LongDescription$para2 ||
                                        null ===
                                            (_LongDescription$para3 =
                                                _LongDescription$para2.docs) ||
                                        void 0 === _LongDescription$para3
                                        ? void 0
                                        : _LongDescription$para3.source
                                ),
                            }
                        ),
                    }
                )),
                (LongTitle.parameters = (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                    (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                        {},
                        LongTitle.parameters
                    ),
                    {},
                    {
                        docs: (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                            (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                                {},
                                null === (_LongTitle$parameters = LongTitle.parameters) ||
                                    void 0 === _LongTitle$parameters
                                    ? void 0
                                    : _LongTitle$parameters.docs
                            ),
                            {},
                            {
                                source: (0,
                                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                                    {
                                        originalSource:
                                            '{\n  args: {\n    children: <Image src={CandidateImg} duration={100} />,\n    title: "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.",\n    description: "Candidate Description",\n    isActive: true,\n    url: "https://google.com"\n  },\n  parameters: {\n    viewport: {\n      disable: true\n    }\n  }\n}',
                                    },
                                    null === (_LongTitle$parameters2 = LongTitle.parameters) ||
                                        void 0 === _LongTitle$parameters2 ||
                                        null ===
                                            (_LongTitle$parameters3 =
                                                _LongTitle$parameters2.docs) ||
                                        void 0 === _LongTitle$parameters3
                                        ? void 0
                                        : _LongTitle$parameters3.source
                                ),
                            }
                        ),
                    }
                )),
                (WithHtml.parameters = (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                    (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                        {},
                        WithHtml.parameters
                    ),
                    {},
                    {
                        docs: (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                            (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                                {},
                                null === (_WithHtml$parameters = WithHtml.parameters) ||
                                    void 0 === _WithHtml$parameters
                                    ? void 0
                                    : _WithHtml$parameters.docs
                            ),
                            {},
                            {
                                source: (0,
                                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                                    {
                                        originalSource:
                                            '{\n  args: {\n    children: <Image src={CandidateImg} duration={100} />,\n    title: <>\n                Micky <b>Mouse</b>\n            </>,\n    description: <>\n                Candidate <b>description</b>\n            </>,\n    isActive: true,\n    url: "https://google.com"\n  },\n  parameters: {\n    viewport: {\n      disable: true\n    }\n  }\n}',
                                    },
                                    null === (_WithHtml$parameters2 = WithHtml.parameters) ||
                                        void 0 === _WithHtml$parameters2 ||
                                        null ===
                                            (_WithHtml$parameters3 = _WithHtml$parameters2.docs) ||
                                        void 0 === _WithHtml$parameters3
                                        ? void 0
                                        : _WithHtml$parameters3.source
                                ),
                            }
                        ),
                    }
                )),
                (Hover.parameters = (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                    (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                        {},
                        Hover.parameters
                    ),
                    {},
                    {
                        docs: (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                            (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                                {},
                                null === (_Hover$parameters = Hover.parameters) ||
                                    void 0 === _Hover$parameters
                                    ? void 0
                                    : _Hover$parameters.docs
                            ),
                            {},
                            {
                                source: (0,
                                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                                    {
                                        originalSource:
                                            '{\n  args: {\n    children: <Image src={CandidateImg} duration={100} />,\n    title: "Micky Mouse",\n    description: "Candidate Description",\n    className: "hover",\n    isActive: true,\n    url: "https://google.com"\n  },\n  parameters: {\n    pseudo: {\n      hover: [".hover"],\n      active: [".active"],\n      focus: [".focus"]\n    },\n    viewport: {\n      disable: true\n    }\n  }\n}',
                                    },
                                    null === (_Hover$parameters2 = Hover.parameters) ||
                                        void 0 === _Hover$parameters2 ||
                                        null ===
                                            (_Hover$parameters2$do = _Hover$parameters2.docs) ||
                                        void 0 === _Hover$parameters2$do
                                        ? void 0
                                        : _Hover$parameters2$do.source
                                ),
                            }
                        ),
                    }
                )),
                (Active.parameters = (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                    (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                        {},
                        Active.parameters
                    ),
                    {},
                    {
                        docs: (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                            (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                                {},
                                null === (_Active$parameters = Active.parameters) ||
                                    void 0 === _Active$parameters
                                    ? void 0
                                    : _Active$parameters.docs
                            ),
                            {},
                            {
                                source: (0,
                                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                                    {
                                        originalSource:
                                            '{\n  args: {\n    children: <Image src={CandidateImg} duration={100} />,\n    title: "Micky Mouse",\n    description: "Candidate Description",\n    className: "hover active",\n    isActive: true,\n    url: "https://google.com"\n  },\n  parameters: {\n    pseudo: {\n      hover: [".hover"],\n      active: [".active"],\n      focus: [".focus"]\n    },\n    viewport: {\n      disable: true\n    }\n  }\n}',
                                    },
                                    null === (_Active$parameters2 = Active.parameters) ||
                                        void 0 === _Active$parameters2 ||
                                        null ===
                                            (_Active$parameters2$d = _Active$parameters2.docs) ||
                                        void 0 === _Active$parameters2$d
                                        ? void 0
                                        : _Active$parameters2$d.source
                                ),
                            }
                        ),
                    }
                )),
                (WriteInSimple.parameters = (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                    (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                        {},
                        WriteInSimple.parameters
                    ),
                    {},
                    {
                        docs: (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                            (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                                {},
                                null === (_WriteInSimple$parame = WriteInSimple.parameters) ||
                                    void 0 === _WriteInSimple$parame
                                    ? void 0
                                    : _WriteInSimple$parame.docs
                            ),
                            {},
                            {
                                source: (0,
                                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                                    {
                                        originalSource:
                                            '{\n  args: {\n    title: "",\n    description: "",\n    isActive: true,\n    isWriteIn: true\n  },\n  parameters: {\n    viewport: {\n      disable: true\n    }\n  }\n}',
                                    },
                                    null === (_WriteInSimple$parame2 = WriteInSimple.parameters) ||
                                        void 0 === _WriteInSimple$parame2 ||
                                        null ===
                                            (_WriteInSimple$parame3 =
                                                _WriteInSimple$parame2.docs) ||
                                        void 0 === _WriteInSimple$parame3
                                        ? void 0
                                        : _WriteInSimple$parame3.source
                                ),
                            }
                        ),
                    }
                )),
                (WriteInInvalid.parameters = (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                    (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                        {},
                        WriteInInvalid.parameters
                    ),
                    {},
                    {
                        docs: (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                            (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                                {},
                                null === (_WriteInInvalid$param = WriteInInvalid.parameters) ||
                                    void 0 === _WriteInInvalid$param
                                    ? void 0
                                    : _WriteInInvalid$param.docs
                            ),
                            {},
                            {
                                source: (0,
                                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                                    {
                                        originalSource:
                                            '{\n  args: {\n    title: "",\n    description: "",\n    isActive: true,\n    isWriteIn: true,\n    writeInValue: "John Connor",\n    isInvalidWriteIn: true\n  },\n  parameters: {\n    viewport: {\n      disable: true\n    }\n  }\n}',
                                    },
                                    null === (_WriteInInvalid$param2 = WriteInInvalid.parameters) ||
                                        void 0 === _WriteInInvalid$param2 ||
                                        null ===
                                            (_WriteInInvalid$param3 =
                                                _WriteInInvalid$param2.docs) ||
                                        void 0 === _WriteInInvalid$param3
                                        ? void 0
                                        : _WriteInInvalid$param3.source
                                ),
                            }
                        ),
                    }
                )),
                (WriteInFields.parameters = (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                    (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                        {},
                        WriteInFields.parameters
                    ),
                    {},
                    {
                        docs: (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                            (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                                {},
                                null === (_WriteInFields$parame = WriteInFields.parameters) ||
                                    void 0 === _WriteInFields$parame
                                    ? void 0
                                    : _WriteInFields$parame.docs
                            ),
                            {},
                            {
                                source: (0,
                                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                                    {
                                        originalSource:
                                            '{\n  args: {\n    title: "",\n    description: "",\n    isActive: true,\n    isWriteIn: true\n  },\n  parameters: {\n    viewport: {\n      disable: true\n    }\n  }\n}',
                                    },
                                    null === (_WriteInFields$parame2 = WriteInFields.parameters) ||
                                        void 0 === _WriteInFields$parame2 ||
                                        null ===
                                            (_WriteInFields$parame3 =
                                                _WriteInFields$parame2.docs) ||
                                        void 0 === _WriteInFields$parame3
                                        ? void 0
                                        : _WriteInFields$parame3.source
                                ),
                            }
                        ),
                    }
                )),
                (InvalidVote.parameters = (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                    (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                        {},
                        InvalidVote.parameters
                    ),
                    {},
                    {
                        docs: (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                            (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                                {},
                                null === (_InvalidVote$paramete = InvalidVote.parameters) ||
                                    void 0 === _InvalidVote$paramete
                                    ? void 0
                                    : _InvalidVote$paramete.docs
                            ),
                            {},
                            {
                                source: (0,
                                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                                    {
                                        originalSource:
                                            '{\n  args: {\n    title: "Micky Mouse",\n    isActive: true,\n    isInvalidVote: true,\n    checked: false\n  },\n  parameters: {\n    viewport: {\n      disable: true\n    }\n  }\n}',
                                    },
                                    null === (_InvalidVote$paramete2 = InvalidVote.parameters) ||
                                        void 0 === _InvalidVote$paramete2 ||
                                        null ===
                                            (_InvalidVote$paramete3 =
                                                _InvalidVote$paramete2.docs) ||
                                        void 0 === _InvalidVote$paramete3
                                        ? void 0
                                        : _InvalidVote$paramete3.source
                                ),
                            }
                        ),
                    }
                ))
            var __namedExportsOrder = [
                "Primary",
                "ReadOnly",
                "NoImage",
                "NoDescription",
                "OnlyTitle",
                "LongDescription",
                "LongTitle",
                "WithHtml",
                "Hover",
                "Active",
                "WriteInSimple",
                "WriteInInvalid",
                "WriteInFields",
                "InvalidVote",
            ]
        },
        "./public/example_candidate.jpg": function (
            module,
            __unused_webpack_exports,
            __webpack_require__
        ) {
            module.exports = __webpack_require__.p + "c9db05ee3210e07b4630.jpg"
        },
    },
])
