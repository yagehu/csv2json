use reqwest::StatusCode;
use uuid::Uuid;

use crate::{DocumentResponse, TestApp};

#[tokio::test]
async fn ok() {
    let app = TestApp::spawn().await;
    let response = app
        .client
        .post(format!("{}/document", &app.address))
        .header("Content-Type", "text/csv; charset=utf8")
        .body("a\nb\nc")
        .send()
        .await
        .expect("Failed to execute request.");
    let status = response.status();
    let text = response
        .text()
        .await
        .expect("Failed to read response body as text.");

    assert_eq!(status, StatusCode::CREATED, "{}", &text);

    let resp: DocumentResponse = serde_json::from_str(&text)
        .expect(&format!("Failed to deserialize response text: {}", &text));

    assert_eq!(resp.content.len(), 3);

    let response = app
        .client
        .get(format!("{}/document/{}", &app.address, resp.id))
        .send()
        .await
        .expect("Failed to execute request.");
    let status = response.status();
    let text = response
        .text()
        .await
        .expect("Failed to read response body as text.");

    assert_eq!(status, StatusCode::OK, "{}", &text);

    let resp: DocumentResponse = serde_json::from_str(&text)
        .expect(&format!("Failed to deserialize response text: {}", &text));

    assert_eq!(resp.content, vec![vec!["a"], vec!["b"], vec!["c"]]);
}

#[tokio::test]
async fn not_found_errs() {
    let app = TestApp::spawn().await;
    let id = Uuid::new_v4();
    let response = app
        .client
        .get(format!("{}/document/{}", &app.address, id))
        .send()
        .await
        .expect("Failed to execute request.");
    let status = response.status();
    let text = response
        .text()
        .await
        .expect("Failed to read response body as text.");

    assert_eq!(status, StatusCode::NOT_FOUND, "{}", &text);
}
