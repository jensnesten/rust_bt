// module for computing performance statistics

use crate::engine::{OhlcData, Trade};
use std::fmt;
use chrono::NaiveDateTime;

/// compute geometric mean from a slice; if any value is <= 0, return 0.0
pub fn geometric_mean(returns: &[f64]) -> f64 {
    if returns.iter().any(|&r| r <= 0.0) {
        return 0.0;
    }
    let sum_logs: f64 = returns.iter().map(|&r| r.ln()).sum();
    let n = returns.len() as f64;
    (sum_logs / n).exp() - 1.0
}

#[derive(Debug)]
pub struct Stats {
    // tick index of start and end of simulation
    pub start: usize,
    pub end: usize,
    pub duration: usize,
    pub exposure_time_pct: f64,
    pub equity_final: f64,
    pub return_pct: f64,
    pub buy_hold_return_pct: f64,
    pub return_ann_pct: f64,
    pub volatility_ann_pct: f64,
    pub sharpe_ratio: f64,
    pub calmar_ratio: f64,
    pub max_drawdown_pct: f64,
    // number of trades executed
    pub num_trades: usize,
    pub win_rate_pct: f64,
    // best trade in currency
    pub best_trade: f64,
    pub worst_trade: f64,
    pub start_date: String,
    pub end_date: String,
    pub profit_factor: f64,
    pub avg_win: f64,
    pub avg_loss: f64,
    pub alpha: f64,
    pub beta: f64,
    // new field for maximum margin usage (percentage)
    pub max_margin_usage: f64,
}

fn max_drawdown(equity: &[f64]) -> f64 {
    let mut peak = equity[0];
    let mut max_dd = 0.0;
    for &val in equity.iter() {
        if val > peak {
            peak = val;
        } else {
            let dd = (val - peak) / peak;
            if dd < max_dd {
                max_dd = dd;
            }
        }
    }
    max_dd
}

fn compute_beta(equity: &[f64], market_prices: &[f64]) -> f64 {
    let mut equity_returns = Vec::with_capacity(equity.len() - 1);
    let mut market_returns = Vec::with_capacity(market_prices.len() - 1);
    
    for i in 1..equity.len() {
        let equity_return = (equity[i] - equity[i - 1]) / equity[i - 1];
        let market_return = (market_prices[i] - market_prices[i - 1]) / market_prices[i - 1];
        equity_returns.push(equity_return);
        market_returns.push(market_return);
    }

    // compute covariance matrix elements
    let n = equity_returns.len() as f64;
    let equity_mean = equity_returns.iter().sum::<f64>() / n;
    let market_mean = market_returns.iter().sum::<f64>() / n;
    
    let mut cov_em = 0.0; // covariance between equity and market
    let mut var_m = 0.0;  // variance of market
    
    for i in 0..equity_returns.len() {
        cov_em += (equity_returns[i] - equity_mean) * (market_returns[i] - market_mean);
        var_m += (market_returns[i] - market_mean).powi(2);
    }
    
    cov_em /= n - 1.0;
    var_m /= n - 1.0;
    
    // beta = cov(equity, market) / var(market)
    if var_m != 0.0 {
        (cov_em / var_m * 100.0).round() / 100.0 
    } else {
        0.0
    }
}

/// compute performance statistics given the closed trades, equity curve and ohlc data.
/// risk_free_rate is provided as a fraction (for example, 0.0).
pub fn compute_stats(
    trades: &[Trade],
    equity: &[f64],
    ohlc: &OhlcData,
    risk_free_rate: f64,
    max_margin_usage: f64
) -> Stats {
    let start = 0;
    let start_date = ohlc.date[start].clone();
    let end = equity.len() - 1;
    let end_date = ohlc.date[end].clone();
    let duration = end - start;

    let equity_final = equity[end];
    let return_pct = (equity_final - equity[0]) / equity[0] * 100.0;
    let buy_hold_return_pct =
        (ohlc.close[ohlc.close.len() - 1] - ohlc.close[0]) / ohlc.close[0] * 100.0;

    // store original string dates
    let start_date_str = start_date.clone();
    let end_date_str = end_date.clone();
    
    // calculate number of years more accurately using actual dates
    let start_date_parsed = NaiveDateTime::parse_from_str(&start_date, "%Y-%m-%d %H:%M:%S").unwrap();
    let end_date_parsed = NaiveDateTime::parse_from_str(&end_date, "%Y-%m-%d %H:%M:%S").unwrap();
    let days = (end_date_parsed - start_date_parsed).num_days() as f64;
    let years = days / 365.0;  // use calendar days for year fraction
    
    // calculate annualized return
    let return_ann_pct = ((1.0 + return_pct / 100.0).powf(1.0 / years) - 1.0) * 100.0;
    
    // --- Compute period returns for volatility ---
    // (Note: each return corresponds to the time between two consecutive equity observations)
    let period_returns: Vec<f64> = equity
        .windows(2)
        .map(|w| (w[1] - w[0]) / w[0])
        .collect();

    // calculate mean of period returns
    let mean_return = if !period_returns.is_empty() {
        period_returns.iter().sum::<f64>() / period_returns.len() as f64
    } else {
        0.0
    };

    // calculate sample standard deviation of period returns
    let std_return = if period_returns.len() > 1 {
        let variance = period_returns
            .iter()
            .map(|r| (r - mean_return).powi(2))
            .sum::<f64>()
            / (period_returns.len() as f64 - 1.0); // using sample variance (n-1)
        variance.sqrt()
    } else {
        0.0
    };

    // Instead of assuming 252 trading days, compute the actual number of periods per year.
    // We use the OHLC dates to calculate the average time delta between observations.
    let mut total_seconds = 0.0;
    for window in ohlc.date.windows(2) {
        let d0 = NaiveDateTime::parse_from_str(&window[0], "%Y-%m-%d %H:%M:%S").unwrap();
        let d1 = NaiveDateTime::parse_from_str(&window[1], "%Y-%m-%d %H:%M:%S").unwrap();
        total_seconds += (d1 - d0).num_seconds() as f64;
    }
    let avg_dt = total_seconds / (ohlc.date.len() as f64 - 1.0);
    let seconds_per_year = 365.0 * 24.0 * 3600.0; // number of seconds in a calendar year
    let periods_per_year = seconds_per_year / avg_dt;

    let volatility_ann_pct: f64 = std_return * periods_per_year.sqrt() * 100.0;
    
    let max_dd = max_drawdown(equity) * 100.0;
    let num_trades = trades.len();
    let num_wins = trades.iter().filter(|t| t.pnl() > 0.0).count();
    let win_rate_pct = if num_trades > 0 {
        num_wins as f64 / num_trades as f64 * 100.0
    } else {
        0.0
    };

    // compute exposure: percentage of ticks where a trade was open
    let total_ticks = equity.len();
    let mut tick_occupied = vec![false; total_ticks];
    for trade in trades.iter() {
        let start_tick = trade.entry_index;
        let end_tick = trade.exit_index.unwrap_or(total_ticks - 1);
        for t in start_tick..=end_tick {
            tick_occupied[t] = true;
        }
    }
    let ticks_with_position = tick_occupied.iter().filter(|&&b| b).count();
    let exposure_time_pct = ticks_with_position as f64 / total_ticks as f64 * 100.0;

    let calmar_ratio = if max_dd.abs() > 0.0 {
        return_ann_pct.abs() / max_dd.abs()
    } else {
        0.0
    };

    // calculate Sharpe ratio using annualized values
    let sharpe_ratio = if volatility_ann_pct != 0.0 {
        (return_ann_pct - risk_free_rate * 100.0) / volatility_ann_pct
    } else {
        0.0
    };

    // compute avg_win and avg_loss
    let avg_win = trades.iter()
        .filter(|t| t.pnl() > 0.0)
        .map(|t| t.pnl())
        .sum::<f64>() / num_wins as f64;
    // Note: In the original code avg_loss was computed dividing by num_wins, which may be a mistake.
    // Here, we divide by the number of losing trades.
    let num_losses = trades.iter().filter(|t| t.pnl() < 0.0).count();
    let avg_loss = if num_losses > 0 {
        trades.iter()
            .filter(|t| t.pnl() < 0.0)
            .map(|t| t.pnl())
            .sum::<f64>() / num_losses as f64
    } else {
        0.0
    };

    // compute profit factor: sum of profits / absolute sum of losses
    let profit_factor = {
        let profits: f64 = trades.iter()
            .filter(|t| t.pnl() > 0.0)
            .map(|t| t.pnl())
            .sum::<f64>();
        
        let losses: f64 = trades.iter()
            .filter(|t| t.pnl() < 0.0)
            .map(|t| t.pnl())
            .sum::<f64>();

        if losses.abs() > 0.0 {
            profits / losses.abs()
        } else {
            f64::NAN  // if no losses, return NaN (equivalent to numpy's np.nan)
        }
    };

    // compute best and worst trades
    let best_trade = trades.iter()
        .map(|t| t.pnl())
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(0.0);

    let worst_trade = trades.iter()
        .map(|t| t.pnl())
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(0.0);

    let alpha = return_pct - buy_hold_return_pct;
    let beta = compute_beta(equity, &ohlc.close);

    Stats {
        start,
        end,
        start_date: start_date_str,  // use string version
        end_date: end_date_str,      // use string version
        duration,
        exposure_time_pct,
        equity_final,
        return_pct,
        buy_hold_return_pct,
        return_ann_pct,
        volatility_ann_pct,
        sharpe_ratio,
        calmar_ratio,
        profit_factor,
        avg_win,
        avg_loss,
        max_drawdown_pct: max_dd,
        num_trades,
        win_rate_pct,
        best_trade,
        worst_trade,
        alpha,
        beta,
        max_margin_usage,
    }
}

impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "\n\nBacktest Statistics:")?;
        writeln!(f, "====================")?;
        
        // format each stat with consistent spacing (35 chars for the label)
        writeln!(f, "{:<35} {:>15}", "Start Date", self.start_date)?;
        writeln!(f, "{:<35} {:>15}", "End Date", self.end_date)?;
        writeln!(f, "{:<35} {:>15.2}", "Exposure Time [%]", self.exposure_time_pct)?;
        writeln!(f, "{:<35} {:>15.2}", "Total Return [%]", self.return_pct)?;
        writeln!(f, "{:<35} {:>15.2}", "Buy & Hold Return [%]", self.buy_hold_return_pct)?;
        writeln!(f, "{:<35} {:>15.2}", "Equity Final [$]", self.equity_final)?;
        writeln!(f, "{:<35} {:>15.2}", "Sharpe Ratio", self.sharpe_ratio)?;
        writeln!(f, "{:<35} {:>15.2}", "Max Drawdown [%]", self.max_drawdown_pct)?;
        writeln!(f, "{:<35} {:>15.2}", "Profit Factor", self.profit_factor)?;
        writeln!(f, "{:<35} {:>15}", "Total Trades", self.num_trades)?;
        writeln!(f, "{:<35} {:>15.2}", "Win Rate [%]", self.win_rate_pct)?;
        writeln!(f, "{:<35} {:>15.2}", "Best Trade [$]", self.best_trade)?;
        writeln!(f, "{:<35} {:>15.2}", "Worst Trade [$]", self.worst_trade)?;
        writeln!(f, "{:<35} {:>15.2}", "Avg. Win [$]", self.avg_win)?;
        writeln!(f, "{:<35} {:>15.2}", "Avg. Loss [$]", self.avg_loss)?;
        writeln!(f, "{:<35} {:>15.2}", "Beta", self.beta)?;
        writeln!(f, "{:<35} {:>15.2}", "Alpha [%]", self.alpha)?;
        writeln!(f, "{:<35} {:>15.2}", "Return Ann [%]", self.return_ann_pct)?;
        writeln!(f, "{:<35} {:>15.2}", "Volatility Ann [%]", self.volatility_ann_pct)?;
        writeln!(f, "{:<35} {:>15.2}", "Max Margin Usage [%]", self.max_margin_usage * 100.0)?;
       
 
        write!(f, "====================")
    }
}
