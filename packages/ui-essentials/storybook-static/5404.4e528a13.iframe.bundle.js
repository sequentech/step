;(self.webpackChunk_sequentech_ui_essentials =
    self.webpackChunk_sequentech_ui_essentials || []).push([
    [5404],
    {
        "./node_modules/storybook-addon-react-router-v6/dist/index.mjs": function (
            __unused_webpack_module,
            __webpack_exports__,
            __webpack_require__
        ) {
            __webpack_require__.d(__webpack_exports__, {
                E: function () {
                    return Dt
                },
            })
            var router_Action,
                react = __webpack_require__("../node_modules/react/index.js"),
                react_namespaceObject = __webpack_require__.t(react, 2),
                external_STORYBOOK_MODULE_PREVIEW_API_ = __webpack_require__(
                    "@storybook/preview-api"
                ),
                dist = __webpack_require__(
                    "../node_modules/react-router-dom/node_modules/react-router/dist/index.js"
                ),
                react_router_dom_dist = __webpack_require__(
                    "../node_modules/react-router-dom/dist/index.js"
                ),
                router = __webpack_require__("../node_modules/@remix-run/router/dist/router.js")
            !(function (Action) {
                ;(Action.Pop = "POP"), (Action.Push = "PUSH"), (Action.Replace = "REPLACE")
            })(router_Action || (router_Action = {}))
            var ResultType
            !(function (ResultType) {
                ;(ResultType.data = "data"),
                    (ResultType.deferred = "deferred"),
                    (ResultType.redirect = "redirect"),
                    (ResultType.error = "error")
            })(ResultType || (ResultType = {}))
            new Set(["lazy", "caseSensitive", "path", "id", "index", "children"])
            Error
            const validMutationMethodsArr = ["post", "put", "patch", "delete"],
                validRequestMethodsArr =
                    (new Set(validMutationMethodsArr), ["get", ...validMutationMethodsArr])
            new Set(validRequestMethodsArr), new Set([301, 302, 303, 307, 308]), new Set([307, 308])
            Symbol("deferred")
            const RouteContext = react.createContext({outlet: null, matches: [], isDataRoute: !1})
            react.Component
            react_namespaceObject.startTransition
            new Promise(() => {})
            react.Component
            var external_STORYBOOK_MODULE_CORE_EVENTS_ = __webpack_require__(
                    "@storybook/core-events"
                ),
                h = "storybook/react-router-v6",
                o = {
                    CLEAR: `${h}/clear`,
                    NAVIGATION: `${h}/navigation`,
                    STORY_LOADED: `${h}/story-loaded`,
                    ROUTE_MATCHES: `${h}/route-matches`,
                    ACTION_INVOKED: `${h}/action_invoked`,
                    ACTION_SETTLED: `${h}/action_settled`,
                    LOADER_INVOKED: `${h}/loader_invoked`,
                    LOADER_SETTLED: `${h}/loader_settled`,
                },
                P = react.createContext([]),
                A = () => react.useContext(P),
                F = () => {
                    let r = (0, react.useRef)(0),
                        e = (0, dist.TH)(),
                        t = (0, dist.UO)(),
                        [a] = (0, react_router_dom_dist.lr)(),
                        n = (0, dist.ur)(),
                        s = A(),
                        p = {}
                    a.forEach((u, i) => {
                        p[i] = u
                    })
                    let c = (() => {
                            let r = (0, dist.TH)()
                            return `${r.pathname}${r.search}${r.hash}`
                        })(),
                        m = s.map((u) => [u.route.path, u.params])
                    return (u) => {
                        switch (u) {
                            case o.STORY_LOADED: {
                                let i = {
                                    url: c,
                                    path: e.pathname,
                                    routeParams: t,
                                    searchParams: p,
                                    routeMatches: m,
                                    hash: e.hash,
                                    routeState: e.state,
                                }
                                return {
                                    key: `${o.STORY_LOADED}_${r.current++}`,
                                    type: o.STORY_LOADED,
                                    data: i,
                                }
                            }
                            case o.NAVIGATION: {
                                let i = {
                                    url: c,
                                    path: e.pathname,
                                    routeParams: t,
                                    searchParams: p,
                                    hash: e.hash,
                                    routeState: e.state,
                                    routeMatches: m,
                                    navigationType: n,
                                }
                                return {
                                    key: `${o.NAVIGATION}_${r.current++}`,
                                    type: o.NAVIGATION,
                                    data: i,
                                }
                            }
                            case o.ROUTE_MATCHES:
                                return {
                                    key: `${o.ROUTE_MATCHES}_${r.current++}`,
                                    type: o.ROUTE_MATCHES,
                                    data: {matches: m},
                                }
                        }
                    }
                }
            async function v(r) {
                let a,
                    e = r.clone(),
                    t = e.headers.get("content-type") || ""
                switch (!0) {
                    case t.startsWith("text"):
                        a = await e.text()
                        break
                    case t.startsWith("application/json"):
                        a = await e.json()
                        break
                    case t.startsWith("multipart/form-data"):
                    case t.startsWith("application/x-www-form-urlencoded"):
                        a = (function tt(r) {
                            let e = {}
                            return (
                                r.forEach((t, a) => {
                                    t instanceof File
                                        ? (e[a] = {
                                              filename: t.name,
                                              filesize: t.size,
                                              filetype: t.type,
                                          })
                                        : (e[a] = t)
                                }),
                                e
                            )
                        })(await e.formData())
                        break
                    default:
                        await e.arrayBuffer().then((s) => s.byteLength)
                }
                return a
            }
            var $ = ({children: r}) => {
                    let e = external_STORYBOOK_MODULE_PREVIEW_API_.addons.getChannel(),
                        t = (0, dist.TH)(),
                        [a, n] = (0, react.useState)(),
                        [s, p] = (0, react.useState)(!1),
                        [c, m] = (0, react.useState)([]),
                        u = F(),
                        i = A(),
                        E = (0, react.useRef)(
                            (function k() {
                                let r = {}
                                return (
                                    (r.promise = new Promise((e, t) => {
                                        ;(r.resolve = e), (r.reject = t)
                                    })),
                                    r
                                )
                            })()
                        )
                    return (
                        (0, react.useLayoutEffect)(() => {
                            n(t)
                        }),
                        (0, react.useEffect)(() => {
                            s && E.current.resolve()
                        }, [s]),
                        (0, react.useEffect)(() => {
                            m(i)
                            let l = setTimeout(() => {
                                s || (p(!0), e.emit(o.STORY_LOADED, u(o.STORY_LOADED)))
                            }, 0)
                            return () => clearTimeout(l)
                        }, [s, i]),
                        (0, react.useEffect)(() => {
                            void 0 !== a &&
                                a.key !== t.key &&
                                E.current.promise.then(() => {
                                    e.emit(o.NAVIGATION, u(o.NAVIGATION))
                                })
                        }, [t]),
                        (0, react.useEffect)(() => {
                            s &&
                                i.length > c.length &&
                                (m(i), e.emit(o.ROUTE_MATCHES, u(o.ROUTE_MATCHES)))
                        }, [i]),
                        react.createElement(react.Fragment, null, r)
                    )
                },
                H = ({
                    children: r,
                    routePath: e,
                    routeParams: t,
                    searchParams: a,
                    routeState: n,
                    browserPath: s,
                    hydrationData: p,
                }) => {
                    let c = external_STORYBOOK_MODULE_PREVIEW_API_.addons.getChannel(),
                        [m, u] = (0, react.useState)(),
                        [i, E] = (0, react.useState)(0),
                        l = (0, react.useRef)()
                    return (
                        c.on(external_STORYBOOK_MODULE_CORE_EVENTS_.STORY_ARGS_UPDATED, () => {
                            E((R) => R + 1)
                        }),
                        (0, react.useLayoutEffect)(() => {
                            let R = (0, router.Gn)(e || "", t),
                                f = new URLSearchParams(a).toString(),
                                y = {search: f.length > 0 ? `?${f}` : "", state: n}
                            void 0 !== s && (y.pathname = s),
                                void 0 === s && "" !== R && (y.pathname = R),
                                void 0 !== l.current && Object.assign(y, l.current)
                            let d = (0, dist.i7)(r),
                                T = (0, dist.bi)(d, {initialEntries: [y], hydrationData: p})
                            T.subscribe((N) => {
                                l.current = N.location
                            }),
                                u(T)
                        }, [i]),
                        void 0 === m
                            ? null
                            : react.createElement(dist.pG, {
                                  router: m,
                                  fallbackElement: react.createElement(lt, null),
                              })
                    )
                }
            function lt() {
                return react.createElement("p", null, "Performing initial data load")
            }
            var _ = () => async (r, e) => {
                    switch (r) {
                        case o.ACTION_INVOKED: {
                            let {request: t, params: a, context: n} = e,
                                s = {url: t.url, method: t.method, body: await v(t)}
                            return {
                                type: o.ACTION_INVOKED,
                                data: {params: a, request: s, context: n},
                            }
                        }
                        case o.ACTION_SETTLED:
                            return {type: o.ACTION_SETTLED, data: e}
                        case o.LOADER_INVOKED: {
                            let {request: t, params: a, context: n} = e,
                                s = {url: t.url, method: t.method, body: v(t)}
                            return {
                                type: o.LOADER_INVOKED,
                                data: {params: a, request: s, context: n},
                            }
                        }
                        case o.LOADER_SETTLED:
                            return {type: o.LOADER_SETTLED, data: e}
                    }
                },
                Y = ({
                    children: r,
                    browserPath: e,
                    routePath: t = "*",
                    routeParams: a,
                    routeHandle: n,
                    searchParams: s,
                    routeState: p,
                    outlet: c,
                    hydrationData: m,
                    action: u,
                    loader: i,
                    errorElement: E,
                    shouldRevalidate: l,
                    routeId: R,
                }) => {
                    let f = external_STORYBOOK_MODULE_PREVIEW_API_.addons.getChannel(),
                        [g, y] = (0, react.useState)([])
                    RouteContext.Provider._context = new Proxy(
                        RouteContext.Provider._context ?? {},
                        {
                            set(N, w, S) {
                                return (
                                    "_currentValue" === w &&
                                        y((I) =>
                                            void 0 !== S && S.matches.length > I.length
                                                ? S.matches
                                                : I
                                        ),
                                    Reflect.set(N, w, S)
                                )
                            },
                        }
                    )
                    let d = (function Et(r) {
                            return (
                                null !== r &&
                                "object" == typeof r &&
                                Object.prototype.hasOwnProperty.call(r, "element")
                            )
                        })(c)
                            ? c
                            : {element: c},
                        T = {
                            element: d.element,
                            handle: d.handle,
                            errorElement: d.errorElement,
                            action: void 0 !== d.action ? K(f, d.action) : void 0,
                            loader: void 0 !== d.loader ? q(f, d.loader) : void 0,
                        }
                    return react.createElement(
                        P.Provider,
                        {value: g},
                        react.createElement(
                            H,
                            {
                                routePath: t,
                                routeParams: a,
                                routeState: p,
                                searchParams: s,
                                browserPath: e,
                                hydrationData: m,
                            },
                            react.createElement(
                                dist.AW,
                                {
                                    id: R,
                                    path: t,
                                    handle: n,
                                    action: void 0 !== u ? K(f, u) : void 0,
                                    loader: void 0 !== i ? q(f, i) : void 0,
                                    shouldRevalidate: l,
                                    errorElement: E,
                                    element: react.createElement($, null, r),
                                },
                                void 0 !== d.element &&
                                    void 0 === d.path &&
                                    react.createElement(dist.AW, {index: !0, ...T}),
                                void 0 !== d.element &&
                                    react.createElement(dist.AW, {path: d.path, ...T})
                            )
                        )
                    )
                }
            function K(r, e) {
                let t = _()
                return async function (a) {
                    if (void 0 === e) return
                    r.emit(o.ACTION_INVOKED, await t(o.ACTION_INVOKED, a))
                    let n = await e(a)
                    return r.emit(o.ACTION_SETTLED, await t(o.ACTION_SETTLED, n)), n
                }
            }
            function q(r, e) {
                let t = _()
                return async function (a) {
                    if (void 0 === e) return
                    r.emit(o.LOADER_INVOKED, await t(o.LOADER_INVOKED, a))
                    let n = await e(a)
                    return r.emit(o.LOADER_SETTLED, await t(o.LOADER_SETTLED, n)), n
                }
            }
            var Dt = (0, external_STORYBOOK_MODULE_PREVIEW_API_.makeDecorator)({
                name: "withRouter",
                parameterName: "reactRouter",
                wrapper: (r, e, {parameters: t = {}}) => {
                    let {
                        routePath: a = "*",
                        routeParams: n,
                        routeState: s,
                        routeHandle: p,
                        searchParams: c,
                        outlet: m,
                        browserPath: u,
                        loader: i,
                        action: E,
                        errorElement: l,
                        hydrationData: R,
                        shouldRevalidate: f,
                        routeId: g,
                    } = t
                    if ("string" != typeof a)
                        throw new Error("React Router decorator : `path` must be a string")
                    if (void 0 !== n && "object" != typeof n)
                        throw new Error(
                            "React Router decorator : `params` must be an object with strings as values"
                        )
                    if (void 0 !== c && "object" != typeof c)
                        throw new Error(
                            "React Router decorator : `search` must be an object with strings as values"
                        )
                    return react.createElement(
                        Y,
                        {
                            browserPath: u,
                            routePath: a,
                            routeParams: n,
                            searchParams: c,
                            routeState: s,
                            routeHandle: p,
                            outlet: m,
                            loader: i,
                            action: E,
                            errorElement: l,
                            hydrationData: R,
                            shouldRevalidate: f,
                            routeId: g,
                        },
                        r(e)
                    )
                },
            })
        },
    },
])
