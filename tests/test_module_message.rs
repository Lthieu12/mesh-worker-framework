use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use bytes::Bytes;

use mesh_module_framework::*;

struct TestModule {
    event_list: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl Module for TestModule {

    async fn on_init(&mut self) {

        let mut event_list = self.event_list.lock().unwrap();

        event_list.push(
            "INIT".to_string()
        );
    }

    async fn on_message(
        &mut self,
        message: FrameworkMessage,
    ) {

        let mut event_list = self.event_list.lock().unwrap();

        event_list.push(
            format!(
                "MESSAGE:{}",
                message.message_id
            )
        );
    }

    async fn on_close(&mut self) {

        let mut event_list = self.event_list.lock().unwrap();

        event_list.push(
            "CLOSE".to_string()
        );
    }
}

#[tokio::test]
async fn test_module_message_fifo() {

    MMF::init().unwrap();

    let event_list =
        Arc::new(
            Mutex::new(
                Vec::<String>::new()
            )
        );

    let module_id =
        MMF::spawn_module(
            TestModule {
                event_list: event_list.clone(),
            },
            None,
        )
        .unwrap();

    MMF::init_module(&module_id)
        .await
        .unwrap();

    MMF::send_message(
        &module_id,
        &module_id,
        &"Message1".to_string(),
        Bytes::new(),
    )
    .await
    .unwrap();

    MMF::send_message(
        &module_id,
        &module_id,
        &"Message2".to_string(),
        Bytes::new(),
    )
    .await
    .unwrap();

    MMF::close_module(&module_id)
        .await
        .unwrap();

    let event_list =
        event_list.lock().unwrap();

    assert_eq!(
        *event_list,
        vec![
            "INIT",
            "MESSAGE:Message1",
            "MESSAGE:Message2",
            "CLOSE",
        ]
    );

    drop(event_list);

    MMF::close().unwrap();
}