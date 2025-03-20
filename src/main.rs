use anyhow::{Context, Result};
use clap::Parser;
use csv::{Reader, Writer};
use dotenv::dotenv;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cmp::min;
use std::env;
use std::fs::File;
use std::path::PathBuf;

// Command line arguments for the CLI tool
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input CSV file path
    #[arg(short, long)]
    input: PathBuf,

    /// Output CSV file path (defaults to overwriting input file if not specified)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Print detailed results to console
    #[arg(short, long)]
    verbose: bool,
}

// Structure for CSV input rows
#[derive(Debug, Deserialize, Serialize)]
struct KeywordRow {
    #[serde(rename = "Keyword")]
    keyword: String,
    
    #[serde(rename = "Search Volume")]
    #[serde(default)]
    search_volume: Option<i32>,
    // Add any other columns from your CSV here
}

// Keywords Everywhere API response structure
#[derive(Debug, Deserialize)]
struct ApiResponse {
    data: Vec<KeywordData>,
    credits: Option<i64>,
    time: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct KeywordData {
    vol: Option<i32>,
    keyword: String,
    #[serde(default)]
    cpc: Option<Cpc>,
    competition: Option<f64>,
    #[serde(default)]
    trend: Vec<TrendData>,
}

#[derive(Debug, Deserialize)]
struct Cpc {
    currency: String,
    value: String,
}

#[derive(Debug, Deserialize)]
struct TrendData {
    month: String,
    year: i32,
    value: i32,
}

fn main() -> Result<()> {
    // Load environment variables from .env file
    // This will not override existing environment variables
    dotenv().ok();
    
    // Parse command line arguments
    let args = Args::parse();
    
    // Get API key from environment
    let api_key = env::var("KEYWORDS_EVERYWHERE_API_KEY")
        .context("KEYWORDS_EVERYWHERE_API_KEY not found in environment variables. Make sure to set it or create a .env file.")?;
    
    println!("Using API key: {}...", &api_key[0..min(5, api_key.len())]);
    
    // Open and read the CSV file
    let file = File::open(&args.input)
        .with_context(|| format!("Failed to open input file: {}", args.input.display()))?;
    
    let mut rdr = Reader::from_reader(file);
    let mut keywords_data: Vec<KeywordRow> = Vec::new();
    
    // Collect all rows from the CSV
    for result in rdr.deserialize() {
        let record: KeywordRow = result.context("Failed to read CSV row")?;
        keywords_data.push(record);
    }
    
    // Create HTTP client
    let client = Client::new();
    let endpoint = "https://api.keywordseverywhere.com/v1/get_keyword_data";
    
    println!("Fetching search volume data for {} keywords...", keywords_data.len());
    
    // Process keywords in batches (API limit is 100 keywords per request)
    let batch_size = 100;
    let total_batches = (keywords_data.len() + batch_size - 1) / batch_size;
    
    for (batch_index, chunk) in keywords_data.chunks_mut(batch_size).enumerate() {
        println!("Processing batch {}/{}", batch_index + 1, total_batches);
        
        // Extract keywords for this batch
        let batch_keywords: Vec<String> = chunk.iter()
            .map(|row| row.keyword.clone())
            .collect();
            
        // Create API request
        let mut form = std::collections::HashMap::new();
        form.insert("country", "us");
        form.insert("currency", "USD");
        form.insert("dataSource", "gkp");
        
        // Add each keyword as a separate kw[] parameter
        let mut params = Vec::new();
        for keyword in &batch_keywords {
            params.push(("kw[]", keyword));
        }
        
        let response = client.post(endpoint)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Accept", "application/json")
            .form(&params)
            .send()
            .context("Failed to send request to Keywords Everywhere API")?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text()?;
            return Err(anyhow::anyhow!(
                "API request failed with status code: {}. Error: {}", 
                status, 
                error_text
            ));
        }
        
        // Get the raw response text for verbose mode
        let response_text = response.text()?;
        
        if args.verbose {
            println!("\nRaw API Response:\n{}", response_text);
        }
        
        let api_data: ApiResponse = serde_json::from_str(&response_text)
            .context("Failed to parse API response as JSON")?;
        
        // Update search volume for each keyword in the batch
        for kw_data in api_data.data {
            // Find the corresponding row in our data
            if let Some(row) = chunk.iter_mut().find(|r| r.keyword == kw_data.keyword) {
                row.search_volume = kw_data.vol;
                
                // Print volume info if verbose mode is enabled
                if args.verbose {
                    let volume = kw_data.vol.map_or("N/A".to_string(), |v| v.to_string());
                    println!("Keyword: {:40} | Search Volume: {}", kw_data.keyword, volume);
                }
            }
        }
    }
    
    // Print summary of results if verbose mode is enabled
    if args.verbose {
        println!("\nSummary of Search Volumes:");
        println!("{:-^80}", "");
        println!("{:40} | {}", "Keyword", "Search Volume");
        println!("{:-^80}", "");
        
        for row in &keywords_data {
            let volume = row.search_volume.map_or("N/A".to_string(), |v| v.to_string());
            println!("{:40} | {}", row.keyword, volume);
        }
        println!("{:-^80}", "");
    }
    
    // Determine output file path (use input file if output not specified)
    let output_path = args.output.unwrap_or_else(|| args.input.clone());
    
    // Write updated data to CSV
    let output_file = File::create(&output_path)
        .with_context(|| format!("Failed to create output file: {}", output_path.display()))?;
    
    let mut wtr = Writer::from_writer(output_file);
    
    for row in keywords_data {
        wtr.serialize(row).context("Failed to write row to CSV")?;
    }
    
    wtr.flush().context("Failed to flush CSV writer")?;
    
    println!("Successfully updated search volumes and saved to: {}", output_path.display());
    Ok(())
}
