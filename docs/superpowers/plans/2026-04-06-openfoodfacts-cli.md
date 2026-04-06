# openfoodfacts CLI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a single-binary Rust CLI named `openfoodfacts` that wraps the Open Food Facts v2 API, outputting structured JSON to stdout, designed for AI agent consumption.

**Architecture:** No auth required. Three commands: `products get`, `products search`, `facets list`. A global `--fields` flag filters output fields. Search routes to v1 (`/cgi/search.pl`) when `--query` is present, v2 (`/api/v2/search`) otherwise. Integration tests use wiremock + assert_cmd with `OFF_BASE_URL` env var injection.

**Tech Stack:** Rust, clap 4 (derive), reqwest 0.12 (rustls-tls), tokio 1, serde_json 1, anyhow 1; dev: wiremock 0.6, assert_cmd 2

---

## File Map

| File | Responsibility |
|------|---------------|
| `Cargo.toml` | Dependencies, binary name |
| `src/main.rs` | Entry point: parse args, dispatch, error → stderr + exit 1 |
| `src/cli.rs` | All clap derive structs |
| `src/client.rs` | HTTP GET, User-Agent injection, status code → error mapping, `OFF_BASE_URL` override |
| `src/output.rs` | TTY detection, JSON pretty/compact print, field filtering |
| `src/commands/mod.rs` | Re-exports |
| `src/commands/products.rs` | `products get` + `products search` logic |
| `src/commands/facets.rs` | `facets list` logic |
| `tests/products_get.rs` | Integration tests for `products get` |
| `tests/products_search.rs` | Integration tests for `products search` |
| `tests/facets_list.rs` | Integration tests for `facets list` |
| `README.md` | Human-facing docs |
| `AGENTS.md` | Agent-facing docs (symlinked as `CLAUDE.md`) |
| `.github/workflows/ci.yml` | Test on every push |
| `.github/workflows/release.yml` | Cross-platform release builds |
| `scripts/install.sh` | Install script |

---

## Task 1: Project Scaffold

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/cli.rs`
- Create: `src/client.rs`
- Create: `src/output.rs`
- Create: `src/commands/mod.rs`
- Create: `src/commands/products.rs`
- Create: `src/commands/facets.rs`

- [ ] **Step 1: Initialize git and cargo project**

```bash
cd /Users/alfredvc/src/off-cli
git init
cargo init --name openfoodfacts
```

- [ ] **Step 2: Write `Cargo.toml`**

```toml
[package]
name = "openfoodfacts"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "openfoodfacts"
path = "src/main.rs"

[dependencies]
anyhow = "1"
clap = { version = "4", features = ["derive"] }
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }

[dev-dependencies]
assert_cmd = "2"
tokio = { version = "1", features = ["rt", "macros"] }
wiremock = "0.6"
```

- [ ] **Step 3: Create stub source files so the project compiles**

`src/cli.rs`:
```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "openfoodfacts", about = "Open Food Facts CLI for AI agents")]
pub struct Cli {
    /// Force compact JSON output
    #[arg(long, global = true)]
    pub json: bool,

    /// Return only these fields, comma-separated (e.g. product_name,brands)
    #[arg(long, global = true, value_delimiter = ',')]
    pub fields: Vec<String>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Product lookup and search
    Products {
        #[command(subcommand)]
        command: ProductsCommand,
    },
    /// Browse facet dimensions
    Facets {
        #[command(subcommand)]
        command: FacetsCommand,
    },
}

#[derive(Subcommand)]
pub enum ProductsCommand {
    /// Look up a single product by barcode
    Get {
        /// Barcode string (e.g. 3017624010701)
        barcode: String,
    },
    /// Search or filter the product database
    Search {
        /// Full-text search query (routes to v1 /cgi/search.pl)
        #[arg(long)]
        query: Option<String>,
        /// Filter by category tag (e.g. en:chocolates)
        #[arg(long)]
        category: Option<String>,
        /// Filter by nutrition grade (a-e)
        #[arg(long)]
        nutrition_grade: Option<String>,
        /// Filter by eco-score grade (a-e)
        #[arg(long)]
        ecoscore_grade: Option<String>,
        /// Filter by label tag (e.g. en:organic)
        #[arg(long)]
        label: Option<String>,
        /// Filter by ingredient tag (e.g. en:salt)
        #[arg(long)]
        ingredient: Option<String>,
        /// Filter by allergen tag (e.g. en:gluten)
        #[arg(long)]
        allergen: Option<String>,
        /// Sort results by field (e.g. last_modified_t, unique_scans_n)
        #[arg(long)]
        sort_by: Option<String>,
        /// Page number (default: 1)
        #[arg(long, default_value = "1")]
        page: u32,
        /// Items per page (default: 20, max: 100)
        #[arg(long, default_value = "20")]
        page_size: u32,
        /// Fetch all pages and return a flat array
        #[arg(long)]
        all: bool,
    },
}

#[derive(Subcommand)]
pub enum FacetsCommand {
    /// List all entries in a facet dimension
    List {
        /// One of: categories, labels, ingredients, brands, countries, additives, allergens, packaging
        facet_type: String,
    },
}
```

`src/output.rs`:
```rust
use serde_json::Value;
use std::io::IsTerminal;

pub struct Output {
    force_compact: bool,
    fields: Vec<String>,
}

impl Output {
    pub fn new(force_compact: bool, fields: Vec<String>) -> Self {
        Self { force_compact, fields }
    }

    pub fn print(&self, value: &Value) {
        let filtered = self.filter_fields(value.clone());
        let s = if self.force_compact || !std::io::stdout().is_terminal() {
            serde_json::to_string(&filtered).unwrap()
        } else {
            serde_json::to_string_pretty(&filtered).unwrap()
        };
        println!("{s}");
    }

    pub fn filter_fields(&self, value: Value) -> Value {
        if self.fields.is_empty() {
            return value;
        }
        match value {
            Value::Object(ref map) if map.contains_key("products") => {
                let mut new_map = map.clone();
                if let Some(Value::Array(products)) = new_map.get("products").cloned() {
                    let filtered: Vec<Value> = products
                        .into_iter()
                        .map(|p| self.filter_object(&p))
                        .collect();
                    new_map.insert("products".to_string(), Value::Array(filtered));
                }
                Value::Object(new_map)
            }
            Value::Array(arr) => {
                Value::Array(arr.into_iter().map(|v| self.filter_object(&v)).collect())
            }
            Value::Object(_) => self.filter_object(&value),
            other => other,
        }
    }

    fn filter_object(&self, value: &Value) -> Value {
        if let Value::Object(map) = value {
            let filtered = map
                .iter()
                .filter(|(k, _)| self.fields.iter().any(|f| f == k.as_str()))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            Value::Object(filtered)
        } else {
            value.clone()
        }
    }
}
```

`src/client.rs`:
```rust
use anyhow::{bail, Context, Result};
use serde_json::Value;

const DEFAULT_BASE_URL: &str = "https://world.openfoodfacts.net";

pub struct Client {
    inner: reqwest::Client,
    pub base_url: String,
}

impl Client {
    pub fn new() -> Result<Self> {
        let base_url = std::env::var("OFF_BASE_URL")
            .unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());
        let base_url = base_url.trim_end_matches('/').to_string();
        let user_agent = format!(
            "openfoodfacts-cli/{} (https://github.com/alfredvc/openfoodfacts-cli)",
            env!("CARGO_PKG_VERSION")
        );
        let inner = reqwest::Client::builder()
            .user_agent(user_agent)
            .build()
            .context("failed to build HTTP client")?;
        Ok(Self { inner, base_url })
    }

    pub async fn get(&self, path: &str, params: &[(&str, &str)]) -> Result<Value> {
        let url = format!("{}{}", self.base_url, path);
        let response = self
            .inner
            .get(&url)
            .query(params)
            .send()
            .await
            .with_context(|| format!("request to {url} failed"))?;

        let status = response.status();
        if status == 429 || status == 403 {
            bail!("rate limit exceeded");
        }
        if status == 404 {
            bail!("not found: {path}");
        }
        if !status.is_success() {
            bail!("API error: HTTP {status}");
        }

        response.json::<Value>().await.context("failed to parse JSON response")
    }
}
```

`src/commands/mod.rs`:
```rust
pub mod facets;
pub mod products;
```

`src/commands/products.rs`:
```rust
use anyhow::{bail, Result};
use crate::cli::ProductsCommand;
use crate::client::Client;
use crate::output::Output;

pub async fn run(command: &ProductsCommand, client: &Client, output: &Output) -> Result<()> {
    match command {
        ProductsCommand::Get { barcode } => get(barcode, client, output).await,
        ProductsCommand::Search { .. } => search(command, client, output).await,
    }
}

async fn get(_barcode: &str, _client: &Client, _output: &Output) -> Result<()> {
    bail!("not yet implemented")
}

async fn search(_command: &ProductsCommand, _client: &Client, _output: &Output) -> Result<()> {
    bail!("not yet implemented")
}
```

`src/commands/facets.rs`:
```rust
use anyhow::{bail, Result};
use crate::cli::FacetsCommand;
use crate::client::Client;
use crate::output::Output;

pub async fn run(command: &FacetsCommand, client: &Client, output: &Output) -> Result<()> {
    match command {
        FacetsCommand::List { facet_type } => list(facet_type, client, output).await,
    }
}

async fn list(_facet_type: &str, _client: &Client, _output: &Output) -> Result<()> {
    bail!("not yet implemented")
}
```

`src/main.rs`:
```rust
use anyhow::Result;
use clap::Parser;

mod cli;
mod client;
mod commands;
mod output;

use cli::{Cli, Commands};

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        let msg = serde_json::json!({"error": e.to_string()});
        eprintln!("{}", serde_json::to_string(&msg).unwrap());
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();
    let output = output::Output::new(cli.json, cli.fields.clone());
    let client = client::Client::new()?;

    match &cli.command {
        Commands::Products { command } => commands::products::run(command, &client, &output).await,
        Commands::Facets { command } => commands::facets::run(command, &client, &output).await,
    }
}
```

- [ ] **Step 4: Verify the project compiles**

```bash
cargo build 2>&1
```
Expected: compiles with no errors (warnings about unused variables in stubs are fine).

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock src/
git commit -m "feat: initial project scaffold"
```

---

## Task 2: Output Module Unit Tests

**Files:**
- Modify: `src/output.rs`

- [ ] **Step 1: Write the failing unit tests**

Add to the bottom of `src/output.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn output_with_fields(fields: &[&str]) -> Output {
        Output::new(true, fields.iter().map(|s| s.to_string()).collect())
    }

    fn output_all_fields() -> Output {
        Output::new(true, vec![])
    }

    #[test]
    fn filter_fields_empty_returns_value_unchanged() {
        let out = output_all_fields();
        let v = json!({"a": 1, "b": 2});
        assert_eq!(out.filter_fields(v.clone()), v);
    }

    #[test]
    fn filter_fields_on_object() {
        let out = output_with_fields(&["product_name", "brands"]);
        let v = json!({"product_name": "Nutella", "brands": "Ferrero", "nutriscore_grade": "e"});
        let result = out.filter_fields(v);
        assert_eq!(result, json!({"product_name": "Nutella", "brands": "Ferrero"}));
    }

    #[test]
    fn filter_fields_on_flat_array() {
        let out = output_with_fields(&["code"]);
        let v = json!([{"code": "123", "name": "A"}, {"code": "456", "name": "B"}]);
        let result = out.filter_fields(v);
        assert_eq!(result, json!([{"code": "123"}, {"code": "456"}]));
    }

    #[test]
    fn filter_fields_preserves_pagination_envelope() {
        let out = output_with_fields(&["code", "product_name"]);
        let v = json!({
            "count": 100,
            "page": 1,
            "page_count": 5,
            "page_size": 20,
            "skip": 0,
            "products": [
                {"code": "123", "product_name": "A", "brands": "X"},
                {"code": "456", "product_name": "B", "brands": "Y"}
            ]
        });
        let result = out.filter_fields(v);
        // Envelope keys preserved
        assert_eq!(result["count"], 100);
        assert_eq!(result["page"], 1);
        assert_eq!(result["page_count"], 5);
        assert_eq!(result["page_size"], 20);
        assert_eq!(result["skip"], 0);
        // Items filtered
        assert_eq!(result["products"][0], json!({"code": "123", "product_name": "A"}));
        assert_eq!(result["products"][1], json!({"code": "456", "product_name": "B"}));
    }

    #[test]
    fn filter_fields_non_object_passthrough() {
        let out = output_with_fields(&["x"]);
        assert_eq!(out.filter_fields(json!(42)), json!(42));
        assert_eq!(out.filter_fields(json!("hello")), json!("hello"));
        assert_eq!(out.filter_fields(json!(null)), json!(null));
    }
}
```

- [ ] **Step 2: Run tests to verify they pass (implementation already exists)**

```bash
cargo test --lib output 2>&1
```
Expected: all 5 tests pass.

- [ ] **Step 3: Commit**

```bash
git add src/output.rs
git commit -m "test: output module unit tests"
```

---

## Task 3: `products get` Command

**Files:**
- Modify: `src/commands/products.rs`
- Create: `tests/products_get.rs`

- [ ] **Step 1: Write the failing integration tests**

Create `tests/products_get.rs`:

```rust
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
    assert!(json["error"].as_str().unwrap().contains("product not found"));
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
        .args(["--fields", "product_name,brands", "products", "get", "3017624010701"])
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
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cargo test --test products_get 2>&1
```
Expected: tests compile but `products_get_success` fails because `get` returns "not yet implemented".

- [ ] **Step 3: Implement `products get` in `src/commands/products.rs`**

Replace the stub `get` function:

```rust
async fn get(barcode: &str, client: &Client, output: &Output) -> Result<()> {
    let path = format!("/api/v2/product/{}.json", barcode);
    let body = client.get(&path, &[]).await?;

    let status = body.get("status").and_then(|v| v.as_u64()).unwrap_or(0);
    if status == 0 {
        bail!("product not found: {}", barcode);
    }

    let product = body
        .get("product")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    output.print(&product);
    Ok(())
}
```

- [ ] **Step 4: Run tests to confirm they pass**

```bash
cargo test --test products_get 2>&1
```
Expected: all 4 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/commands/products.rs tests/products_get.rs
git commit -m "feat: products get command"
```

---

## Task 4: `products search` Command

**Files:**
- Modify: `src/commands/products.rs`
- Create: `tests/products_search.rs`

- [ ] **Step 1: Write the failing integration tests**

Create `tests/products_search.rs`:

```rust
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
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cargo test --test products_search 2>&1
```
Expected: compilation fails or tests fail with "not yet implemented".

- [ ] **Step 3: Implement `products search` in `src/commands/products.rs`**

Replace the entire `search` function and add helpers. Final `src/commands/products.rs`:

```rust
use anyhow::{bail, Result};
use serde_json::Value;

use crate::cli::ProductsCommand;
use crate::client::Client;
use crate::output::Output;

pub async fn run(command: &ProductsCommand, client: &Client, output: &Output) -> Result<()> {
    match command {
        ProductsCommand::Get { barcode } => get(barcode, client, output).await,
        ProductsCommand::Search { .. } => search(command, client, output).await,
    }
}

async fn get(barcode: &str, client: &Client, output: &Output) -> Result<()> {
    let path = format!("/api/v2/product/{}.json", barcode);
    let body = client.get(&path, &[]).await?;

    let status = body.get("status").and_then(|v| v.as_u64()).unwrap_or(0);
    if status == 0 {
        bail!("product not found: {}", barcode);
    }

    let product = body
        .get("product")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    output.print(&product);
    Ok(())
}

async fn search(command: &ProductsCommand, client: &Client, output: &Output) -> Result<()> {
    let ProductsCommand::Search {
        query,
        category,
        nutrition_grade,
        ecoscore_grade,
        label,
        ingredient,
        allergen,
        sort_by,
        page,
        page_size,
        all,
    } = command
    else {
        unreachable!()
    };

    if let Some(q) = query {
        search_v1(
            q,
            category.as_deref(),
            nutrition_grade.as_deref(),
            ecoscore_grade.as_deref(),
            label.as_deref(),
            ingredient.as_deref(),
            allergen.as_deref(),
            sort_by.as_deref(),
            *page,
            *page_size,
            *all,
            client,
            output,
        )
        .await
    } else {
        search_v2(
            category.as_deref(),
            nutrition_grade.as_deref(),
            ecoscore_grade.as_deref(),
            label.as_deref(),
            ingredient.as_deref(),
            allergen.as_deref(),
            sort_by.as_deref(),
            *page,
            *page_size,
            *all,
            client,
            output,
        )
        .await
    }
}

async fn search_v2(
    category: Option<&str>,
    nutrition_grade: Option<&str>,
    ecoscore_grade: Option<&str>,
    label: Option<&str>,
    ingredient: Option<&str>,
    allergen: Option<&str>,
    sort_by: Option<&str>,
    page: u32,
    page_size: u32,
    all: bool,
    client: &Client,
    output: &Output,
) -> Result<()> {
    let page_str = page.to_string();
    let page_size_str = page_size.to_string();

    let mut params: Vec<(&str, &str)> = vec![
        ("page", &page_str),
        ("page_size", &page_size_str),
    ];
    if let Some(v) = category { params.push(("categories_tags", v)); }
    if let Some(v) = nutrition_grade { params.push(("nutrition_grades_tags", v)); }
    if let Some(v) = ecoscore_grade { params.push(("ecoscore_tags", v)); }
    if let Some(v) = label { params.push(("labels_tags", v)); }
    if let Some(v) = ingredient { params.push(("ingredients_tags", v)); }
    if let Some(v) = allergen { params.push(("allergens_tags", v)); }
    if let Some(v) = sort_by { params.push(("sort_by", v)); }

    if all {
        let all_products = fetch_all_pages_v2(&params, client).await?;
        output.print(&Value::Array(all_products));
    } else {
        let body = client.get("/api/v2/search", &params).await?;
        output.print(&body);
    }
    Ok(())
}

async fn fetch_all_pages_v2(
    base_params: &[(&str, &str)],
    client: &Client,
) -> Result<Vec<Value>> {
    let filtered_params: Vec<(&str, &str)> = base_params
        .iter()
        .filter(|(k, _)| *k != "page")
        .copied()
        .collect();

    let page_str_1 = "1".to_string();
    let mut params = filtered_params.clone();
    params.push(("page", &page_str_1));

    let first = client.get("/api/v2/search", &params).await?;
    let page_count = first
        .get("page_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(1);
    let mut all: Vec<Value> = extract_products(&first);

    for p in 2..=page_count {
        let page_num = p.to_string();            // owned, lives for this iteration
        let mut page_params = filtered_params.clone();
        page_params.push(("page", &page_num));   // borrow lives until end of loop body
        let body = client.get("/api/v2/search", &page_params).await?;
        all.extend(extract_products(&body));
    }
    Ok(all)
}

fn extract_products(body: &Value) -> Vec<Value> {
    body.get("products")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
}

async fn search_v1(
    query: &str,
    category: Option<&str>,
    nutrition_grade: Option<&str>,
    ecoscore_grade: Option<&str>,
    label: Option<&str>,
    ingredient: Option<&str>,
    allergen: Option<&str>,
    sort_by: Option<&str>,
    page: u32,
    page_size: u32,
    all: bool,
    client: &Client,
    output: &Output,
) -> Result<()> {
    let page_str = page.to_string();
    let page_size_str = page_size.to_string();

    let mut params: Vec<(&str, &str)> = vec![
        ("search_terms", query),
        ("json", "1"),
        ("page", &page_str),
        ("page_size", &page_size_str),
    ];
    if let Some(v) = sort_by { params.push(("sort_by", v)); }

    // Map filter flags to v1 tagtype triplets
    let filters: Vec<(&str, &str)> = [
        category.map(|v| ("categories", v)),
        nutrition_grade.map(|v| ("nutrition_grades", v)),
        ecoscore_grade.map(|v| ("ecoscore_grade", v)),
        label.map(|v| ("labels", v)),
        ingredient.map(|v| ("ingredients", v)),
        allergen.map(|v| ("allergens", v)),
    ]
    .into_iter()
    .flatten()
    .collect();

    // Build tagtype_N/tag_contains_N/tag_N params as owned strings
    // We need owned strings because params borrows &str
    let mut owned: Vec<(String, String)> = Vec::new();
    for (n, (tagtype, tag_value)) in filters.iter().enumerate() {
        owned.push((format!("tagtype_{}", n), tagtype.to_string()));
        owned.push((format!("tag_contains_{}", n), "contains".to_string()));
        owned.push((format!("tag_{}", n), tag_value.to_string()));
    }
    let tag_params: Vec<(&str, &str)> = owned
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    params.extend(tag_params.iter().copied());

    if all {
        let all_products = fetch_all_pages_v1(&params, client).await?;
        output.print(&Value::Array(all_products));
    } else {
        let body = client.get("/cgi/search.pl", &params).await?;
        output.print(&body);
    }
    Ok(())
}

async fn fetch_all_pages_v1(
    base_params: &[(&str, &str)],
    client: &Client,
) -> Result<Vec<Value>> {
    let filtered_params: Vec<(&str, &str)> = base_params
        .iter()
        .filter(|(k, _)| *k != "page")
        .copied()
        .collect();

    let page_str_1 = "1".to_string();
    let mut params = filtered_params.clone();
    params.push(("page", &page_str_1));

    let first = client.get("/cgi/search.pl", &params).await?;
    let page_count = first
        .get("page_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(1);
    let mut all: Vec<Value> = extract_products(&first);

    for p in 2..=page_count {
        let page_num = p.to_string();            // owned, lives for this iteration
        let mut page_params = filtered_params.clone();
        page_params.push(("page", &page_num));   // borrow lives until end of loop body
        let body = client.get("/cgi/search.pl", &page_params).await?;
        all.extend(extract_products(&body));
    }
    Ok(all)
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test --test products_search 2>&1
```
Expected: all 6 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/commands/products.rs tests/products_search.rs
git commit -m "feat: products search command (v1/v2 routing, --all, field filtering)"
```

---

## Task 5: `facets list` Command

**Files:**
- Modify: `src/commands/facets.rs`
- Create: `tests/facets_list.rs`

- [ ] **Step 1: Write the failing integration tests**

Create `tests/facets_list.rs`:

```rust
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
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cargo test --test facets_list 2>&1
```
Expected: `facets_list_categories` fails with "not yet implemented".

- [ ] **Step 3: Implement `facets list` in `src/commands/facets.rs`**

```rust
use anyhow::{bail, Result};
use crate::cli::FacetsCommand;
use crate::client::Client;
use crate::output::Output;

const VALID_FACET_TYPES: &[&str] = &[
    "categories",
    "labels",
    "ingredients",
    "brands",
    "countries",
    "additives",
    "allergens",
    "packaging",
];

pub async fn run(command: &FacetsCommand, client: &Client, output: &Output) -> Result<()> {
    match command {
        FacetsCommand::List { facet_type } => list(facet_type, client, output).await,
    }
}

async fn list(facet_type: &str, client: &Client, output: &Output) -> Result<()> {
    if !VALID_FACET_TYPES.contains(&facet_type) {
        bail!(
            "unknown facet type: \"{}\" — valid: {}",
            facet_type,
            VALID_FACET_TYPES.join(", ")
        );
    }

    let path = format!("/{}.json", facet_type);
    let body = client.get(&path, &[]).await?;

    let tags = body
        .get("tags")
        .cloned()
        .unwrap_or(serde_json::Value::Array(vec![]));
    output.print(&tags);
    Ok(())
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test --test facets_list 2>&1
```
Expected: all 3 tests pass.

- [ ] **Step 5: Run full test suite**

```bash
cargo test 2>&1
```
Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/commands/facets.rs tests/facets_list.rs
git commit -m "feat: facets list command"
```

---

## Task 6: README.md and AGENTS.md

**Files:**
- Create: `README.md`
- Create: `AGENTS.md`
- Create: `CLAUDE.md` (symlink to AGENTS.md)

- [ ] **Step 1: Write `README.md`**

```markdown
# openfoodfacts

Single-binary CLI for the [Open Food Facts](https://world.openfoodfacts.net) API. Designed for AI agent consumption — all output is JSON.

## Installation

```bash
curl -fsSL https://raw.githubusercontent.com/alfredvc/openfoodfacts-cli/main/scripts/install.sh | bash
```

Or build from source:

```bash
cargo install --git https://github.com/alfredvc/openfoodfacts-cli
```

## Quick Start

```bash
# Look up a product by barcode
openfoodfacts products get 3017624010701

# Search by category
openfoodfacts products search --category en:chocolates --nutrition-grade a

# Full-text search
openfoodfacts products search --query "organic olive oil" --label en:organic

# List all categories
openfoodfacts facets list categories

# Return only specific fields (saves tokens for AI agents)
openfoodfacts --fields product_name,brands,nutriscore_grade products get 3017624010701
```

## Commands

### Global Flags

| Flag | Description |
|------|-------------|
| `--fields f1,f2` | Return only specified fields |
| `--json` | Force compact JSON (default when piped) |

### `products get <barcode>`

Look up a single product by barcode string.

```bash
openfoodfacts products get 3017624010701
openfoodfacts products get 3017624010701 --fields product_name,brands,nutriscore_grade,ecoscore_grade
```

### `products search`

Filter and search the product database.

| Flag | Description |
|------|-------------|
| `--query <text>` | Full-text search |
| `--category <tag>` | e.g. `en:chocolates` |
| `--nutrition-grade <a-e>` | Filter by Nutri-Score |
| `--ecoscore-grade <a-e>` | Filter by Eco-Score |
| `--label <tag>` | e.g. `en:organic` |
| `--ingredient <tag>` | e.g. `en:salt` |
| `--allergen <tag>` | e.g. `en:gluten` |
| `--sort-by <field>` | e.g. `last_modified_t`, `unique_scans_n` |
| `--page <n>` | Page number (default: 1) |
| `--page-size <n>` | Items per page (default: 20, max: 100) |
| `--all` | Fetch all pages, return flat array |

### `facets list <type>`

Browse a facet dimension. Valid types: `categories`, `labels`, `ingredients`, `brands`, `countries`, `additives`, `allergens`, `packaging`.

```bash
openfoodfacts facets list categories
openfoodfacts --fields id,products facets list labels
```

## Output

- **Success:** JSON to stdout, exit 0
- **Error:** `{"error": "..."}` to stderr, exit 1
- TTY: pretty-printed JSON; piped: compact JSON

## Rate Limits

Open Food Facts enforces: 100 req/min (product lookups), 10 req/min (search), 2 req/min (facets).
```

- [ ] **Step 2: Write `AGENTS.md`**

```markdown
# openfoodfacts CLI — Agent Reference

All output is JSON. Errors go to stderr as `{"error": "..."}` with exit code 1.

## Commands

### products get <barcode>
```
openfoodfacts products get <barcode-string>
openfoodfacts --fields product_name,brands,nutriscore_grade products get <barcode>
```
Returns the product object. Exit 1 with `{"error": "product not found: <barcode>"}` if not found.

### products search
```
openfoodfacts products search [flags...]
openfoodfacts --fields code,product_name products search --category en:chocolates --nutrition-grade a
```
Returns `{"count":N,"page":N,"page_count":N,"page_size":N,"skip":N,"products":[...]}`.
`--all` returns flat array `[...]` of all pages.

Flags: `--query`, `--category`, `--nutrition-grade`, `--ecoscore-grade`, `--label`, `--ingredient`, `--allergen`, `--sort-by`, `--page`, `--page-size`, `--all`

### facets list <type>
```
openfoodfacts facets list <type>
```
Type: `categories` `labels` `ingredients` `brands` `countries` `additives` `allergens` `packaging`
Returns array of `{"id":"en:...","name":"...","products":N}`.

## Global Flags
- `--fields f1,f2` — filter output fields (envelope preserved on paginated results)
- `--json` — compact output

## Rate Limits
100/min product · 10/min search · 2/min facets
```

- [ ] **Step 3: Create CLAUDE.md symlink**

```bash
ln -s AGENTS.md CLAUDE.md
```

- [ ] **Step 4: Commit**

```bash
git add README.md AGENTS.md CLAUDE.md
git commit -m "docs: README and AGENTS.md"
```

---

## Task 7: GitHub Actions CI + Release Workflow

**Files:**
- Create: `.github/workflows/ci.yml`
- Create: `.github/workflows/release.yml`
- Create: `scripts/install.sh`

- [ ] **Step 1: Write `.github/workflows/ci.yml`**

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo test
      - run: cargo clippy -- -D warnings
      - run: cargo fmt --check
```

- [ ] **Step 2: Write `.github/workflows/release.yml`**

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  build:
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            artifact: openfoodfacts-linux-x86_64
          - os: ubuntu-latest
            target: aarch64-unknown-linux-gnu
            artifact: openfoodfacts-linux-aarch64
            cross: true
          - os: macos-latest
            target: x86_64-apple-darwin
            artifact: openfoodfacts-macos-x86_64
          - os: macos-latest
            target: aarch64-apple-darwin
            artifact: openfoodfacts-macos-aarch64

    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - uses: Swatinem/rust-cache@v2

      - name: Install cross (for aarch64 Linux)
        if: matrix.cross
        run: cargo install cross --git https://github.com/cross-rs/cross

      - name: Patch version from tag
        run: |
          VERSION="${GITHUB_REF_NAME#v}"
          sed -i.bak "s/^version = \".*\"/version = \"$VERSION\"/" Cargo.toml
          rm -f Cargo.toml.bak

      - name: Build (native)
        if: '!matrix.cross'
        run: cargo build --release --target ${{ matrix.target }}

      - name: Build (cross)
        if: matrix.cross
        run: cross build --release --target ${{ matrix.target }}

      - name: Package
        run: |
          mkdir -p dist
          cp target/${{ matrix.target }}/release/openfoodfacts dist/
          tar -czf dist/${{ matrix.artifact }}.tar.gz -C dist openfoodfacts

      - uses: actions/upload-artifact@v4
        with:
          name: ${{ matrix.artifact }}
          path: dist/${{ matrix.artifact }}.tar.gz

  release:
    needs: build
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - uses: actions/download-artifact@v4
        with:
          path: artifacts
          merge-multiple: true

      - uses: softprops/action-gh-release@v2
        with:
          files: artifacts/*.tar.gz
```

- [ ] **Step 3: Write `scripts/install.sh`**

```bash
#!/usr/bin/env bash
set -euo pipefail

REPO="alfredvc/openfoodfacts-cli"
INSTALL_DIR="${HOME}/.local/bin"
BINARY="openfoodfacts"

# Detect OS and arch
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$OS" in
  linux)  OS_NAME="linux" ;;
  darwin) OS_NAME="macos" ;;
  *) echo "Unsupported OS: $OS" >&2; exit 1 ;;
esac

case "$ARCH" in
  x86_64)  ARCH_NAME="x86_64" ;;
  aarch64|arm64) ARCH_NAME="aarch64" ;;
  *) echo "Unsupported architecture: $ARCH" >&2; exit 1 ;;
esac

ARTIFACT="${BINARY}-${OS_NAME}-${ARCH_NAME}"
LATEST=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed 's/.*"tag_name": "\(.*\)".*/\1/')
URL="https://github.com/${REPO}/releases/download/${LATEST}/${ARTIFACT}.tar.gz"

echo "Installing ${BINARY} ${LATEST} for ${OS_NAME}/${ARCH_NAME}..."
mkdir -p "$INSTALL_DIR"
curl -fsSL "$URL" | tar -xz -C "$INSTALL_DIR" "$BINARY"
chmod +x "${INSTALL_DIR}/${BINARY}"
echo "Installed to ${INSTALL_DIR}/${BINARY}"

if [[ ":$PATH:" != *":${INSTALL_DIR}:"* ]]; then
  echo "Note: Add ${INSTALL_DIR} to your PATH"
fi
```

- [ ] **Step 4: Make install script executable and commit**

```bash
mkdir -p .github/workflows scripts
chmod +x scripts/install.sh
git add .github/ scripts/
git commit -m "ci: GitHub Actions CI, release workflow, install script"
```

---

## Task 8: Smoke Test + Final Verification

- [ ] **Step 1: Run all tests**

```bash
cargo test 2>&1
```
Expected: all tests pass.

- [ ] **Step 2: Build release binary**

```bash
cargo build --release 2>&1
```
Expected: builds successfully.

- [ ] **Step 3: Smoke test product get against real API**

```bash
./target/release/openfoodfacts products get 3017624010701 --fields product_name,brands,nutriscore_grade
```
Expected: JSON with `product_name`, `brands`, `nutriscore_grade` for Nutella.

- [ ] **Step 4: Smoke test search against real API**

```bash
./target/release/openfoodfacts products search --category en:chocolates --nutrition-grade a --page-size 3 --fields code,product_name,nutriscore_grade
```
Expected: JSON envelope with 3 products, all having `nutriscore_grade: "a"`.

- [ ] **Step 5: Smoke test facets against real API**

```bash
./target/release/openfoodfacts facets list categories --fields id,products 2>&1 | head -5
```
Expected: JSON array of category objects with `id` and `products` fields only.

- [ ] **Step 6: Smoke test full-text search against real API**

```bash
./target/release/openfoodfacts products search --query "organic pasta" --page-size 2 --fields code,product_name
```
Expected: JSON envelope with products.

- [ ] **Step 7: Smoke test error handling**

```bash
./target/release/openfoodfacts products get 0000000000000
```
Expected: exits 1, stderr contains `{"error": "product not found: 0000000000000"}`.

```bash
./target/release/openfoodfacts facets list bogustype
```
Expected: exits 1, stderr contains `{"error": "unknown facet type: \"bogustype\"..."}`.

- [ ] **Step 8: Commit final state**

```bash
git add -A
git commit -m "chore: release-ready"
```

---

## Notes

**`--all` with large datasets:** Fetching all pages of a large facet or broad search can make hundreds of HTTP requests. The caller is responsible for using appropriate filters to limit result size.
