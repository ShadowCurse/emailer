use crate::helpers::{spawn_app, Links, TestApp};
use uuid::Uuid;
use wiremock::matchers::{any, method, path};
use wiremock::{Mock, ResponseTemplate};

#[actix_rt::test]
async fn request_missing_auth_rejected() {
    let test_app = spawn_app().await;

    let newsletter_req_body = serde_json::json!({
        "title": "Newsletter titile",
        "content": {
            "text": "Text body",
            "html": "<p>Html body</p>",
        }
    });

    let response = reqwest::Client::new()
        .post(&format!("{}/newsletters", &test_app.address))
        .json(&newsletter_req_body)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status().as_u16(), 401);
    assert_eq!(
        response.headers()["WWW-Authenticate"],
        "Basic realm=\"publish\""
    );
}

#[actix_rt::test]
async fn newsletter_are_not_send_to_unconfirmed_subs() {
    let test_app = spawn_app().await;
    create_unconfirmed_sub(&test_app).await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&test_app.email_server)
        .await;

    let newsletter_req_body = serde_json::json!({
        "title": "Newsletter titile",
        "content": {
            "text": "Text body",
            "html": "<p>Html body</p>",
        }
    });
    let response = reqwest::Client::new()
        .post(&format!("{}/newsletters", &test_app.address))
        .json(&newsletter_req_body)
        .basic_auth(Uuid::new_v4().to_string(), Some(Uuid::new_v4().to_string()))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status().as_u16(), 200);
}

#[actix_rt::test]
async fn newsletter_are_send_to_confirmed_subs() {
    let test_app = spawn_app().await;
    create_confirmed_sub(&test_app).await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    let newsletter_req_body = serde_json::json!({
        "title": "Newsletter titile",
        "content": {
            "text": "Text body",
            "html": "<p>Html body</p>",
        }
    });
    let response = reqwest::Client::new()
        .post(&format!("{}/newsletters", &test_app.address))
        .json(&newsletter_req_body)
        .basic_auth(Uuid::new_v4().to_string(), Some(Uuid::new_v4().to_string()))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status().as_u16(), 200);
}

async fn create_unconfirmed_sub(app: &TestApp) -> Links {
    let body = "name=pog%20dog&email=pogolius%40gmail.com".to_string();

    let _mg = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed sub")
        .expect(1)
        .mount_as_scoped(&app.email_server)
        .await;

    app.post_subsciptions(body)
        .await
        .error_for_status()
        .unwrap();

    let email = &app
        .email_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();
    app.get_links(email)
}

async fn create_confirmed_sub(app: &TestApp) {
    let links = create_unconfirmed_sub(app).await;
    reqwest::get(links.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}
