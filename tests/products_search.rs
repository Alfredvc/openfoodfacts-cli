use assert_cmd::Command;
use serde_json::{json, Value};
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn setup() -> MockServer {
    MockServer::start().await
}

fn cmd(server: &MockServer) -> Command {
    let mut c = Command::cargo_bin("openfoodfacts").unwrap();
    c.env("OFF_BASE_URL", server.uri());
    c
}

fn search_page(page: u32, page_count: u32, items: Vec<Value>) -> Value {
    json!({
        "count": 100,
        "page": page,
        "page_count": page_count,
        "page_size": 20,
        "skip": 0,
        "products": items
    })
}

fn product(code: &str, name: &str) -> Value {
    json!({"code": code, "product_name": name, "brands": "TestBrand"})
}

#[tokio::test]
async fn search_filter_only_uses_v2() {
    let server = setup().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/search"))
        .and(query_param("categories_tags", "en:chocolates"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(search_page(1, 1, vec![product("123", "Dark Choc")])),
        )
        .mount(&server)
        .await;

    let output = cmd(&server)
        .args(["products", "search", "--category", "en:chocolates"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(json["count"], 100);
    assert_eq!(json["products"][0]["product_name"], "Dark Choc");
}

#[tokio::test]
async fn search_query_only_uses_v1() {
    let server = setup().await;
    Mock::given(method("GET"))
        .and(path("/cgi/search.pl"))
        .and(query_param("search_terms", "chocolate"))
        .and(query_param("json", "1"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(search_page(1, 1, vec![product("456", "Choc Bar")])),
        )
        .mount(&server)
        .await;

    let output = cmd(&server)
        .args(["products", "search", "--query", "chocolate"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(json["products"][0]["product_name"], "Choc Bar");
}

#[tokio::test]
async fn search_query_with_filter_uses_v1_tagtype_syntax() {
    let server = setup().await;
    Mock::given(method("GET"))
        .and(path("/cgi/search.pl"))
        .and(query_param("search_terms", "biscuit"))
        .and(query_param("tagtype_0", "nutrition_grades"))
        .and(query_param("tag_contains_0", "contains"))
        .and(query_param("tag_0", "a"))
        .and(query_param("json", "1"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(search_page(1, 1, vec![product("789", "Healthy Biscuit")])),
        )
        .mount(&server)
        .await;

    let output = cmd(&server)
        .args(["products", "search", "--query", "biscuit", "--nutrition-grade", "a"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(json["products"][0]["code"], "789");
}

#[tokio::test]
async fn search_fields_preserves_envelope() {
    let server = setup().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/search"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(search_page(
                1,
                1,
                vec![product("111", "Thing"), product("222", "Other")],
            )),
        )
        .mount(&server)
        .await;

    let output = cmd(&server)
        .args(["--fields", "code,product_name", "products", "search"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).unwrap();
    // Envelope preserved
    assert_eq!(json["count"], 100);
    assert_eq!(json["page"], 1);
    assert_eq!(json["page_count"], 1);
    assert_eq!(json["page_size"], 20);
    // Items filtered
    assert_eq!(json["products"][0], json!({"code": "111", "product_name": "Thing"}));
    assert!(json["products"][0].get("brands").is_none());
}

#[tokio::test]
async fn search_all_fetches_multiple_pages() {
    let server = setup().await;

    // Page 1
    Mock::given(method("GET"))
        .and(path("/api/v2/search"))
        .and(query_param("page", "1"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(search_page(1, 2, vec![product("111", "A")])),
        )
        .mount(&server)
        .await;

    // Page 2
    Mock::given(method("GET"))
        .and(path("/api/v2/search"))
        .and(query_param("page", "2"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(search_page(2, 2, vec![product("222", "B")])),
        )
        .mount(&server)
        .await;

    let output = cmd(&server)
        .args(["products", "search", "--all"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).unwrap();
    // --all returns flat array
    assert!(json.is_array());
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["code"], "111");
    assert_eq!(arr[1]["code"], "222");
}

#[tokio::test]
async fn search_multiple_filters_v2() {
    let server = setup().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/search"))
        .and(query_param("categories_tags", "en:chocolates"))
        .and(query_param("nutrition_grades_tags", "a"))
        .and(query_param("labels_tags", "en:organic"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(search_page(1, 1, vec![product("999", "Organic Dark Choc")])),
        )
        .mount(&server)
        .await;

    let output = cmd(&server)
        .args([
            "products", "search",
            "--category", "en:chocolates",
            "--nutrition-grade", "a",
            "--label", "en:organic",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(json["products"][0]["code"], "999");
}
