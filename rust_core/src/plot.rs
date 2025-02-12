use plotters::prelude::*;
use chrono::NaiveDateTime;

/// function plot_equity that plots equity values as a function of time
/// it takes a slice of (naivedatetime, equity_value) tuples and an output file path
pub fn plot_equity(data: &[(NaiveDateTime, f64)], output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // determine the minimum and maximum dates for the x-axis
    let start_date = data.first().unwrap().0;
    let end_date = data.last().unwrap().0;
    // convert naivedatetime to timestamp (i64) for plotting
    let start_ts = start_date.and_utc().timestamp();
    let end_ts = end_date.and_utc().timestamp();

    // determine the equity range for the y-axis
    let min_equity = data.iter().map(|&(_, equity)| equity).fold(f64::INFINITY, f64::min);
    let max_equity = data.iter().map(|&(_, equity)| equity).fold(f64::NEG_INFINITY, f64::max);

    // create a drawing area for the plot
    let root_area = BitMapBackend::new(output_path, (800, 600)).into_drawing_area();
    root_area.fill(&WHITE)?;

    // build the chart object with axis labels and margins, using timestamp range for x-axis
    let mut chart = ChartBuilder::on(&root_area)
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(50)
        .build_cartesian_2d(start_ts..end_ts, min_equity..max_equity)?;

    // configure the mesh for the chart and add a custom x-axis label formatter
    chart.configure_mesh()
        .x_label_formatter(&|x| {
            // convert timestamp to datetime
            let dt = NaiveDateTime::from_timestamp(*x, 0);
            dt.format("%Y-%m-%d").to_string()
        })
        .x_labels(5)
        .y_labels(5)
        .draw()?;

    // draw the equity line series, converting the naivedatetime for plotting
    chart.draw_series(LineSeries::new(
        data.iter().map(|&(time, equity)| (time.and_utc().timestamp(), equity)),
        &BLUE,
    ))?
    .label("equity")
    .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &BLUE));

    // configure and draw the legend for clarity
    chart.configure_series_labels()
        .border_style(&BLACK)
        .draw()?;

    // return ok upon successful completion
    Ok(())
}

pub fn plot_equity_and_benchmark(
    equity: &[(NaiveDateTime, f64)],
    benchmark: &[(NaiveDateTime, f64)],
    output_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // compute the starting and ending dates for equity
    let start_date_equity = equity.first().unwrap().0;
    let end_date_equity = equity.last().unwrap().0;
    // compute the starting and ending dates for benchmark
    let start_date_benchmark = benchmark.first().unwrap().0;
    let end_date_benchmark = benchmark.last().unwrap().0;
    // determine the overall start and end dates by taking the union
    let start_date = if start_date_equity < start_date_benchmark {
        start_date_equity
    } else {
        start_date_benchmark
    };
    let end_date = if end_date_equity > end_date_benchmark {
        end_date_equity
    } else {
        end_date_benchmark
    };
    // convert naivedatetime into timestamp (i64)
    let start_ts = start_date.and_utc().timestamp();
    let end_ts = end_date.and_utc().timestamp();

    // compute the y-axis bounds for the equity data
    let equity_min = equity.iter().map(|&(_, value)| value).fold(f64::INFINITY, f64::min);
    let equity_max = equity.iter().map(|&(_, value)| value).fold(f64::NEG_INFINITY, f64::max);
    // compute the y-axis bounds for the benchmark data
    let benchmark_min = benchmark.iter().map(|&(_, value)| value).fold(f64::INFINITY, f64::min);
    let benchmark_max = benchmark.iter().map(|&(_, value)| value).fold(f64::NEG_INFINITY, f64::max);
    // take the union of the y-axis ranges
    let min_value = equity_min.min(benchmark_min);
    let max_value = equity_max.max(benchmark_max);

    // create the drawing area for the plot and clear it with white background
    let root_area = BitMapBackend::new(output_path, (800, 600)).into_drawing_area();
    root_area.fill(&WHITE)?;

    // build the chart with the computed x and y ranges
    let mut chart = ChartBuilder::on(&root_area)
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(50)
        .build_cartesian_2d(start_ts..end_ts, min_value..max_value)?;

    // configure the chart's mesh with custom formatting for the x-axis stamps
    chart
        .configure_mesh()
        .x_label_formatter(&|x| {
            // convert timestamp to datetime
            let dt = NaiveDateTime::from_timestamp(*x, 0);
            dt.format("%Y-%m-%d").to_string()
        })
        .x_labels(5)
        .y_labels(5)
        .draw()?;

    // draw the equity series in blue, converting datetime to timestamp
    chart
        .draw_series(LineSeries::new(
            equity.iter().map(|&(time, value)| (time.and_utc().timestamp(), value)),
            &BLUE,
        ))?
        .label("equity")
        .legend(|(x, y)| {
            // create a legend entry for equity
            PathElement::new(vec![(x, y), (x + 20, y)], &BLUE)
        });

    // draw the benchmark series in red, converting datetime to timestamp
    chart
        .draw_series(LineSeries::new(
            benchmark.iter().map(|&(time, value)| (time.and_utc().timestamp(), value)),
            &RED,
        ))?
        .label("benchmark")
        .legend(|(x, y)| {
            // create a legend entry for benchmark
            PathElement::new(vec![(x, y), (x + 20, y)], &RED)
        });

    // configure and draw the legend on the chart for clarity
    chart.configure_series_labels()
        .border_style(&BLACK)
        .draw()?;

    // return ok if the plot completes successfully
    Ok(())
}

pub fn plot_margin_usage(data: &[(NaiveDateTime, f64)], output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // determine the minimum and maximum dates for the x-axis
    let start_date = data.first().unwrap().0;
    let end_date = data.last().unwrap().0;
    // convert naivedatetime to timestamp (i64) for plotting
    let start_ts = start_date.and_utc().timestamp();
    let end_ts = end_date.and_utc().timestamp();

    // determine the y-axis bounds for the margin usage data
    let min_margin_usage = data.iter().map(|&(_, margin_usage)| margin_usage).fold(f64::INFINITY, f64::min);
    let max_margin_usage = data.iter().map(|&(_, margin_usage)| margin_usage).fold(f64::NEG_INFINITY, f64::max);

    // adjust y-axis range so upper bound is always at least 1.0
    let (y_lower, y_upper) = if (max_margin_usage - min_margin_usage).abs() < std::f64::EPSILON {
        // constant data; add padding
        (min_margin_usage - 1.0, (max_margin_usage + 1.0).max(1.0))
    } else {
        (min_margin_usage, max_margin_usage.max(1.0))
    };
    let y_range = y_lower..y_upper;

    // create a drawing area for the plot
    let root_area = BitMapBackend::new(output_path, (800, 600)).into_drawing_area();
    root_area.fill(&WHITE)?;

    // build the chart object with axis labels and margins, using timestamp range for x-axis
    let mut chart = ChartBuilder::on(&root_area)
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(50)
        .build_cartesian_2d(start_ts..end_ts, y_range)?;

    // configure the mesh for the chart and add a custom x-axis label formatter
    chart.configure_mesh()
        .x_label_formatter(&|x| {
            // convert timestamp to datetime
            let dt = NaiveDateTime::from_timestamp(*x, 0);
            dt.format("%Y-%m-%d").to_string()
        })
        .x_labels(5)
        .y_labels(5)
        .draw()?;

    // draw the margin usage series, converting the naivedatetime for plotting
    chart.draw_series(LineSeries::new(
        data.iter().map(|&(time, margin_usage)| (time.and_utc().timestamp(), margin_usage)),
        &BLUE,
    ))?
    .label("margin usage")
    .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &BLUE));

    // return ok to satisfy the function result type
    Ok(())
}
