use deadpool_postgres::{Manager, Pool, Transaction};
use futures::future::BoxFuture;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use wasmtime::*;

use crate::services::database::get_hasura_pool;

struct MyState {
    pool: Arc<Pool>,
    tx_map: Mutex<HashMap<String, Transaction<'static>>>,
}

async fn create_plugin_instance() -> Result<Instance> {
    let engine = Engine::default();
    let module = Module::from_file(&engine, "examples/hello.wat")?;

    let pool = get_hasura_pool().await;
    let mut store = Store::new(
        &engine,
        MyState {
            pool,
            tx_map: Mutex::new(HashMap::new()),
        },
    );

    let hello_func = Func::wrap_async(
        &mut store,
        |mut caller: Caller<'_, MyState>, ()| -> BoxFuture<'static, Result<(), Trap>> {
            async move {
                let pool = caller.data().pool.clone();
                let client = pool.get().await.map_err(|e| Trap::new(e.to_string()))?;

                let tx = client
                    .transaction()
                    .await
                    .map_err(|e| Trap::new(e.to_string()))?;

                caller
                    .data()
                    .tx_map
                    .lock()
                    .unwrap()
                    .insert("id".to_string(), tx); // TODO: need to generate unique IDs and return it.
                Ok(())
            }
            .boxed() // turn the async block into a BoxFuture
        },
    );
    let imports = [hello_func.into()];
    let instance = Instance::new(&mut store, &module, &imports)?;

    Ok(instance)
}
