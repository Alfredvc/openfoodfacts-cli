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
        #[arg(long, default_value = "20", value_parser = clap::value_parser!(u32).range(1..=100))]
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
