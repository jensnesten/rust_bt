'use client';

import TradingChart from '@/components/TradingChart';

export default function Home() {
  return (
    <main className="min-h-screen bg-[#131722] p-4">
      <h1 className="text-2xl font-bold text-white mb-4">Live Trading Equity</h1>
      <div className="w-full h-[600px]">
        <TradingChart />
      </div>
    </main>
  );
}