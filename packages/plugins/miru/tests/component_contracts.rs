// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! Execute the real compiled component. Only imported authorization is synthetic.
use wasmtime::component::{Component, ComponentType, Lift, Linker, ResourceTable};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

#[derive(ComponentType, Lift, Debug, PartialEq)]
#[component(record)]
struct Route {
    path: String,
    handler: String,
    #[component(name = "process-as-task")]
    process_as_task: bool,
}
#[derive(ComponentType, Lift, Debug)]
#[component(record)]
struct Manifest {
    #[component(name = "plugin-name")]
    plugin_name: String,
    hooks: Vec<String>,
    routes: Vec<Route>,
    tasks: Vec<String>,
}
#[derive(Debug, PartialEq)]
struct Authorization {
    claims: String,
    allow_super_admin: bool,
    tenant: Option<String>,
    permissions: Vec<String>,
}
struct Host {
    wasi: WasiCtx,
    table: ResourceTable,
    calls: Vec<Authorization>,
    reject: bool,
}
impl WasiView for Host {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}
fn component() -> (Store<Host>, wasmtime::component::Instance) {
    let path = std::env::var("MIRU_TEST_COMPONENT")
        .expect("build miru for wasm32-wasip2 and set MIRU_TEST_COMPONENT");
    let mut config = Config::new();
    config.wasm_component_model(true).consume_fuel(true);
    let engine = Engine::new(&config).unwrap();
    let component = Component::from_file(&engine, path).unwrap();
    let mut linker = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker).unwrap();
    linker
        .instance("plugins-manager:jwt/authorization")
        .unwrap()
        .func_wrap(
            "authorize",
            |mut store: wasmtime::StoreContextMut<'_, Host>,
             (claims, allow_super_admin, tenant, permissions): (
                String,
                bool,
                Option<String>,
                Vec<String>,
            )| {
                store.data_mut().calls.push(Authorization {
                    claims,
                    allow_super_admin,
                    tenant,
                    permissions,
                });
                Ok((if store.data().reject {
                    Err("synthetic permission denied".to_owned())
                } else {
                    Ok(())
                },))
            },
        )
        .unwrap();
    // No inherited environment, filesystem directories, sockets or credentials.
    let mut store = Store::new(
        &engine,
        Host {
            wasi: WasiCtx::builder().build(),
            table: ResourceTable::new(),
            calls: Vec::new(),
            reject: false,
        },
    );
    store.set_fuel(10_000_000).unwrap();
    let instance = linker.instantiate(&mut store, &component).unwrap();
    (store, instance)
}

#[test]
fn authorized_call_passes_the_literal_host_contract_and_propagates_denial() {
    let (mut store, instance) = component();
    let call = instance
        .get_typed_func::<(String,), (Result<String, String>,)>(
            &mut store,
            "create-transmission-package",
        )
        .unwrap();
    let result = call
        .call(&mut store, (r#"{"claims":"synthetic-claims"}"#.into(),))
        .unwrap()
        .0
        .unwrap();
    assert_eq!(
        result,
        r#"{"data":"Transmission package created successfully"}"#
    );
    assert_eq!(
        store.data().calls,
        vec![Authorization {
            claims: "synthetic-claims".into(),
            allow_super_admin: true,
            tenant: Some("90505c8a-23a9-4cdf-a26b-4e19f6a097d5".into()),
            permissions: vec!["admin-user".into()]
        }]
    );
    store.data_mut().reject = true;
    let result = call
        .call(&mut store, (r#"{"claims":"synthetic-claims"}"#.into(),))
        .unwrap()
        .0;
    assert_eq!(
        result.unwrap_err(),
        "Error creating transmission package: synthetic permission denied"
    );
    assert_eq!(store.data().calls.len(), 2);
}

#[test]
fn malformed_input_is_rejected_before_authorization() {
    let (mut store, instance) = component();
    let call = instance
        .get_typed_func::<(String,), (Result<String, String>,)>(
            &mut store,
            "create-transmission-package",
        )
        .unwrap();
    for (input, expected) in [
        ("not JSON", "Error parsed input as json value:"),
        ("{}", "Error get claims"),
        (r#"{"claims":null}"#, "Error parsed claims as str"),
        (r#"{"claims":12}"#, "Error parsed claims as str"),
    ] {
        let error = call
            .call(&mut store, (input.into(),))
            .unwrap()
            .0
            .unwrap_err();
        assert!(error.starts_with(expected), "{error}");
    }
    assert!(store.data().calls.is_empty());
    let valid = call
        .call(&mut store, (r#"{"claims":"synthetic-control"}"#.into(),))
        .unwrap()
        .0;
    assert!(valid.is_ok());
    assert_eq!(store.data().calls.len(), 1);
}

#[test]
fn manifest_exposes_the_expected_task_route_and_hook() {
    let (mut store, instance) = component();
    let interface = instance
        .get_export_index(&mut store, None, "plugins-manager:common/plugin-common")
        .unwrap();
    let export = instance
        .get_export_index(&mut store, Some(&interface), "get-manifest")
        .unwrap();
    let call = instance
        .get_typed_func::<(), (Manifest,)>(&mut store, &export)
        .unwrap();
    let manifest = call.call(&mut store, ()).unwrap().0;
    assert_eq!(manifest.plugin_name, "miru");
    assert_eq!(manifest.hooks, ["create-transmission-package"]);
    assert_eq!(
        manifest.routes,
        vec![Route {
            path: "/miru/create-transmission-package".into(),
            handler: "create-transmission-package".into(),
            process_as_task: true
        }]
    );
    assert!(manifest.tasks.is_empty());
    assert!(store.data().calls.is_empty());
}
