(self.webpackChunk_sequentech_ui_essentials =
    self.webpackChunk_sequentech_ui_essentials || []).push([
    [7557],
    {
        "../node_modules/lodash/_arrayIncludes.js": function (
            module,
            __unused_webpack_exports,
            __webpack_require__
        ) {
            var baseIndexOf = __webpack_require__("../node_modules/lodash/_baseIndexOf.js")
            module.exports = function arrayIncludes(array, value) {
                return !!(null == array ? 0 : array.length) && baseIndexOf(array, value, 0) > -1
            }
        },
        "../node_modules/lodash/_arrayIncludesWith.js": function (module) {
            module.exports = function arrayIncludesWith(array, value, comparator) {
                for (var index = -1, length = null == array ? 0 : array.length; ++index < length; )
                    if (comparator(value, array[index])) return !0
                return !1
            }
        },
        "../node_modules/lodash/_baseFindIndex.js": function (module) {
            module.exports = function baseFindIndex(array, predicate, fromIndex, fromRight) {
                for (
                    var length = array.length, index = fromIndex + (fromRight ? 1 : -1);
                    fromRight ? index-- : ++index < length;

                )
                    if (predicate(array[index], index, array)) return index
                return -1
            }
        },
        "../node_modules/lodash/_baseIndexOf.js": function (
            module,
            __unused_webpack_exports,
            __webpack_require__
        ) {
            var baseFindIndex = __webpack_require__("../node_modules/lodash/_baseFindIndex.js"),
                baseIsNaN = __webpack_require__("../node_modules/lodash/_baseIsNaN.js"),
                strictIndexOf = __webpack_require__("../node_modules/lodash/_strictIndexOf.js")
            module.exports = function baseIndexOf(array, value, fromIndex) {
                return value == value
                    ? strictIndexOf(array, value, fromIndex)
                    : baseFindIndex(array, baseIsNaN, fromIndex)
            }
        },
        "../node_modules/lodash/_baseIsNaN.js": function (module) {
            module.exports = function baseIsNaN(value) {
                return value != value
            }
        },
        "../node_modules/lodash/_baseUniq.js": function (
            module,
            __unused_webpack_exports,
            __webpack_require__
        ) {
            var SetCache = __webpack_require__("../node_modules/lodash/_SetCache.js"),
                arrayIncludes = __webpack_require__("../node_modules/lodash/_arrayIncludes.js"),
                arrayIncludesWith = __webpack_require__(
                    "../node_modules/lodash/_arrayIncludesWith.js"
                ),
                cacheHas = __webpack_require__("../node_modules/lodash/_cacheHas.js"),
                createSet = __webpack_require__("../node_modules/lodash/_createSet.js"),
                setToArray = __webpack_require__("../node_modules/lodash/_setToArray.js")
            module.exports = function baseUniq(array, iteratee, comparator) {
                var index = -1,
                    includes = arrayIncludes,
                    length = array.length,
                    isCommon = !0,
                    result = [],
                    seen = result
                if (comparator) (isCommon = !1), (includes = arrayIncludesWith)
                else if (length >= 200) {
                    var set = iteratee ? null : createSet(array)
                    if (set) return setToArray(set)
                    ;(isCommon = !1), (includes = cacheHas), (seen = new SetCache())
                } else seen = iteratee ? [] : result
                outer: for (; ++index < length; ) {
                    var value = array[index],
                        computed = iteratee ? iteratee(value) : value
                    if (
                        ((value = comparator || 0 !== value ? value : 0),
                        isCommon && computed == computed)
                    ) {
                        for (var seenIndex = seen.length; seenIndex--; )
                            if (seen[seenIndex] === computed) continue outer
                        iteratee && seen.push(computed), result.push(value)
                    } else
                        includes(seen, computed, comparator) ||
                            (seen !== result && seen.push(computed), result.push(value))
                }
                return result
            }
        },
        "../node_modules/lodash/_createSet.js": function (
            module,
            __unused_webpack_exports,
            __webpack_require__
        ) {
            var Set = __webpack_require__("../node_modules/lodash/_Set.js"),
                noop = __webpack_require__("../node_modules/lodash/noop.js"),
                setToArray = __webpack_require__("../node_modules/lodash/_setToArray.js"),
                createSet =
                    Set && 1 / setToArray(new Set([, -0]))[1] == 1 / 0
                        ? function (values) {
                              return new Set(values)
                          }
                        : noop
            module.exports = createSet
        },
        "../node_modules/lodash/_strictIndexOf.js": function (module) {
            module.exports = function strictIndexOf(array, value, fromIndex) {
                for (var index = fromIndex - 1, length = array.length; ++index < length; )
                    if (array[index] === value) return index
                return -1
            }
        },
        "../node_modules/lodash/noop.js": function (module) {
            module.exports = function noop() {}
        },
        "../node_modules/lodash/uniq.js": function (
            module,
            __unused_webpack_exports,
            __webpack_require__
        ) {
            var baseUniq = __webpack_require__("../node_modules/lodash/_baseUniq.js")
            module.exports = function uniq(array) {
                return array && array.length ? baseUniq(array) : []
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
