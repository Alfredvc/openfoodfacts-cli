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
async fn products_get_success() {
    let server = setup().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/product/3017624010701.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": 1,
            "status_verbose": "product found",
            "product": {
                "code": "3017624010701",
                "product_name": "Nutella",
                "brands": "Ferrero",
                "nutriscore_grade": "e"
            }
        })))
        .mount(&server)
        .await;

    let output = cmd(&server)
        .args(["products", "get", "3017624010701"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(json["product_name"], "Nutella");
    assert_eq!(json["brands"], "Ferrero");
    assert_eq!(json["nutriscore_grade"], "e");
    // API wrapper fields must be stripped
    assert!(json.get("status").is_none());
    assert!(json.get("status_verbose").is_none());
}

#[tokio::test]
async fn products_get_not_found() {
    let server = setup().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/product/0000000000000.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": 0,
            "status_verbose": "product not found"
        })))
        .mount(&server)
        .await;

    let output = cmd(&server)
        .args(["products", "get", "0000000000000"])
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();

    let json: Value = serde_json::from_slice(&output).unwrap();
    assert!(json["error"]
        .as_str()
        .unwrap()
        .contains("product not found"));
    assert!(json["error"].as_str().unwrap().contains("0000000000000"));
}

#[tokio::test]
async fn products_get_with_fields() {
    let server = setup().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/product/3017624010701.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": 1,
            "status_verbose": "product found",
            "product": {
                "code": "3017624010701",
                "product_name": "Nutella",
                "brands": "Ferrero",
                "nutriscore_grade": "e"
            }
        })))
        .mount(&server)
        .await;

    let output = cmd(&server)
        .args([
            "--fields",
            "product_name,brands",
            "products",
            "get",
            "3017624010701",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(json["product_name"], "Nutella");
    assert_eq!(json["brands"], "Ferrero");
    assert!(json.get("nutriscore_grade").is_none());
}

#[tokio::test]
async fn products_get_missing_product_key() {
    let server = setup().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/product/1234567890123.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": 1,
            "status_verbose": "product found"
            // no "product" key
        })))
        .mount(&server)
        .await;

    let output = cmd(&server)
        .args(["products", "get", "1234567890123"])
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();

    let json: Value = serde_json::from_slice(&output).unwrap();
    assert!(json["error"].as_str().unwrap().contains("no product data"));
}

#[tokio::test]
async fn products_get_rate_limited() {
    let server = setup().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/product/3017624010701.json"))
        .respond_with(ResponseTemplate::new(429))
        .mount(&server)
        .await;

    let output = cmd(&server)
        .args(["products", "get", "3017624010701"])
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();

    let json: Value = serde_json::from_slice(&output).unwrap();
    assert!(json["error"].as_str().unwrap().contains("rate limit"));
}
