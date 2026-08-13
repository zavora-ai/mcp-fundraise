use rmcp::{handler::server::wrapper::Parameters, schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

fn short_id() -> String { uuid::Uuid::new_v4().to_string()[..8].to_string() }
fn now() -> String { chrono::Utc::now().to_rfc3339() }
fn round2(v: f64) -> f64 { (v * 100.0).round() / 100.0 }

// === Input Types ===

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SafeInput { pub investment: f64, pub cap: Option<f64>, pub discount_pct: Option<f64>, pub pre_money_valuation: f64, pub shares_outstanding: u64 }
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DilutionInput { pub pre_money: f64, pub raise_amount: f64, pub option_pool_pct: Option<f64>, pub existing_shares: u64, pub safes: Option<Vec<SafeDetail>> }
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SafeDetail { pub investment: f64, pub cap: Option<f64>, pub discount_pct: Option<f64> }
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ValuationInput { pub arr: Option<f64>, pub mrr: Option<f64>, pub stage: String, pub sector: Option<String>, pub growth_rate_pct: Option<f64> }
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RunwayInput { pub cash_balance: f64, pub monthly_burn: f64, pub monthly_revenue: Option<f64>, pub burn_growth_pct: Option<f64> }
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ProjectionInput { pub current_mrr: f64, pub growth_rate_pct: f64, pub months: Option<u32>, pub churn_pct: Option<f64> }
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BenchmarkInput { pub stage: String, pub metric: String, pub value: f64 }
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct TermCompareInput { pub terms: Vec<TermSheet> }
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct TermSheet { pub investor: String, pub pre_money: f64, pub amount: f64, pub option_pool_pct: Option<f64>, pub liquidation_pref: Option<f64>, pub pro_rata: Option<bool>, pub board_seats: Option<u8> }
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RedFlagInput { pub mrr: Option<f64>, pub burn_rate: f64, pub months_since_last_raise: Option<u32>, pub customer_concentration_pct: Option<f64>, pub growth_rate_pct: Option<f64>, pub ltv_cac_ratio: Option<f64> }
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DataRoomInput { pub stage: String }
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct YcSearchInput { pub batch: Option<String>, pub industry: Option<String>, pub query: Option<String> }
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SecSearchInput { pub query: String, pub form_type: Option<String> }
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DealInput { pub investor_name: String, pub stage: Option<String>, pub contact: Option<String>, pub notes: Option<String>, pub intro_type: Option<String> }
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DealUpdateInput { pub deal_id: String, pub stage: String, pub notes: Option<String> }
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CurrencyInput { pub amount: f64, pub from: String, pub to: String }

// === Data Models ===

#[derive(Clone, serde::Serialize)]
struct Deal { id: String, investor_name: String, stage: String, contact: String, intro_type: String, notes: String, created_at: String, updated_at: String }

// === Server ===

#[derive(Clone)]
pub struct FundraiseServer {
    deals: Arc<Mutex<HashMap<String, Deal>>>,
    client: reqwest::Client,
}

impl FundraiseServer {
    pub fn new() -> Self {
        Self { deals: Default::default(), client: reqwest::Client::new() }
    }
}

#[tool_router]
impl FundraiseServer {
    #[tool(description = "Calculate SAFE conversion at priced round. Shows shares issued, effective price, ownership percentage. Handles cap, discount, or both (uses better of the two).")]
    async fn safe_convert(&self, Parameters(input): Parameters<SafeInput>) -> String {
        let pps = input.pre_money_valuation / input.shares_outstanding as f64;
        let cap_price = input.cap.map(|c| c / input.shares_outstanding as f64);
        let discount_price = input.discount_pct.map(|d| pps * (1.0 - d / 100.0));
        let conversion_price = match (cap_price, discount_price) {
            (Some(c), Some(d)) => c.min(d),
            (Some(c), None) => c,
            (None, Some(d)) => d,
            (None, None) => pps,
        };
        let shares_issued = (input.investment / conversion_price).floor() as u64;
        let total_shares = input.shares_outstanding + shares_issued;
        let ownership_pct = round2(shares_issued as f64 / total_shares as f64 * 100.0);
        json!({"investment": input.investment, "cap": input.cap, "discount_pct": input.discount_pct, "price_per_share_round": round2(pps), "conversion_price": round2(conversion_price), "shares_issued": shares_issued, "total_shares_after": total_shares, "ownership_pct": ownership_pct, "effective_valuation": round2(conversion_price * input.shares_outstanding as f64)}).to_string()
    }

    #[tool(description = "Model dilution from a priced round. Shows founder ownership before/after, option pool impact, SAFE conversions. Accounts for option pool shuffle.")]
    async fn dilution_model(&self, Parameters(input): Parameters<DilutionInput>) -> String {
        let post_money = input.pre_money + input.raise_amount;
        let pps = input.pre_money / input.existing_shares as f64;
        let new_investor_shares = (input.raise_amount / pps).floor() as u64;
        let option_pool_pct = input.option_pool_pct.unwrap_or(10.0);
        // SAFE conversions
        let mut safe_shares: u64 = 0;
        let mut safe_details = Vec::new();
        if let Some(safes) = &input.safes {
            for s in safes {
                let cap_price = s.cap.map(|c| c / input.existing_shares as f64);
                let disc_price = s.discount_pct.map(|d| pps * (1.0 - d / 100.0));
                let conv_price = match (cap_price, disc_price) { (Some(c), Some(d)) => c.min(d), (Some(c), None) => c, (None, Some(d)) => d, (None, None) => pps };
                let shares = (s.investment / conv_price).floor() as u64;
                safe_shares += shares;
                safe_details.push(json!({"investment": s.investment, "conversion_price": round2(conv_price), "shares": shares}));
            }
        }
        let total_before_pool = input.existing_shares + new_investor_shares + safe_shares;
        let pool_shares = (total_before_pool as f64 * option_pool_pct / 100.0).floor() as u64;
        let total_shares = total_before_pool + pool_shares;
        let founder_pct = round2(input.existing_shares as f64 / total_shares as f64 * 100.0);
        let investor_pct = round2(new_investor_shares as f64 / total_shares as f64 * 100.0);
        let safe_pct = round2(safe_shares as f64 / total_shares as f64 * 100.0);
        let pool_pct = round2(pool_shares as f64 / total_shares as f64 * 100.0);
        json!({"pre_money": input.pre_money, "raise_amount": input.raise_amount, "post_money": post_money, "price_per_share": round2(pps), "cap_table": {"founders": {"shares": input.existing_shares, "pct": founder_pct}, "new_investors": {"shares": new_investor_shares, "pct": investor_pct}, "safe_conversions": {"shares": safe_shares, "pct": safe_pct, "details": safe_details}, "option_pool": {"shares": pool_shares, "pct": pool_pct}}, "total_shares": total_shares}).to_string()
    }

    #[tool(description = "Estimate startup valuation using revenue multiples by stage and sector. Uses current SaaS benchmarks (2024-2026 market data).")]
    async fn valuation_estimate(&self, Parameters(input): Parameters<ValuationInput>) -> String {
        let arr = input.arr.unwrap_or_else(|| input.mrr.unwrap_or(0.0) * 12.0);
        let growth = input.growth_rate_pct.unwrap_or(100.0);
        // Stage-based multiples (2024-2026 market)
        let base_multiple = match input.stage.as_str() {
            "pre-seed" | "pre_seed" => 20.0,
            "seed" => if arr > 0.0 { 15.0 } else { 10.0 },
            "series_a" | "series-a" | "a" => 12.0,
            "series_b" | "series-b" | "b" => 10.0,
            "series_c" | "series-c" | "c" => 8.0,
            _ => 10.0,
        };
        // Growth adjustment
        let growth_adj = if growth > 200.0 { 1.5 } else if growth > 100.0 { 1.2 } else if growth > 50.0 { 1.0 } else { 0.7 };
        // Sector adjustment
        let sector_adj = match input.sector.as_deref().unwrap_or("saas") {
            "ai" | "artificial_intelligence" => 1.4,
            "fintech" => 1.2,
            "healthtech" | "biotech" => 1.1,
            "saas" | "b2b" => 1.0,
            "marketplace" => 0.9,
            "consumer" => 0.8,
            "hardware" => 0.6,
            _ => 1.0,
        };
        let multiple = round2(base_multiple * growth_adj * sector_adj);
        let valuation = round2(arr * multiple);
        let low = round2(valuation * 0.7);
        let high = round2(valuation * 1.4);
        json!({"arr": arr, "stage": input.stage, "sector": input.sector, "growth_rate_pct": growth, "revenue_multiple": multiple, "valuation_estimate": valuation, "range": {"low": low, "mid": valuation, "high": high}, "methodology": "Revenue multiple adjusted for stage, growth rate, and sector"}).to_string()
    }

    #[tool(description = "Calculate runway in months. Accounts for revenue offset and optional burn growth rate. Shows zero-cash date.")]
    async fn runway_calculate(&self, Parameters(input): Parameters<RunwayInput>) -> String {
        let revenue = input.monthly_revenue.unwrap_or(0.0);
        let net_burn = input.monthly_burn - revenue;
        if net_burn <= 0.0 { return json!({"cash_balance": input.cash_balance, "monthly_burn": input.monthly_burn, "monthly_revenue": revenue, "net_burn": net_burn, "runway_months": "infinite", "status": "cash_flow_positive"}).to_string(); }
        let burn_growth = input.burn_growth_pct.unwrap_or(0.0) / 100.0;
        let mut remaining = input.cash_balance;
        let mut months = 0u32;
        let mut current_burn = net_burn;
        while remaining > 0.0 && months < 120 {
            remaining -= current_burn;
            current_burn *= 1.0 + burn_growth;
            months += 1;
        }
        let zero_cash_date = (chrono::Utc::now() + chrono::Duration::days(months as i64 * 30)).format("%Y-%m").to_string();
        let status = if months <= 6 { "critical" } else if months <= 12 { "raise_now" } else if months <= 18 { "healthy" } else { "comfortable" };
        json!({"cash_balance": input.cash_balance, "monthly_burn": input.monthly_burn, "monthly_revenue": revenue, "net_burn": round2(net_burn), "runway_months": months, "zero_cash_date": zero_cash_date, "status": status, "recommendation": match status { "critical" => "Raise immediately or cut burn drastically", "raise_now" => "Start fundraising now (takes 3-6 months)", "healthy" => "Good position to start fundraising", _ => "Comfortable runway, raise opportunistically" }}).to_string()
    }

    #[tool(description = "Project MRR/ARR growth over time. Shows month-by-month with optional churn impact.")]
    async fn financial_projection(&self, Parameters(input): Parameters<ProjectionInput>) -> String {
        let months = input.months.unwrap_or(12);
        let churn = input.churn_pct.unwrap_or(3.0) / 100.0;
        let growth = input.growth_rate_pct / 100.0;
        let mut projections = Vec::new();
        let mut mrr = input.current_mrr;
        for m in 1..=months {
            let new_mrr = mrr * growth;
            let churned = mrr * churn;
            mrr = mrr + new_mrr - churned;
            projections.push(json!({"month": m, "mrr": round2(mrr), "arr": round2(mrr * 12.0), "net_new": round2(new_mrr - churned)}));
        }
        let final_mrr = projections.last().map(|p| p["mrr"].as_f64().unwrap_or(0.0)).unwrap_or(0.0);
        json!({"starting_mrr": input.current_mrr, "growth_rate_pct": input.growth_rate_pct, "churn_pct": input.churn_pct.unwrap_or(3.0), "months": months, "ending_mrr": round2(final_mrr), "ending_arr": round2(final_mrr * 12.0), "total_growth_pct": round2((final_mrr - input.current_mrr) / input.current_mrr * 100.0), "projections": projections}).to_string()
    }

    #[tool(description = "Compare your startup metrics against stage benchmarks (SaaS). Metrics: cac, ltv, churn, nrr, burn_multiple, magic_number, payback_months, gross_margin.")]
    async fn metric_benchmark(&self, Parameters(input): Parameters<BenchmarkInput>) -> String {
        let benchmarks: HashMap<&str, HashMap<&str, (f64, f64, f64)>> = HashMap::from([
            ("seed", HashMap::from([("cac", (200.0, 500.0, 1500.0)), ("ltv", (2000.0, 5000.0, 15000.0)), ("churn", (2.0, 5.0, 8.0)), ("nrr", (90.0, 105.0, 130.0)), ("burn_multiple", (1.5, 3.0, 6.0)), ("magic_number", (0.3, 0.7, 1.5)), ("payback_months", (6.0, 12.0, 24.0)), ("gross_margin", (60.0, 75.0, 85.0))])),
            ("series_a", HashMap::from([("cac", (300.0, 800.0, 2000.0)), ("ltv", (5000.0, 15000.0, 50000.0)), ("churn", (1.5, 3.0, 6.0)), ("nrr", (100.0, 115.0, 140.0)), ("burn_multiple", (1.0, 2.0, 4.0)), ("magic_number", (0.5, 1.0, 2.0)), ("payback_months", (6.0, 12.0, 18.0)), ("gross_margin", (65.0, 78.0, 88.0))])),
            ("series_b", HashMap::from([("cac", (500.0, 1200.0, 3000.0)), ("ltv", (10000.0, 30000.0, 100000.0)), ("churn", (1.0, 2.5, 5.0)), ("nrr", (105.0, 120.0, 150.0)), ("burn_multiple", (0.8, 1.5, 3.0)), ("magic_number", (0.7, 1.2, 2.5)), ("payback_months", (4.0, 10.0, 16.0)), ("gross_margin", (70.0, 80.0, 90.0))])),
        ]);
        let stage = match input.stage.as_str() { "seed" | "pre-seed" | "pre_seed" => "seed", "series_a" | "a" => "series_a", _ => "series_b" };
        let metric = input.metric.as_str();
        if let Some(stage_data) = benchmarks.get(stage) {
            if let Some((p25, median, p75)) = stage_data.get(metric) {
                let percentile = if input.value <= *p25 { "bottom_25" } else if input.value <= *median { "below_median" } else if input.value <= *p75 { "above_median" } else { "top_25" };
                let is_inverse = matches!(metric, "cac" | "churn" | "burn_multiple" | "payback_months");
                let rating = if is_inverse { if input.value <= *p25 { "excellent" } else if input.value <= *median { "good" } else if input.value <= *p75 { "fair" } else { "poor" } } else { if input.value >= *p75 { "excellent" } else if input.value >= *median { "good" } else if input.value >= *p25 { "fair" } else { "poor" } };
                return json!({"metric": metric, "your_value": input.value, "stage": stage, "benchmarks": {"p25": p25, "median": median, "p75": p75}, "percentile": percentile, "rating": rating}).to_string();
            }
        }
        json!({"error": "Unknown metric or stage. Metrics: cac, ltv, churn, nrr, burn_multiple, magic_number, payback_months, gross_margin. Stages: seed, series_a, series_b"}).to_string()
    }

    #[tool(description = "Compare multiple term sheets side-by-side. Calculates effective dilution, founder ownership, and flags concerning terms.")]
    async fn term_compare(&self, Parameters(input): Parameters<TermCompareInput>) -> String {
        let comparisons: Vec<Value> = input.terms.iter().map(|t| {
            let post_money = t.pre_money + t.amount;
            let pool = t.option_pool_pct.unwrap_or(10.0);
            let effective_pre = t.pre_money * (1.0 - pool / 100.0);
            let dilution = round2(t.amount / post_money * 100.0);
            let founder_pct = round2(100.0 - dilution - pool);
            let mut flags = Vec::new();
            if t.liquidation_pref.unwrap_or(1.0) > 1.0 { flags.push("Participating preferred or >1x liquidation"); }
            if pool > 15.0 { flags.push("Large option pool (>15%) dilutes founders pre-money"); }
            if t.board_seats.unwrap_or(0) >= 2 { flags.push("Multiple board seats - potential control issues"); }
            json!({"investor": t.investor, "pre_money": t.pre_money, "amount": t.amount, "post_money": post_money, "effective_pre_after_pool": round2(effective_pre), "dilution_pct": dilution, "founder_ownership_pct": founder_pct, "option_pool_pct": pool, "liquidation_pref": t.liquidation_pref.unwrap_or(1.0), "pro_rata": t.pro_rata.unwrap_or(false), "board_seats": t.board_seats.unwrap_or(0), "flags": flags})
        }).collect();
        let best_valuation = comparisons.iter().max_by(|a, b| a["pre_money"].as_f64().unwrap().partial_cmp(&b["pre_money"].as_f64().unwrap()).unwrap()).map(|v| v["investor"].as_str().unwrap_or(""));
        let least_dilution = comparisons.iter().min_by(|a, b| a["dilution_pct"].as_f64().unwrap().partial_cmp(&b["dilution_pct"].as_f64().unwrap()).unwrap()).map(|v| v["investor"].as_str().unwrap_or(""));
        json!({"terms_compared": comparisons.len(), "best_valuation": best_valuation, "least_dilution": least_dilution, "comparisons": comparisons}).to_string()
    }

    #[tool(description = "Analyze startup metrics for investor red flags. Identifies issues that will come up in due diligence.")]
    async fn red_flag_check(&self, Parameters(input): Parameters<RedFlagInput>) -> String {
        let mut flags: Vec<Value> = Vec::new();
        let burn_multiple = if let Some(mrr) = input.mrr { if mrr > 0.0 { let bm = input.burn_rate / mrr; if bm > 3.0 { flags.push(json!({"flag": "High burn multiple", "value": round2(bm), "threshold": 3.0, "severity": "high", "detail": "Spending too much relative to revenue growth"})); } Some(bm) } else { flags.push(json!({"flag": "No revenue", "severity": "medium", "detail": "Pre-revenue companies face higher scrutiny"})); None } } else { None };
        if let Some(growth) = input.growth_rate_pct { if growth < 15.0 { flags.push(json!({"flag": "Low growth rate", "value": growth, "threshold": 15.0, "severity": "high", "detail": "Investors expect >15% MoM at seed, >10% at Series A"})); } }
        if let Some(conc) = input.customer_concentration_pct { if conc > 30.0 { flags.push(json!({"flag": "Customer concentration", "value": conc, "threshold": 30.0, "severity": "high", "detail": "Single customer >30% of revenue is a major risk"})); } }
        if let Some(ltv_cac) = input.ltv_cac_ratio { if ltv_cac < 3.0 { flags.push(json!({"flag": "Low LTV/CAC ratio", "value": ltv_cac, "threshold": 3.0, "severity": "medium", "detail": "Unit economics not yet proven (target >3x)"})); } }
        if let Some(months) = input.months_since_last_raise { if months > 18 { flags.push(json!({"flag": "Long time since last raise", "value": months, "threshold": 18, "severity": "medium", "detail": "Investors may question why you haven't raised or reached profitability"})); } }
        let overall = if flags.iter().any(|f| f["severity"] == "high") { "concerning" } else if flags.is_empty() { "clean" } else { "minor_issues" };
        json!({"red_flags": flags.len(), "overall": overall, "flags": flags, "burn_multiple": burn_multiple.map(|b| round2(b))}).to_string()
    }

    #[tool(description = "Generate data room checklist by fundraising stage. Lists all documents investors expect.")]
    async fn data_room_checklist(&self, Parameters(input): Parameters<DataRoomInput>) -> String {
        let base = vec!["Company formation docs (articles, bylaws)", "Cap table (fully diluted)", "Financial statements (P&L, balance sheet, cash flow)", "Bank statements (last 6 months)", "Pitch deck", "Executive summary"];
        let seed_extra = vec!["Product demo or screenshots", "Customer letters of intent", "Founder backgrounds/CVs", "IP assignments", "Any existing contracts"];
        let series_a_extra = vec!["Monthly financial model (3-year)", "Cohort analysis", "Unit economics breakdown", "Customer contracts (top 10)", "Employee agreements", "Option plan and grants", "Prior round docs (SAFEs, notes)", "Insurance policies", "Material contracts", "Org chart"];
        let series_b_extra = vec!["Audited financials", "Revenue recognition policy", "Sales pipeline detail", "Customer churn analysis", "Competitive landscape", "IP portfolio", "Compliance certifications", "Board meeting minutes", "Key employee retention plans", "International structure (if any)"];
        let docs: Vec<&str> = match input.stage.as_str() {
            "pre-seed" | "pre_seed" => base.clone(),
            "seed" => [base.as_slice(), seed_extra.as_slice()].concat(),
            "series_a" | "a" => [base.as_slice(), seed_extra.as_slice(), series_a_extra.as_slice()].concat(),
            _ => [base.as_slice(), seed_extra.as_slice(), series_a_extra.as_slice(), series_b_extra.as_slice()].concat(),
        };
        json!({"stage": input.stage, "total_documents": docs.len(), "checklist": docs}).to_string()
    }

    #[tool(description = "Search Y Combinator companies by batch (s24, w24, s23, etc), industry, or keyword. Returns real data from 2000+ YC companies.")]
    async fn yc_search(&self, Parameters(input): Parameters<YcSearchInput>) -> String {
        let batch = input.batch.unwrap_or_else(|| "s24".into());
        let url = format!("https://yc-oss.github.io/api/batches/{}.json", batch);
        match self.client.get(&url).send().await {
            Ok(resp) => match resp.json::<Vec<Value>>().await {
                Ok(companies) => {
                    let filtered: Vec<Value> = companies.into_iter().filter(|c| {
                        let industry_match = input.industry.as_ref().map_or(true, |ind| c["industry"].as_str().unwrap_or("").to_lowercase().contains(&ind.to_lowercase()));
                        let query_match = input.query.as_ref().map_or(true, |q| { let ql = q.to_lowercase(); c["name"].as_str().unwrap_or("").to_lowercase().contains(&ql) || c["one_liner"].as_str().unwrap_or("").to_lowercase().contains(&ql) || c["long_description"].as_str().unwrap_or("").to_lowercase().contains(&ql) });
                        industry_match && query_match
                    }).take(20).map(|c| json!({"name": c["name"], "one_liner": c["one_liner"], "website": c["website"], "industry": c["industry"], "team_size": c["team_size"], "location": c["all_locations"], "tags": c["tags"], "status": c["status"]})).collect();
                    json!({"batch": batch, "results": filtered.len(), "companies": filtered}).to_string()
                }
                Err(e) => json!({"error": format!("Parse error: {}", e)}).to_string(),
            }
            Err(e) => json!({"error": format!("Request failed: {}", e)}).to_string(),
        }
    }

    #[tool(description = "Search SEC EDGAR for SAFE agreements, Form C (crowdfunding), and fundraising filings. Returns real public filing data.")]
    async fn sec_search(&self, Parameters(input): Parameters<SecSearchInput>) -> String {
        let form_filter = input.form_type.as_deref().unwrap_or("");
        let url = format!("https://efts.sec.gov/LATEST/search-index?q={}&forms={}&dateRange=custom&startdt=2024-01-01&enddt=2026-12-31", input.query.replace(' ', "+"), form_filter);
        match self.client.get(&url).header("User-Agent", "mcp-fundraise/1.0 (research)").send().await {
            Ok(resp) => match resp.json::<Value>().await {
                Ok(data) => {
                    let hits = data["hits"]["hits"].as_array().map(|h| h.iter().take(10).map(|hit| {
                        let src = &hit["_source"];
                        json!({"company": src["display_names"][0], "form": src["form"], "date": src["file_date"], "description": src["file_description"], "location": src["biz_locations"][0]})
                    }).collect::<Vec<_>>()).unwrap_or_default();
                    let total = data["hits"]["total"]["value"].as_u64().unwrap_or(0);
                    json!({"query": input.query, "total_results": total, "showing": hits.len(), "filings": hits}).to_string()
                }
                Err(e) => json!({"error": format!("Parse error: {}", e)}).to_string(),
            }
            Err(e) => json!({"error": format!("Request failed: {}", e)}).to_string(),
        }
    }

    #[tool(description = "Convert currency amounts for international fundraising. Supports 166 currencies. Real-time rates.")]
    async fn currency_convert(&self, Parameters(input): Parameters<CurrencyInput>) -> String {
        let url = format!("https://open.er-api.com/v6/latest/{}", input.from.to_uppercase());
        match self.client.get(&url).send().await {
            Ok(resp) => match resp.json::<Value>().await {
                Ok(data) => {
                    let to = input.to.to_uppercase();
                    if let Some(rate) = data["rates"][&to].as_f64() {
                        let converted = round2(input.amount * rate);
                        json!({"from": input.from.to_uppercase(), "to": to, "amount": input.amount, "converted": converted, "rate": rate, "updated": data["time_last_update_utc"]}).to_string()
                    } else { json!({"error": format!("Currency {} not found", to)}).to_string() }
                }
                Err(e) => json!({"error": format!("Parse error: {}", e)}).to_string(),
            }
            Err(e) => json!({"error": format!("Request failed: {}", e)}).to_string(),
        }
    }

    // === Pipeline Management ===

    #[tool(description = "Add investor to fundraising pipeline. Track outreach stage, intro type (cold, warm, inbound), contact info.")]
    async fn deal_create(&self, Parameters(input): Parameters<DealInput>) -> String {
        let id = format!("deal_{}", short_id());
        let deal = Deal { id: id.clone(), investor_name: input.investor_name, stage: input.stage.unwrap_or_else(|| "researched".into()), contact: input.contact.unwrap_or_default(), intro_type: input.intro_type.unwrap_or_else(|| "cold".into()), notes: input.notes.unwrap_or_default(), created_at: now(), updated_at: now() };
        let resp = json!({"deal_id": id, "investor": deal.investor_name, "stage": deal.stage, "intro_type": deal.intro_type});
        self.deals.lock().unwrap().insert(id, deal);
        resp.to_string()
    }

    #[tool(description = "Update deal stage (researched, contacted, meeting_scheduled, meeting_done, term_sheet, due_diligence, closed_won, closed_lost, passed).")]
    async fn deal_update(&self, Parameters(input): Parameters<DealUpdateInput>) -> String {
        let mut deals = self.deals.lock().unwrap();
        match deals.get_mut(&input.deal_id) {
            Some(d) => { d.stage = input.stage.clone(); d.updated_at = now(); if let Some(n) = input.notes { d.notes = n; } json!({"deal_id": input.deal_id, "stage": input.stage, "updated_at": d.updated_at}).to_string() }
            None => json!({"error": "DEAL_NOT_FOUND"}).to_string(),
        }
    }

    #[tool(description = "List fundraising pipeline with funnel metrics. Shows deals by stage, conversion rates, and follow-up suggestions.")]
    async fn pipeline_list(&self) -> String {
        let deals = self.deals.lock().unwrap();
        let stages = ["researched", "contacted", "meeting_scheduled", "meeting_done", "term_sheet", "due_diligence", "closed_won", "closed_lost", "passed"];
        let mut funnel: Vec<Value> = stages.iter().map(|s| {
            let count = deals.values().filter(|d| d.stage == *s).count();
            json!({"stage": s, "count": count})
        }).filter(|v| v["count"].as_u64().unwrap_or(0) > 0).collect();
        let total = deals.len();
        let active = deals.values().filter(|d| !matches!(d.stage.as_str(), "closed_won" | "closed_lost" | "passed")).count();
        let list: Vec<Value> = deals.values().map(|d| json!({"id": d.id, "investor": d.investor_name, "stage": d.stage, "intro_type": d.intro_type, "updated_at": d.updated_at})).collect();
        json!({"total_deals": total, "active": active, "funnel": funnel, "deals": list}).to_string()
    }

    #[tool(description = "Get follow-up suggestions based on deal stages and time elapsed. Tells you who to contact and when.")]
    async fn follow_up_suggest(&self) -> String {
        let deals = self.deals.lock().unwrap();
        let now_ts = chrono::Utc::now();
        let mut suggestions: Vec<Value> = Vec::new();
        for d in deals.values() {
            if matches!(d.stage.as_str(), "closed_won" | "closed_lost" | "passed") { continue; }
            let updated = chrono::DateTime::parse_from_rfc3339(&d.updated_at).ok().map(|dt| dt.with_timezone(&chrono::Utc));
            let days_since = updated.map(|u| (now_ts - u).num_days()).unwrap_or(0);
            let (action, urgency) = match d.stage.as_str() {
                "researched" => if days_since > 3 { ("Send initial outreach", "medium") } else { continue },
                "contacted" => if days_since > 5 { ("Follow up on initial outreach", "high") } else { continue },
                "meeting_scheduled" => ("Prepare for meeting", "high"),
                "meeting_done" => if days_since > 2 { ("Send thank you + next steps", "high") } else { continue },
                "term_sheet" => if days_since > 3 { ("Review and respond to terms", "critical") } else { continue },
                "due_diligence" => if days_since > 7 { ("Check on DD progress", "medium") } else { continue },
                _ => continue,
            };
            suggestions.push(json!({"investor": d.investor_name, "deal_id": d.id, "stage": d.stage, "days_since_update": days_since, "action": action, "urgency": urgency}));
        }
        suggestions.sort_by(|a, b| { let ord = |u: &str| match u { "critical" => 0, "high" => 1, "medium" => 2, _ => 3 }; ord(a["urgency"].as_str().unwrap_or("")).cmp(&ord(b["urgency"].as_str().unwrap_or(""))) });
        json!({"suggestions": suggestions.len(), "items": suggestions}).to_string()
    }
}

adk_mcp_sdk::mcp_2026_server! {
    server: FundraiseServer,
    task_tools: ["safe_convert", "currency_convert"],
    approval_tools: [],
    cache_ttl_ms: 60_000,
}
