use assert_cmd::Command;
use serde_json::{json, Value};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn setup() -> MockServer {
    MockServer::start().await
}

fn cmd(server: &MockServer) -> Command {
    let mut c = Command::cargo_bin("openfoodfacts").unwrap();
    c.env("OFF_BASE_URL", server.uri());
    c
}

#[tokio::test]
async fn facets_list_categories() {
    let server = setup().await;
    Mock::given(method("GET"))
        .and(path("/categories.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "count": 3,
            "tags": [
                {"id": "en:chocolates", "name": "Chocolates", "products": 4821, "url": "https://world.openfoodfacts.net/category/en:chocolates", "known": 1},
                {"id": "en:breads", "name": "Breads", "products": 2000, "url": "https://world.openfoodfacts.net/category/en:breads", "known": 1},
                {"id": "en:cheeses", "name": "Cheeses", "products": 1500, "url": "https://world.openfoodfacts.net/category/en:cheeses", "known": 1}
            ]
        })))
        .mount(&server)
        .await;

    let output = cmd(&server)
        .args(["facets", "list", "categories"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).unwrap();
    assert!(json.is_array());
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 3);
    assert_eq!(arr[0]["id"], "en:chocolates");
    assert_eq!(arr[0]["name"], "Chocolates");
    assert_eq!(arr[0]["products"], 4821);
    // count envelope dropped
    assert!(json.get("count").is_none());
}

#[tokio::test]
async fn facets_list_unknown_type_errors() {
    let server = setup().await;

    let output = cmd(&server)
        .args(["facets", "list", "foobar"])
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();

    let json: Value = serde_json::from_slice(&output).unwrap();
    let error = json["error"].as_str().unwrap();
    assert!(error.contains("unknown facet type"));
    assert!(error.contains("foobar"));
    assert!(error.contains("categories"));
}

#[tokio::test]
async fn facets_list_with_fields() {
    let server = setup().await;
    Mock::given(method("GET"))
        .and(path("/labels.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "count": 2,
            "tags": [
                {"id": "en:organic", "name": "Organic", "products": 1000, "url": "...", "known": 1},
                {"id": "en:vegan", "name": "Vegan", "products": 500, "url": "...", "known": 1}
            ]
        })))
        .mount(&server)
        .await;

    let output = cmd(&server)
        .args(["--fields", "id,products", "facets", "list", "labels"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).unwrap();
    assert!(json.is_array());
    assert_eq!(json[0], json!({"id": "en:organic", "products": 1000}));
    assert!(json[0].get("name").is_none());
    assert!(json[0].get("url").is_none());
}
