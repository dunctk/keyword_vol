# Keyword Volume CLI

A command-line tool that enriches CSV files with search volume data for keywords using the Keywords Everywhere API.

## Features

- Batch processing of keywords (100 per API request)
- CSV input/output with automatic column mapping
- Detailed logging with verbose mode
- Environment variable configuration for API keys

## Installation

### Prerequisites

- Rust and Cargo (install via [rustup](https://rustup.rs/))
- Keywords Everywhere API key [get here](https://keywordseverywhere.com/api-documentation.html)

### Building from source

```bash
# Clone the repository
git clone https://github.com/dunctk/keyword_vol.git
cd keyword_vol

# Build the project
cargo build --release
```

The compiled binary will be available at `target/release/keyword_vol`.

## Usage

```bash
# Basic usage
keyword_vol --input keywords.csv --output enriched_keywords.csv

# Overwrite input file
keyword_vol --input keywords.csv

# Show detailed output
keyword_vol --input keywords.csv --verbose
```

### Input CSV Format

The tool expects a CSV file with at least a "Keyword" column. Other columns will be preserved.

Example:
```csv
Keyword,Some Other Data
rust programming,additional info
learn rust,more info
```

### API Key Configuration

Create a `.env` file in the project directory:
```
KEYWORDS_EVERYWHERE_API_KEY=your_api_key_here
```

Or set the environment variable directly:
```bash
export KEYWORDS_EVERYWHERE_API_KEY=your_api_key_here
```

## License

MIT License

Copyright (c) 2023 [Your Name]

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE. 