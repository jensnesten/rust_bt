'use client';

import { useEffect, useRef } from 'react';
import { createChart, ColorType, IChartApi, ISeriesApi, CandlestickSeriesOptions, Time, ChartOptions } from 'lightweight-charts';

const TradingChart = () => {
  const chartContainerRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<IChartApi | null>(null);

  useEffect(() => {
    if (!chartContainerRef.current) return;

    const chart = createChart(chartContainerRef.current, {
      layout: {
        background: { type: ColorType.Solid, color: '#131722' },
        textColor: '#d1d4dc',
      },
      grid: {
        vertLines: { color: '#242632' },
        horzLines: { color: '#242632' },
      },
      timeScale: {
        timeVisible: true,
        secondsVisible: true,
        borderColor: '#485c7b',
        rightOffset: 20,  // More space on the right
        barSpacing: 3,    // Smaller spacing between points
        fixLeftEdge: true,
        fixRightEdge: false, // Allow right edge to expand
        minBarSpacing: 2, // Prevent bars from getting too compressed
        lockVisibleTimeRangeOnResize: false, // Don't lock time range on resize
      },
      rightPriceScale: {
        borderColor: '#485c7b',
        autoScale: true,
        scaleMargins: {
          top: 0.3,    // More space at top
          bottom: 0.3, // More space at bottom
        },
      },
      width: chartContainerRef.current.clientWidth,  // Use full width
      height: window.innerHeight - 50,  // Almost full height
    } as ChartOptions) as IChartApi & { addCandlestickSeries: (options: CandlestickSeriesOptions) => ISeriesApi<"Candlestick"> };

    const baselineSeries = chart.addBaselineSeries({
      baseValue: { type: 'price', price: 0 },
      topFillColor1: '#2962FF',
      topFillColor2: 'rgba(41, 98, 255, 0.28)',
      topLineColor: '#2962FF',
      bottomFillColor1: 'rgba(239, 83, 80, 0.05)',
      bottomFillColor2: 'rgba(239, 83, 80, 0.28)',
      bottomLineColor: '#ef5350',
      lineWidth: 2,
      priceLineVisible: false,
      baseLineVisible: true,
      lastValueVisible: true,
    });

    // enable interactions
    chart.applyOptions({
      handleScroll: true,
      handleScale: true,
    });
    
    const ws = new WebSocket(`ws://localhost:3000/ws`);
    
    ws.onmessage = (event) => {
      const data = JSON.parse(event.data);
      if (data.length > 0) {
        // transform data to start from 0 (relative changes)
        const baseValue = data[0].close;
        const chartData = data.map((candle: any) => ({
          time: candle.time,
          value: candle.close - baseValue, // relative change from start
        }));
        
        baselineSeries.setData(chartData);
        
        // Don't reset the visible range every time - let it scroll naturally
        // Just ensure the price scale starts at 0
        const maxChange = Math.max(...chartData.map((point: any) => Math.abs(point.value)));
        const buffer = maxChange * 0.5; // 50% buffer
        
        // ensure minimum is 0
        chart.priceScale('right').applyOptions({
          autoScale: false,
          scaleMargins: {
            top: 0.1,    // Less space at top
            bottom: 0.1, // Less space at bottom
          },
        });
        
        // Set price range only once when data first loads
        if (chartData.length === data.length) {
          // Fit to initial data then let it scroll
          chart.timeScale().fitContent();
        }
      }
    };

    const handleResize = () => {
      if (chartContainerRef.current) {
        chart.applyOptions({
          width: chartContainerRef.current.clientWidth,
        });
      }
    };

    window.addEventListener('resize', handleResize);
    chartRef.current = chart;

    return () => {
      window.removeEventListener('resize', handleResize);
      chart.remove();
      ws.close();
    };
  }, []);

  return (
    <div ref={chartContainerRef} style={{
      position: 'absolute',
      left: 0,
      top: 0,
      right: 0,
      bottom: 0,
    }} />
  );
};

export default TradingChart;