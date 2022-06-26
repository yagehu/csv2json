use reqwest::StatusCode;

use crate::{DocumentResponse, TestApp};

#[tokio::test]
async fn line_break_ok() {
    let app = TestApp::spawn().await;
    let cases = [
        ("", 0),
        ("\n\n\n", 0),
        ("\r\n\r\n\r\n", 0),
        ("a,b,c\n", 1),
        ("a\nb\nc\n", 3),
        ("a\r\n", 1),
        ("a\r\nb\r\n", 2),
        ("a\r\nb\n", 2),
        ("a\r\nb\nc\r", 3),
    ];

    for case in cases {
        let response = app
            .client
            .post(format!("{}/document", &app.address))
            .header("Content-Type", "text/csv; charset=utf8")
            .body(case.0)
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

        assert_eq!(resp.content.len(), case.1, "{} -> {:?}", case.0, resp);
    }
}
