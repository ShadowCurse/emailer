use crate::helpers;

#[actix_rt::test]
async fn subscribe_ret_200_if_valid_form() {
    let (addr, pool) = helpers::spawn_app().await;

    let client = reqwest::Client::new();
    let body = "name=pog%20dog&email=pogolius%40gmail.com";

    let responce = client
        .post(&format!("{addr}/subscriptions"))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(responce.status().as_u16(), 200);

    let saved = sqlx::query!("select email, name from subscriptions")
        .fetch_one(&pool)
        .await
        .expect("Failed to fetch saved subscription");

    assert_eq!(saved.email, "pogolius@gmail.com");
    assert_eq!(saved.name, "pog dog");
}

#[actix_rt::test]
async fn subscribe_ret_400_if_invalid_form() {
    let (addr, _) = helpers::spawn_app().await;

    let client = reqwest::Client::new();
    let invalid_forms = vec![
        ("name=pog%20dog", "missing email"),
        ("email=pogolius%40gmail.com", "missing name"),
        ("", "missing name, and email"),
        ("name=&email=pogolius%40gmail.com", "invalid name"),
        ("name=pogdog&email=some_mail_address", "invalid email"),
        ("name=pog%20dog&email=", "invalid email"),
    ];

    for (form, error) in invalid_forms {
        let responce = client
            .post(&format!("{addr}/subscriptions"))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(form)
            .send()
            .await
            .expect("Failed to execute request");

        assert_eq!(
            responce.status().as_u16(),
            400,
            "Did not fail with invalid form, expected error: {}",
            error
        );
    }
}
