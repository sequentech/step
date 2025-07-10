(self.webpackChunk_sequentech_ui_essentials =
    self.webpackChunk_sequentech_ui_essentials || []).push([
    [6965],
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
        "./src/components/CustomDropFile/__stories__/CustomDropFile.mdx": function (
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
                _CustomDropFile_stories__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(
                    "./src/components/CustomDropFile/__stories__/CustomDropFile.stories.tsx"
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
                                    of: _CustomDropFile_stories__WEBPACK_IMPORTED_MODULE_2__,
                                    title: "components/CustomDropFile",
                                }
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(
                                _components.h1,
                                {id: "customdropfile", children: "CustomDropFile"}
                            ),
                            "\n",
                            (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_1__.jsx)(_components.p, {
                                children:
                                    "A CustomDropFile is the box at the top of every screen. It includes the logo, the\nsoftware version, language toggle and an optional button to log out.",
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
                                        _CustomDropFile_stories__WEBPACK_IMPORTED_MODULE_2__.BasicDropFile,
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
        "./src/components/CustomDropFile/__stories__/CustomDropFile.stories.tsx": function (
            __unused_webpack_module,
            __webpack_exports__,
            __webpack_require__
        ) {
            __webpack_require__.r(__webpack_exports__),
                __webpack_require__.d(__webpack_exports__, {
                    BasicDropFile: function () {
                        return BasicDropFile
                    },
                    WithButtonDropFile: function () {
                        return WithButtonDropFile
                    },
                    __namedExportsOrder: function () {
                        return __namedExportsOrder
                    },
                })
            var _templateObject,
                _BasicDropFile$parame,
                _BasicDropFile$parame2,
                _BasicDropFile$parame3,
                _WithButtonDropFile$p,
                _WithButtonDropFile$p2,
                _WithButtonDropFile$p3,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_regeneratorRuntime_js__WEBPACK_IMPORTED_MODULE_13__ = __webpack_require__(
                    "../node_modules/@babel/runtime/helpers/esm/regeneratorRuntime.js"
                ),
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_asyncToGenerator_js__WEBPACK_IMPORTED_MODULE_12__ = __webpack_require__(
                    "../node_modules/@babel/runtime/helpers/esm/asyncToGenerator.js"
                ),
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectWithoutProperties_js__WEBPACK_IMPORTED_MODULE_9__ = __webpack_require__(
                    "../node_modules/@babel/runtime/helpers/esm/objectWithoutProperties.js"
                ),
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__ = __webpack_require__(
                    "../node_modules/@babel/runtime/helpers/esm/objectSpread2.js"
                ),
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_taggedTemplateLiteral_js__WEBPACK_IMPORTED_MODULE_7__ = __webpack_require__(
                    "../node_modules/@babel/runtime/helpers/esm/taggedTemplateLiteral.js"
                ),
                react__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(
                    "../node_modules/react/index.js"
                ),
                _CustomDropFile__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(
                    "./src/components/CustomDropFile/CustomDropFile.tsx"
                ),
                _mui_material_Button__WEBPACK_IMPORTED_MODULE_11__ = __webpack_require__(
                    "../node_modules/@mui/material/Button/Button.js"
                ),
                _storybook_testing_library__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(
                    "../node_modules/@storybook/testing-library/dist/esm/index.js"
                ),
                _storybook_jest__WEBPACK_IMPORTED_MODULE_3__ = __webpack_require__(
                    "./node_modules/@storybook/jest/dist/esm/index.js"
                ),
                _mui_material_Box__WEBPACK_IMPORTED_MODULE_6__ = __webpack_require__(
                    "../node_modules/@mui/material/Box/Box.js"
                ),
                _mui_material_styles__WEBPACK_IMPORTED_MODULE_5__ = __webpack_require__(
                    "../node_modules/@mui/material/styles/styled.js"
                ),
                _mui_material_Paper__WEBPACK_IMPORTED_MODULE_10__ = __webpack_require__(
                    "../node_modules/@mui/material/Paper/Paper.js"
                ),
                react_jsx_runtime__WEBPACK_IMPORTED_MODULE_4__ = __webpack_require__(
                    "../node_modules/react/jsx-runtime.js"
                ),
                _excluded = ["text", "buttonText"],
                StyledBox = (0, _mui_material_styles__WEBPACK_IMPORTED_MODULE_5__.ZP)(
                    _mui_material_Box__WEBPACK_IMPORTED_MODULE_6__.Z
                )(
                    _templateObject ||
                        (_templateObject = (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_taggedTemplateLiteral_js__WEBPACK_IMPORTED_MODULE_7__.Z)(
                            [
                                "\n    height: 16rem;\n    width: 28rem;\n    max-width: 100%;\n    position: relative;\n",
                            ]
                        ))
                )
            __webpack_exports__.default = {
                title: "components/CustomDropFile",
                component: _CustomDropFile__WEBPACK_IMPORTED_MODULE_1__.Z,
                parameters: {backgrounds: {default: "white"}},
                argTypes: {},
            }
            var BasicDropFile = function BasicTemplate(args) {
                return (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_4__.jsx)(
                    _CustomDropFile__WEBPACK_IMPORTED_MODULE_1__.Z,
                    (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                        (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                            {},
                            args
                        ),
                        {},
                        {
                            children: (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_4__.jsx)(
                                StyledBox,
                                {}
                            ),
                        }
                    )
                )
            }.bind({})
            BasicDropFile.args = {handleFiles: function handleFiles(files) {}}
            var WithButtonDropFile = function WithButtonTemplate(_ref) {
                var text = _ref.text,
                    buttonText = _ref.buttonText,
                    args = (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectWithoutProperties_js__WEBPACK_IMPORTED_MODULE_9__.Z)(
                        _ref,
                        _excluded
                    ),
                    inputRef = (0, react__WEBPACK_IMPORTED_MODULE_0__.useRef)(null)
                return (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_4__.jsx)(
                    _CustomDropFile__WEBPACK_IMPORTED_MODULE_1__.Z,
                    (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                        (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                            {},
                            args
                        ),
                        {},
                        {
                            handleFiles: function handleFiles(files) {
                                alert("Number of files: " + files.length)
                            },
                            ref: inputRef,
                            children: (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_4__.jsxs)(
                                _mui_material_Paper__WEBPACK_IMPORTED_MODULE_10__.Z,
                                {
                                    variant: "responsive",
                                    children: [
                                        text,
                                        (0, react_jsx_runtime__WEBPACK_IMPORTED_MODULE_4__.jsx)(
                                            _mui_material_Button__WEBPACK_IMPORTED_MODULE_11__.Z,
                                            {
                                                "variant": "outlined",
                                                "onClick": function onButtonClick() {
                                                    var _inputRef$current
                                                    null ===
                                                        (_inputRef$current = inputRef.current) ||
                                                        void 0 === _inputRef$current ||
                                                        _inputRef$current.click()
                                                },
                                                "data-testid": "drop-file-button",
                                                "children": buttonText,
                                            }
                                        ),
                                    ],
                                }
                            ),
                        }
                    )
                )
            }.bind({})
            ;(WithButtonDropFile.args = {
                text: "Drop a file here or",
                buttonText: "Click to Upload",
            }),
                (WithButtonDropFile.play = (function () {
                    var _ref3 = (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_asyncToGenerator_js__WEBPACK_IMPORTED_MODULE_12__.Z)(
                        (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_regeneratorRuntime_js__WEBPACK_IMPORTED_MODULE_13__.Z)().mark(
                            function _callee(_ref2) {
                                var canvasElement, canvas, fakeFile, inputFile
                                return (0,
                                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_regeneratorRuntime_js__WEBPACK_IMPORTED_MODULE_13__.Z)().wrap(
                                    function _callee$(_context) {
                                        for (;;)
                                            switch ((_context.prev = _context.next)) {
                                                case 0:
                                                    (canvasElement = _ref2.canvasElement),
                                                        (canvas = (0,
                                                        _storybook_testing_library__WEBPACK_IMPORTED_MODULE_2__.uh)(
                                                            canvasElement
                                                        )),
                                                        (fakeFile = new File(
                                                            ["hello"],
                                                            "hello.png",
                                                            {type: "image/png"}
                                                        )),
                                                        (inputFile = canvas.getByTestId(
                                                            "drop-input-file"
                                                        )),
                                                        _storybook_testing_library__WEBPACK_IMPORTED_MODULE_2__.mV.upload(
                                                            inputFile,
                                                            fakeFile
                                                        ),
                                                        (0,
                                                        _storybook_jest__WEBPACK_IMPORTED_MODULE_3__.l)(
                                                            inputFile.files
                                                        ).toHaveLength(1),
                                                        (0,
                                                        _storybook_jest__WEBPACK_IMPORTED_MODULE_3__.l)(
                                                            inputFile.files[0]
                                                        ).toStrictEqual(fakeFile),
                                                        (0,
                                                        _storybook_jest__WEBPACK_IMPORTED_MODULE_3__.l)(
                                                            inputFile.files.item(0)
                                                        ).toStrictEqual(fakeFile)
                                                case 8:
                                                case "end":
                                                    return _context.stop()
                                            }
                                    },
                                    _callee
                                )
                            }
                        )
                    )
                    return function (_x) {
                        return _ref3.apply(this, arguments)
                    }
                })()),
                (BasicDropFile.parameters = (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                    (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                        {},
                        BasicDropFile.parameters
                    ),
                    {},
                    {
                        docs: (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                            (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                                {},
                                null === (_BasicDropFile$parame = BasicDropFile.parameters) ||
                                    void 0 === _BasicDropFile$parame
                                    ? void 0
                                    : _BasicDropFile$parame.docs
                            ),
                            {},
                            {
                                source: (0,
                                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                                    {
                                        originalSource:
                                            "args => <CustomDropFile {...args}>\n        <StyledBox />\n    </CustomDropFile>",
                                    },
                                    null === (_BasicDropFile$parame2 = BasicDropFile.parameters) ||
                                        void 0 === _BasicDropFile$parame2 ||
                                        null ===
                                            (_BasicDropFile$parame3 =
                                                _BasicDropFile$parame2.docs) ||
                                        void 0 === _BasicDropFile$parame3
                                        ? void 0
                                        : _BasicDropFile$parame3.source
                                ),
                            }
                        ),
                    }
                )),
                (WithButtonDropFile.parameters = (0,
                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                    (0,
                    _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                        {},
                        WithButtonDropFile.parameters
                    ),
                    {},
                    {
                        docs: (0,
                        _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                            (0,
                            _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                                {},
                                null === (_WithButtonDropFile$p = WithButtonDropFile.parameters) ||
                                    void 0 === _WithButtonDropFile$p
                                    ? void 0
                                    : _WithButtonDropFile$p.docs
                            ),
                            {},
                            {
                                source: (0,
                                _workspaces_step_packages_node_modules_babel_runtime_helpers_esm_objectSpread2_js__WEBPACK_IMPORTED_MODULE_8__.Z)(
                                    {
                                        originalSource:
                                            '({\n  text,\n  buttonText,\n  ...args\n}) => {\n  const inputRef = useRef<HTMLInputElement | null>(null);\n  const handleFiles = (files: FileList) => {\n    alert("Number of files: " + files.length);\n  };\n\n  // triggers the input when the button is clicked\n  const onButtonClick = () => {\n    inputRef.current?.click();\n  };\n  return <CustomDropFile {...args} handleFiles={handleFiles} ref={inputRef}>\n            <Paper variant="responsive">\n                {text}\n                <Button variant="outlined" onClick={onButtonClick} data-testid="drop-file-button">\n                    {buttonText}\n                </Button>\n            </Paper>\n        </CustomDropFile>;\n}',
                                    },
                                    null ===
                                        (_WithButtonDropFile$p2 = WithButtonDropFile.parameters) ||
                                        void 0 === _WithButtonDropFile$p2 ||
                                        null ===
                                            (_WithButtonDropFile$p3 =
                                                _WithButtonDropFile$p2.docs) ||
                                        void 0 === _WithButtonDropFile$p3
                                        ? void 0
                                        : _WithButtonDropFile$p3.source
                                ),
                            }
                        ),
                    }
                ))
            var __namedExportsOrder = ["BasicDropFile", "WithButtonDropFile"]
        },
        "?d91c": function () {},
    },
])
