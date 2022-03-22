use crate::helpers;

#[actix_rt::test]
async fn health_check_test() {
    let test_app = helpers::spawn_app().await;

    let client = reqwest::Client::new();

    let responce = client
        .get(format!("{}/health_check", test_app.address))
        .send()
        .await
        .expect("Failed to execute request");

    assert!(responce.status().is_success());
    assert_eq!(responce.content_length(), Some(0));
}
