use anyhow::{Context, Result};
use clap::Parser;
use dotenv::dotenv;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
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
#[derive(Debug, Clone, Deserialize, Serialize)]
struct KeywordRow {
    #[serde(rename = "Keyword")]
    keyword: String,
    
    #[serde(rename = "Search Volume")]
    #[serde(default)]
    search_volume: Option<i32>,
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
    
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(file);
    
    // Get headers to find keyword and search volume positions
    let headers = rdr.headers().context("Failed to read CSV headers")?;
    let keyword_index = headers.iter().position(|h| h == "Keyword")
        .context("CSV must have a 'Keyword' column")?;
    
    // Search Volume column might not exist yet
    let search_volume_index = headers.iter().position(|h| h == "Search Volume");
    
    // Clone headers to avoid borrow checker issues
    let headers = headers.clone();
    
    // Store records and parsed keywords
    let mut records: Vec<csv::StringRecord> = Vec::new();
    let mut keywords: Vec<String> = Vec::new();
    
    // Read all records
    for result in rdr.records() {
        let record = result.context("Failed to read CSV row")?;
        keywords.push(record[keyword_index].to_string());
        records.push(record);
    }
    
    // Create HTTP client
    let client = Client::new();
    let endpoint = "https://api.keywordseverywhere.com/v1/get_keyword_data";
    
    println!("Fetching search volume data for {} keywords...", keywords.len());
    
    // Process keywords in batches (API limit is 100 keywords per request)
    let batch_size = 100;
    let total_batches = (keywords.len() + batch_size - 1) / batch_size;
    
    // Store the API results
    let mut volumes: std::collections::HashMap<String, Option<i32>> = std::collections::HashMap::new();
    
    for (batch_index, keyword_chunk) in keywords.chunks(batch_size).enumerate() {
        println!("Processing batch {}/{}", batch_index + 1, total_batches);
        
        // Create API request
        let mut form = std::collections::HashMap::new();
        form.insert("country", "us");
        form.insert("currency", "USD");
        form.insert("dataSource", "gkp");
        
        // Add each keyword as a separate kw[] parameter
        let mut params = Vec::new();
        for keyword in keyword_chunk {
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
            volumes.insert(kw_data.keyword.clone(), kw_data.vol);
            
            // Print volume info if verbose mode is enabled
            if args.verbose {
                let volume = kw_data.vol.map_or("N/A".to_string(), |v| v.to_string());
                println!("Keyword: {:40} | Search Volume: {}", kw_data.keyword, volume);
            }
        }
    }
    
    // Print summary of results if verbose mode is enabled
    if args.verbose {
        println!("\nSummary of Search Volumes:");
        println!("{:-^80}", "");
        println!("{:40} | {}", "Keyword", "Search Volume");
        println!("{:-^80}", "");
        
        for (keyword, volume) in &volumes {
            let volume_str = volume.map_or("N/A".to_string(), |v| v.to_string());
            println!("{:40} | {}", keyword, volume_str);
        }
        println!("{:-^80}", "");
    }
    
    // Determine output file path (use input file if output not specified)
    let output_path = args.output.unwrap_or_else(|| args.input.clone());
    
    // Write updated data to CSV
    let output_file = File::create(&output_path)
        .with_context(|| format!("Failed to create output file: {}", output_path.display()))?;
    
    // Create a CSV writer
    let mut wtr = csv::WriterBuilder::new()
        .from_writer(output_file);
    
    // Create a new headers row with "Search Volume" if it doesn't exist
    let mut new_headers = headers.clone();
    if search_volume_index.is_none() {
        new_headers.push_field("Search Volume");
    }
    
    // Write the headers
    wtr.write_record(&new_headers)?;
    
    // Write all records with updated search volume
    for record in records {
        let keyword = &record[keyword_index];
        
        if let Some(sv_index) = search_volume_index {
            // If Search Volume column already exists, update it
            let mut new_record = record.clone();
            if let Some(volume) = volumes.get(keyword) {
                if let Some(vol) = volume {
                    // Create a completely new record as StringRecord doesn't have a get_mut method
                    let mut fields: Vec<String> = new_record.iter().map(|s| s.to_string()).collect();
                    fields[sv_index] = vol.to_string();
                    new_record = csv::StringRecord::from(fields);
                }
            }
            wtr.write_record(&new_record)?;
        } else {
            // If Search Volume column doesn't exist, add it
            let mut new_record = record.clone();
            if let Some(volume) = volumes.get(keyword) {
                if let Some(vol) = volume {
                    new_record.push_field(&vol.to_string());
                } else {
                    new_record.push_field("");
                }
            } else {
                new_record.push_field("");
            }
            wtr.write_record(&new_record)?;
        }
    }
    
    wtr.flush().context("Failed to flush CSV writer")?;
    
    println!("Successfully updated search volumes and saved to: {}", output_path.display());
    Ok(())
}
