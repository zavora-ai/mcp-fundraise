# Fundraise MCP Server

[![Crates.io](https://img.shields.io/crates/v/mcp-fundraise.svg)](https://crates.io/crates/mcp-fundraise)
[![Docs.rs](https://docs.rs/mcp-fundraise/badge.svg)](https://docs.rs/mcp-fundraise)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![ADK-Rust Enterprise](https://img.shields.io/badge/ADK--Rust-Enterprise-purple.svg)](https://enterprise.adk-rust.com)

Startup fundraising intelligence for AI agents. 16 tools covering SAFE/convertible note conversion, dilution modeling, valuation estimation, runway calculation, metric benchmarking, term sheet comparison, red flag detection, YC company search (2000+ companies), SEC EDGAR filings, currency conversion, and investor pipeline management. **Real financial math, real data, zero configuration.**

## Key Principles

- **Real math** — SAFE conversion handles cap, discount, MFN, and picks the better deal for the investor (as SAFEs actually work)
- **Real data** — YC companies API (2000+), SEC EDGAR (10,000+ SAFE filings), live exchange rates (166 currencies)
- **Stage-aware** — benchmarks, checklists, and multiples adjust by stage (pre-seed through Series C)
- **Founder-first** — red flag detection tells you what investors will ask before they ask it

## Tools (16)

### Financial Math (5)

| Tool | Description |
|------|-------------|
| `safe_convert` | SAFE conversion at priced round (cap, discount, or both — uses better of two) |
| `dilution_model` | Full dilution model: founders, investors, SAFE conversions, option pool shuffle |
| `valuation_estimate` | Revenue multiple valuation by stage, sector, and growth rate |
| `runway_calculate` | Months of runway with burn growth, revenue offset, zero-cash date |
| `financial_projection` | MRR/ARR projections with churn impact, month-by-month |

### Due Diligence (3)

| Tool | Description |
|------|-------------|
| `metric_benchmark` | Compare your metrics vs stage benchmarks (CAC, LTV, churn, NRR, burn multiple, etc) |
| `red_flag_check` | Identify issues investors will flag (burn multiple, concentration, growth, LTV/CAC) |
| `data_room_checklist` | Stage-appropriate document checklist for data rooms |

### Deal Analysis (2)

| Tool | Description |
|------|-------------|
| `term_compare` | Side-by-side term sheet comparison with dilution calc and flag detection |
| `currency_convert` | Multi-currency conversion for international rounds (166 currencies, live rates) |

### Live Data (2)

| Tool | Description |
|------|-------------|
| `yc_search` | Search 2000+ YC companies by batch, industry, or keyword (live API) |
| `sec_search` | Search SEC EDGAR for SAFE agreements and fundraising filings (live API) |

### Pipeline Management (4)

| Tool | Description |
|------|-------------|
| `deal_create` | Add investor to pipeline (stage, intro type, contact) |
| `deal_update` | Move deal through stages (researched through closed) |
| `pipeline_list` | Funnel view with stage counts and active deals |
| `follow_up_suggest` | AI-powered follow-up timing based on stage and days elapsed |

## Installation

```bash
cargo install mcp-fundraise
```

### Client Configuration

```json
{
  "mcpServers": {
    "fundraise": { "command": "mcp-fundraise" }
  }
}
```

## Quick Start

### SAFE Conversion

```json
{"name": "safe_convert", "arguments": {"investment": 500000, "cap": 8000000, "discount_pct": 20, "pre_money_valuation": 12000000, "shares_outstanding": 10000000}}
```

Result: Converts at $0.80/share (cap is better than discount), 625K shares, 5.88% ownership.

### Dilution Modeling

```json
{"name": "dilution_model", "arguments": {"pre_money": 15000000, "raise_amount": 5000000, "option_pool_pct": 10, "existing_shares": 10000000, "safes": [{"investment": 500000, "cap": 8000000}, {"investment": 250000, "discount_pct": 20}]}}
```

### Valuation Estimate

```json
{"name": "valuation_estimate", "arguments": {"mrr": 50000, "stage": "seed", "sector": "ai", "growth_rate_pct": 120}}
```

### Red Flag Check

```json
{"name": "red_flag_check", "arguments": {"mrr": 30000, "burn_rate": 150000, "growth_rate_pct": 8, "customer_concentration_pct": 45, "ltv_cac_ratio": 2.1}}
```

### YC Company Search

```json
{"name": "yc_search", "arguments": {"batch": "s24", "industry": "Fintech"}}
```

### Compare Term Sheets

```json
{"name": "term_compare", "arguments": {"terms": [{"investor": "Sequoia", "pre_money": 20000000, "amount": 5000000, "option_pool_pct": 10}, {"investor": "a16z", "pre_money": 18000000, "amount": 7000000, "option_pool_pct": 15, "board_seats": 2}]}}
```

## Competitive Comparison

| Feature | Carta | AngelList | Pulley | Visible | **Us** |
|---------|:-:|:-:|:-:|:-:|:-:|
| SAFE conversion | ✅ | ❌ | ✅ | ❌ | ✅ |
| Dilution modeling | ✅ | ❌ | ✅ | ❌ | ✅ |
| Valuation estimate | ❌ | ❌ | ❌ | ❌ | ✅ |
| Runway calculator | ❌ | ❌ | ❌ | ✅ | ✅ |
| Metric benchmarks | ❌ | ❌ | ❌ | ✅ | ✅ |
| Term sheet compare | ❌ | ❌ | ❌ | ❌ | ✅ |
| Red flag detection | ❌ | ❌ | ❌ | ❌ | ✅ |
| YC company search | ❌ | ❌ | ❌ | ❌ | ✅ |
| SEC filing search | ❌ | ❌ | ❌ | ❌ | ✅ |
| Investor pipeline | ❌ | ✅ | ❌ | ✅ | ✅ |
| Follow-up AI | ❌ | ❌ | ❌ | ❌ | ✅ |
| Zero config | ❌ | ❌ | ❌ | ❌ | ✅ |
| Open source | ❌ | ❌ | ❌ | ❌ | ✅ |
| MCP native | ❌ | ❌ | ❌ | ❌ | ✅ |

## License

Apache-2.0

---

Part of the [ADK-Rust Enterprise](https://enterprise.adk-rust.com) MCP server ecosystem.

Built with ❤️ by [Zavora AI](https://zavora.ai)

## rmcp and MCP compatibility

This server is built with [`rmcp` 3.1.2](https://github.com/modelcontextprotocol/rust-sdk/releases/tag/rmcp-v3.1.2) and requires Rust 1.88 or newer. The rmcp 3 rollout retains legacy MCP initialization compatibility and targets MCP protocol revisions `2025-11-25` and `2026-07-28`.
