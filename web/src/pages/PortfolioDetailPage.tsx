import { useState } from "react";
import { useParams, Link } from "react-router-dom";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  usePortfolio,
  useHoldings,
  useCash,
  useSnapshots,
  useTotalReturn,
} from "../hooks/usePortfolios";
import * as tinvestApi from "../api/tinvest";
import * as strategiesApi from "../api/strategies";

export default function PortfolioDetailPage() {
  const { id } = useParams<{ id: string }>();
  const portfolioId = Number(id);
  const queryClient = useQueryClient();

  const { data: portfolio, isLoading } = usePortfolio(portfolioId);
  const { data: holdings } = useHoldings(portfolioId);
  const { data: cashData } = useCash(portfolioId);
  const { data: snapshots } = useSnapshots(portfolioId);
  const { data: returnData } = useTotalReturn(portfolioId);
  const { data: tinvestStatus } = useQuery({
    queryKey: ["tinvest-status"],
    queryFn: tinvestApi.getStatus,
  });
  const { data: strategies } = useQuery({
    queryKey: ["strategies"],
    queryFn: strategiesApi.listStrategies,
  });

  const [importing, setImporting] = useState(false);
  const [runningStrategy, setRunningStrategy] = useState(false);
  const [strategyMessage, setStrategyMessage] = useState("");

  if (isLoading) return <div className="loading">Loading...</div>;

  const lastSnapshot = snapshots?.length ? snapshots[snapshots.length - 1] : null;
  const totalReturn = returnData?.total_return ? Number(returnData.total_return) : null;

  const handleImport = async () => {
    setImporting(true);
    try {
      const result = await tinvestApi.importPortfolio(portfolioId);
      queryClient.invalidateQueries({ queryKey: ["holdings", portfolioId] });
      queryClient.invalidateQueries({ queryKey: ["cash", portfolioId] });
      alert(`Imported ${result.holdings_imported} holdings, cash: ${result.cash_rub} RUB`);
    } catch (e: any) {
      alert(e.response?.data?.error || "Import failed");
    } finally {
      setImporting(false);
    }
  };

  const handleSetStrategy = async (strategyId: string) => {
    try {
      await strategiesApi.setStrategy(portfolioId, strategyId);
      queryClient.invalidateQueries({ queryKey: ["portfolio", portfolioId] });
    } catch (e: any) {
      alert(e.response?.data?.error || "Failed to set strategy");
    }
  };

  const handleClearStrategy = async () => {
    try {
      await strategiesApi.clearStrategy(portfolioId);
      queryClient.invalidateQueries({ queryKey: ["portfolio", portfolioId] });
      setStrategyMessage("");
    } catch (e: any) {
      alert(e.response?.data?.error || "Failed to clear strategy");
    }
  };

  const handleRunStrategy = async () => {
    setRunningStrategy(true);
    setStrategyMessage("");
    try {
      const result = await strategiesApi.runStrategy(portfolioId);
      setStrategyMessage(result.message);
      queryClient.invalidateQueries({ queryKey: ["holdings", portfolioId] });
      queryClient.invalidateQueries({ queryKey: ["cash", portfolioId] });
    } catch (e: any) {
      setStrategyMessage(e.response?.data?.error || "Strategy execution failed");
    } finally {
      setRunningStrategy(false);
    }
  };

  const currentStrategy = strategies?.find((s) => s.id === portfolio?.strategy_name);

  return (
    <div className="page">
      <Link to="/" className="back-link">&larr; Back to portfolios</Link>

      <div className="portfolio-header">
        <h2>{portfolio?.name}</h2>
        {lastSnapshot && (
          <div className="portfolio-value">
            {Number(lastSnapshot.market_value).toLocaleString("ru-RU", {
              style: "currency",
              currency: "RUB",
            })}
          </div>
        )}
        {totalReturn !== null && (
          <div className={`portfolio-return ${totalReturn >= 0 ? "positive" : "negative"}`}>
            {totalReturn >= 0 ? "+" : ""}
            {(totalReturn * 100).toFixed(2)}%
          </div>
        )}
      </div>

      {/* T-Invest Import */}
      <section className="detail-section" style={{ marginBottom: 16 }}>
        <h3>T-Bank Import</h3>
        {tinvestStatus?.connected ? (
          <div>
            <p style={{ marginBottom: 8 }}>
              Connected: <strong>{tinvestStatus.account_id}</strong> ({tinvestStatus.endpoint})
            </p>
            <button
              className="btn-primary"
              onClick={handleImport}
              disabled={importing}
            >
              {importing ? "Importing..." : "Import from T-Bank"}
            </button>
          </div>
        ) : (
          <p>
            T-Bank not connected. <Link to="/settings">Connect in Settings</Link>
          </p>
        )}
      </section>

      {/* Strategy */}
      <section className="detail-section" style={{ marginBottom: 16 }}>
        <h3>Strategy</h3>
        {currentStrategy ? (
          <div>
            <div className="strategy-active">
              <strong>{currentStrategy.name}</strong>
              <p className="meta">{currentStrategy.description}</p>
            </div>
            <div style={{ display: "flex", gap: 8, marginTop: 12 }}>
              <button
                className="btn-primary"
                onClick={handleRunStrategy}
                disabled={runningStrategy || !tinvestStatus?.connected}
              >
                {runningStrategy ? "Running..." : "Run Strategy"}
              </button>
              <button onClick={handleClearStrategy}>Remove Strategy</button>
            </div>
            {!tinvestStatus?.connected && (
              <p className="meta" style={{ marginTop: 8 }}>
                Connect T-Bank in Settings to run the strategy
              </p>
            )}
            {strategyMessage && (
              <p style={{ marginTop: 8 }}>{strategyMessage}</p>
            )}
          </div>
        ) : (
          <div className="strategy-picker">
            <p style={{ marginBottom: 8 }}>Select a strategy:</p>
            <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
              {strategies?.map((s) => (
                <button
                  key={s.id}
                  className="strategy-card"
                  onClick={() => handleSetStrategy(s.id)}
                >
                  <strong>{s.name}</strong>
                  <span className="meta">{s.description}</span>
                </button>
              ))}
            </div>
          </div>
        )}
      </section>

      <div className="detail-grid">
        <section className="detail-section">
          <h3>Cash</h3>
          <div className="cash-amount">
            {cashData
              ? Number(cashData.amount).toLocaleString("ru-RU", {
                  style: "currency",
                  currency: "RUB",
                })
              : "0 RUB"}
          </div>
        </section>

        <section className="detail-section">
          <h3>Holdings ({holdings?.length || 0})</h3>
          {holdings?.length === 0 && <p className="empty-state">No holdings</p>}
          <table className="holdings-table">
            <thead>
              <tr>
                <th>ISIN</th>
                <th>Quantity</th>
                <th>Updated</th>
              </tr>
            </thead>
            <tbody>
              {holdings?.map((h) => (
                <tr key={h.id}>
                  <td className="isin">{h.isin}</td>
                  <td>{h.quantity}</td>
                  <td className="meta">{new Date(h.updated_at).toLocaleDateString()}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>

        {snapshots && snapshots.length > 0 && (
          <section className="detail-section full-width">
            <h3>Value History</h3>
            <div className="snapshot-chart">
              <div className="chart-bar-container">
                {snapshots.map((s, i) => {
                  const values = snapshots.map((x) => Number(x.market_value));
                  const max = Math.max(...values);
                  const min = Math.min(...values);
                  const range = max - min || 1;
                  const height = ((Number(s.market_value) - min) / range) * 100;
                  return (
                    <div
                      key={i}
                      className="chart-bar"
                      style={{ height: `${Math.max(height, 2)}%` }}
                      title={`${s.date}: ${Number(s.market_value).toLocaleString("ru-RU")} RUB`}
                    />
                  );
                })}
              </div>
              <div className="chart-labels">
                <span>{snapshots[0].date}</span>
                <span>{snapshots[snapshots.length - 1].date}</span>
              </div>
            </div>
          </section>
        )}
      </div>
    </div>
  );
}
