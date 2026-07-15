use super::*;
use crate::config::Credentials;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[tokio::test]
async fn dump_without_issue_syncs_before_fetching_current_schema() {
    // Given: a Bytebase-compatible server that records sync and schema requests.
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut buffer = vec![0_u8; 4096];
        let bytes_read = stream.read(&mut buffer).await.unwrap();
        let request = String::from_utf8_lossy(&buffer[..bytes_read]);
        let first_request = request.lines().next().unwrap().to_string();

        if !first_request.starts_with("POST ") {
            let body = r#"{"schema":"CREATE TABLE setting(id INT);"}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).await.unwrap();
            return vec![first_request];
        }

        let body = "{}";
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).await.unwrap();

        let (mut stream, _) = listener.accept().await.unwrap();
        let bytes_read = stream.read(&mut buffer).await.unwrap();
        let request = String::from_utf8_lossy(&buffer[..bytes_read]);
        let second_request = request.lines().next().unwrap().to_string();
        let body = r#"{"schema":"CREATE TABLE setting(id INT);"}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).await.unwrap();
        vec![first_request, second_request]
    });
    let credentials = Credentials {
        url: format!("http://{address}"),
        service_account: "test@example.com".to_string(),
        service_key: None,
        access_token: "test-token".to_string(),
    };
    let client = LiveApiClient::new(&credentials).unwrap();

    // When: dump fetches the current schema without an issue selector.
    let schema = fetch_current_schema(&client, "dev", "setting")
        .await
        .unwrap();
    let requests = server.await.unwrap();

    // Then: it synchronizes Bytebase before retrieving the schema.
    assert_eq!(schema, "CREATE TABLE setting(id INT);");
    assert_eq!(
        requests,
        vec![
            "POST /v1/instances/dev/databases/setting:sync HTTP/1.1",
            "GET /v1/instances/dev/databases/setting/schema HTTP/1.1",
        ]
    );
}
