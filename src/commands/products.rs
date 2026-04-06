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
    if let Some(v) = ecoscore_grade { params.push(("ecoscore_grade_tags", v)); }
    if let Some(v) = label { params.push(("labels_tags", v)); }
    if let Some(v) = ingredient { params.push(("ingredients_tags", v)); }
    if let Some(v) = allergen { params.push(("allergens_tags", v)); }
    if let Some(v) = sort_by { params.push(("sort_by", v)); }

    if all {
        let all_products = fetch_all_pages("/api/v2/search", &params, client).await?;
        output.print(&Value::Array(all_products));
    } else {
        let body = client.get("/api/v2/search", &params).await?;
        output.print(&body);
    }
    Ok(())
}

async fn fetch_all_pages(path: &str, base_params: &[(&str, &str)], client: &Client) -> Result<Vec<Value>> {
    let filtered_params: Vec<(&str, &str)> = base_params
        .iter()
        .filter(|(k, _)| *k != "page")
        .copied()
        .collect();

    let page_str_1 = "1".to_string();
    let mut params = filtered_params.clone();
    params.push(("page", &page_str_1));

    let first = client.get(path, &params).await?;
    let page_count = first
        .get("page_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(1);
    let mut all: Vec<Value> = extract_products(&first);

    for p in 2..=page_count {
        let page_num = p.to_string();
        let mut page_params = filtered_params.clone();
        page_params.push(("page", &page_num));
        let body = client.get(path, &page_params).await?;
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
        let all_products = fetch_all_pages("/cgi/search.pl", &params, client).await?;
        output.print(&Value::Array(all_products));
    } else {
        let body = client.get("/cgi/search.pl", &params).await?;
        output.print(&body);
    }
    Ok(())
}
