use std::net::SocketAddr;
use std::sync::Once;

use tokio;
use tonic::Code;

use paas_types::{ExecRequest, StatusRequest};
use paasc::make_client;
use paasd::make_server;

fn exec_request(args: &[&str]) -> ExecRequest {
    ExecRequest {
        args: args.iter().cloned().map(ToOwned::to_owned).collect(),
    }
}

fn init() {
    static INIT: Once = Once::new();
    // Tests launch in $REPO/paasd, but certificate paths hardcoded as $REPO/data
    INIT.call_once(|| {
        std::env::set_current_dir("..").unwrap();
    });
}

fn test_server(port: u16) {
    tokio::spawn(async move {
        let server = make_server().unwrap();
        server
            .serve(SocketAddr::new("127.0.0.1".parse().unwrap(), port))
            .await
            .unwrap();
    });
}

#[tokio::test]
async fn test_spawn() {
    init();
    test_server(18001);
    let mut client = make_client(18001, "client1").await.unwrap();
    client.exec(exec_request(&["echo"])).await.unwrap();
}

#[tokio::test]
async fn test_authorization() {
    init();
    test_server(18002);
    let mut client1 = make_client(18002, "client1").await.unwrap();
    let mut client2 = make_client(18002, "client2").await.unwrap();

    let pid1 = client1
        .exec(exec_request(&["echo"]))
        .await
        .unwrap()
        .into_inner()
        .id;

    // Can access own process
    assert!(client1
        .get_status(StatusRequest { id: pid1.clone() })
        .await
        .is_ok());

    // Other client can not access the process
    let err = client2
        .get_status(StatusRequest { id: pid1.clone() })
        .await
        .unwrap_err();
    assert_eq!(err.code(), Code::Unauthenticated);
}
