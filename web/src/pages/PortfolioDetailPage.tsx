import { useState } from "react";
import { useParams, Link } from "react-router-dom";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  usePortfolio,
  useHoldings,
  useCash,
  useSnapshots,
  useTotalReturn,
  usePortfolioValue,
} from "../hooks/usePortfolios";
import * as tinvestApi from "../api/tinvest";
import type { AccountInfo } from "../api/tinvest";
import * as strategiesApi from "../api/strategies";
import * as bondsApi from "../api/bonds";
import type { BondInfo } from "../api/bonds";

export default function PortfolioDetailPage() {
  const { id } = useParams<{ id: string }>();
  const portfolioId = Number(id);
  const queryClient = useQueryClient();

  const { data: portfolio, isLoading } = usePortfolio(portfolioId);
  const { data: holdings } = useHoldings(portfolioId);
  const { data: cashData } = useCash(portfolioId);
  const { data: snapshots } = useSnapshots(portfolioId);
  const { data: returnData } = useTotalReturn(portfolioId);
  const { data: valuation, isLoading: valuationLoading } = usePortfolioValue(portfolioId);
  const { data: tinvestStatus } = useQuery({
    queryKey: ["tinvest-status", portfolioId],
    queryFn: () => tinvestApi.getStatus(portfolioId),
  });
  const { data: strategies } = useQuery({
    queryKey: ["strategies"],
    queryFn: strategiesApi.listStrategies,
  });

  const [importing, setImporting] = useState(false);
  const [runningStrategy, setRunningStrategy] = useState(false);
  const [strategyMessage, setStrategyMessage] = useState("");
  const [selectedBond, setSelectedBond] = useState<BondInfo | null>(null);
  const [loadingBond, setLoadingBond] = useState(false);

  // T-Invest connection state
  const [token, setToken] = useState("");
  const [endpoint, setEndpoint] = useState("sandbox");
  const [sandboxAmount, setSandboxAmount] = useState("1000000");
  const [fetchingAccounts, setFetchingAccounts] = useState(false);
  const [connectError, setConnectError] = useState("");
  const [accounts, setAccounts] = useState<AccountInfo[] | null>(null);
  const [connecting, setConnecting] = useState(false);

  const handleFetchAccounts = async () => {
    if (!token) {
      setConnectError("Token is required");
      return;
    }
    setFetchingAccounts(true);
    setConnectError("");
    setAccounts(null);
    try {
      const result = await tinvestApi.fetchAccounts(
        token,
        endpoint,
        endpoint === "sandbox" ? Number(sandboxAmount) || 1000000 : undefined
      );
      if (result.length === 0) {
        setConnectError("No accounts found for this token");
      } else {
        setAccounts(result);
      }
    } catch (e: any) {
      setConnectError(e.response?.data?.error || "Failed to fetch accounts. Check token and endpoint.");
    } finally {
      setFetchingAccounts(false);
    }
  };

  const handleSelectAccount = async (accountId: string) => {
    setConnecting(true);
    setConnectError("");
    try {
      await tinvestApi.connect(portfolioId, token, accountId, endpoint);
      queryClient.invalidateQueries({ queryKey: ["tinvest-status", portfolioId] });
      setToken("");
      setAccounts(null);
    } catch (e: any) {
      setConnectError(e.response?.data?.error || "Connection failed");
    } finally {
      setConnecting(false);
    }
  };

  const handleDisconnect = async () => {
    await tinvestApi.disconnect(portfolioId);
    queryClient.invalidateQueries({ queryKey: ["tinvest-status", portfolioId] });
  };

  const handleBondClick = async (isin: string) => {
    setLoadingBond(true);
    setSelectedBond(null);
    try {
      const info = await bondsApi.getBondInfo(isin, portfolioId);
      setSelectedBond(info);
    } catch {
      alert("Не удалось загрузить информацию об облигации");
    } finally {
      setLoadingBond(false);
    }
  };

  if (isLoading) return <div className="loading">Loading...</div>;

  const lastSnapshot = snapshots?.length ? snapshots[snapshots.length - 1] : null;
  const totalReturn = returnData?.total_return ? Number(returnData.total_return) : null;

  const handleImport = async () => {
    setImporting(true);
    try {
      const result = await tinvestApi.importPortfolio(portfolioId);
      queryClient.invalidateQueries({ queryKey: ["holdings", portfolioId] });
      queryClient.invalidateQueries({ queryKey: ["cash", portfolioId] });
      queryClient.invalidateQueries({ queryKey: ["portfolioValue", portfolioId] });
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
      queryClient.invalidateQueries({ queryKey: ["portfolioValue", portfolioId] });
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
        {valuation ? (
          <div className="portfolio-value">
            {Number(valuation.total_value).toLocaleString("ru-RU", {
              style: "currency",
              currency: "RUB",
            })}
          </div>
        ) : lastSnapshot ? (
          <div className="portfolio-value">
            {Number(lastSnapshot.market_value).toLocaleString("ru-RU", {
              style: "currency",
              currency: "RUB",
            })}
          </div>
        ) : null}
        {totalReturn !== null && (
          <div className={`portfolio-return ${totalReturn >= 0 ? "positive" : "negative"}`}>
            {totalReturn >= 0 ? "+" : ""}
            {(totalReturn * 100).toFixed(2)}%
          </div>
        )}
      </div>

      {/* T-Invest Import */}
      <section className="detail-section" style={{ marginBottom: 16 }}>
        <h3>T-Bank Connection</h3>
        {tinvestStatus?.connected ? (
          <div>
            <p style={{ marginBottom: 8 }}>
              Connected: <strong>{tinvestStatus.account_id}</strong> ({tinvestStatus.endpoint})
            </p>
            <div style={{ display: "flex", gap: 8 }}>
              <button
                className="btn-primary"
                onClick={handleImport}
                disabled={importing}
              >
                {importing ? "Importing..." : "Import from T-Bank"}
              </button>
              <button
                onClick={handleDisconnect}
                style={{
                  color: "#f44336",
                  border: "1px solid #f44336",
                  background: "transparent",
                  padding: "8px 16px",
                  borderRadius: 8,
                  cursor: "pointer",
                }}
              >
                Disconnect
              </button>
            </div>
          </div>
        ) : (
          <div>
            {connectError && (
              <div className="error-msg" style={{ marginBottom: 12 }}>
                {connectError}
              </div>
            )}

            {!accounts ? (
              <div style={{ display: "flex", flexDirection: "column", gap: 8, maxWidth: 400 }}>
                <p style={{ color: "#888", marginBottom: 4 }}>
                  Enter your T-Invest API token for this portfolio
                </p>
                <input
                  type="password"
                  placeholder="T-Invest Token (t.xxx...)"
                  value={token}
                  onChange={(e) => setToken(e.target.value)}
                  style={{
                    padding: "10px 12px",
                    border: "1px solid #ddd",
                    borderRadius: 8,
                    fontSize: 14,
                  }}
                />
                <select
                  value={endpoint}
                  onChange={(e) => setEndpoint(e.target.value)}
                  style={{
                    padding: "10px 12px",
                    border: "1px solid #ddd",
                    borderRadius: 8,
                    fontSize: 14,
                  }}
                >
                  <option value="sandbox">Sandbox</option>
                  <option value="production">Production</option>
                </select>
                {endpoint === "sandbox" && (
                  <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                    <input
                      type="number"
                      placeholder="Сумма (₽)"
                      value={sandboxAmount}
                      onChange={(e) => setSandboxAmount(e.target.value)}
                      min={1000}
                      step={1000}
                      style={{
                        padding: "10px 12px",
                        border: "1px solid #ddd",
                        borderRadius: 8,
                        fontSize: 14,
                        flex: 1,
                      }}
                    />
                    <span style={{ color: "#888", fontSize: 13, whiteSpace: "nowrap" }}>₽ на виртуальный счёт</span>
                  </div>
                )}
                <button
                  className="btn-primary"
                  onClick={handleFetchAccounts}
                  disabled={fetchingAccounts}
                  style={{ alignSelf: "flex-start" }}
                >
                  {fetchingAccounts ? "Fetching accounts..." : "Get Accounts"}
                </button>
              </div>
            ) : (
              <div>
                <p style={{ color: "#888", marginBottom: 8 }}>
                  Select an account:
                </p>
                <div style={{ display: "flex", flexDirection: "column", gap: 8, maxWidth: 400 }}>
                  {accounts.map((acc) => (
                    <button
                      key={acc.id}
                      className="strategy-card"
                      onClick={() => handleSelectAccount(acc.id)}
                      disabled={connecting}
                      style={{ width: "100%" }}
                    >
                      <strong>{acc.name}</strong>
                      <span className="meta">
                        {acc.account_type} &middot; ID: {acc.id}
                      </span>
                    </button>
                  ))}
                </div>
                <button
                  onClick={() => {
                    setAccounts(null);
                    setConnectError("");
                  }}
                  style={{
                    marginTop: 12,
                    background: "transparent",
                    border: "1px solid #ddd",
                    padding: "8px 16px",
                    borderRadius: 8,
                    cursor: "pointer",
                  }}
                >
                  Back
                </button>
              </div>
            )}
          </div>
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
          <h3>Счёт</h3>
          <div className="cash-amount">
            {cashData
              ? Number(cashData.amount).toLocaleString("ru-RU", {
                  style: "currency",
                  currency: "RUB",
                })
              : "0 RUB"}
          </div>
        </section>

        <section className="detail-section full-width">
          <h3>Облигации ({holdings?.length || 0})</h3>
          {valuationLoading && <p className="meta">Загрузка цен...</p>}
          {holdings?.length === 0 && <p className="empty-state">Нет позиций</p>}
          {holdings && holdings.length > 0 && (
            <div style={{ overflowX: "auto" }}>
              <table className="holdings-table" style={{ width: "100%", tableLayout: "fixed" }}>
                <colgroup>
                  <col style={{ width: "25%" }} />
                  <col style={{ width: "30%" }} />
                  <col style={{ width: "10%" }} />
                  <col style={{ width: "17%" }} />
                  <col style={{ width: "18%" }} />
                </colgroup>
                <thead>
                  <tr>
                    <th>ISIN</th>
                    <th>Название</th>
                    <th style={{ textAlign: "right" }}>Кол-во</th>
                    <th style={{ textAlign: "right" }}>Цена</th>
                    <th style={{ textAlign: "right" }}>Стоимость</th>
                  </tr>
                </thead>
                <tbody>
                  {(valuation?.holdings ?? holdings.map((h) => ({ isin: h.isin, name: h.isin, quantity: h.quantity, price: "0", value: "0", estimated: true }))).map((hv) => (
                    <tr
                      key={hv.isin}
                      onClick={() => handleBondClick(hv.isin)}
                      style={{ cursor: "pointer" }}
                      className="holdings-row-clickable"
                    >
                      <td style={{ fontSize: "0.8em", wordBreak: "break-all" }}>{hv.isin}</td>
                      <td>{hv.name}</td>
                      <td style={{ textAlign: "right" }}>{hv.quantity}</td>
                      <td style={{ textAlign: "right" }}>
                        {Number(hv.price).toLocaleString("ru-RU", { minimumFractionDigits: 2, maximumFractionDigits: 2 })} ₽
                        {hv.estimated && <span title="Оценка по номиналу" style={{ color: "#f59e0b", marginLeft: 4 }}>~</span>}
                      </td>
                      <td style={{ textAlign: "right" }}>
                        {Number(hv.value).toLocaleString("ru-RU", { minimumFractionDigits: 2, maximumFractionDigits: 2 })} ₽
                        {hv.estimated && <span title="Оценка по номиналу" style={{ color: "#f59e0b", marginLeft: 4 }}>~</span>}
                      </td>
                    </tr>
                  ))}
                </tbody>
                {valuation && (
                  <tfoot>
                    <tr style={{ fontWeight: "bold", borderTop: "2px solid var(--border-color, #e2e8f0)" }}>
                      <td colSpan={3}></td>
                      <td style={{ textAlign: "right" }}>Облигации:</td>
                      <td style={{ textAlign: "right" }}>
                        {Number(valuation.bonds_value).toLocaleString("ru-RU", { minimumFractionDigits: 2, maximumFractionDigits: 2 })} ₽
                      </td>
                    </tr>
                    <tr style={{ fontWeight: "bold" }}>
                      <td colSpan={3}></td>
                      <td style={{ textAlign: "right" }}>Денежные средства:</td>
                      <td style={{ textAlign: "right" }}>
                        {Number(valuation.cash).toLocaleString("ru-RU", { minimumFractionDigits: 2, maximumFractionDigits: 2 })} ₽
                      </td>
                    </tr>
                    <tr style={{ fontWeight: "bold", fontSize: "1.1em" }}>
                      <td colSpan={3}></td>
                      <td style={{ textAlign: "right" }}>Итого:</td>
                      <td style={{ textAlign: "right" }}>
                        {Number(valuation.total_value).toLocaleString("ru-RU", { minimumFractionDigits: 2, maximumFractionDigits: 2 })} ₽
                      </td>
                    </tr>
                  </tfoot>
                )}
              </table>
            </div>
          )}
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

      {/* Bond Info Modal */}
      {(selectedBond || loadingBond) && (
        <div className="modal-overlay" onClick={() => { setSelectedBond(null); setLoadingBond(false); }}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            {loadingBond ? (
              <p>Загрузка...</p>
            ) : selectedBond && (
              <>
                <h3 style={{ marginBottom: 16 }}>{selectedBond.name}</h3>
                <div className="bond-info-grid">
                  <div className="bond-info-row">
                    <span className="bond-info-label">Тикер</span>
                    <span>{selectedBond.ticker}</span>
                  </div>
                  <div className="bond-info-row">
                    <span className="bond-info-label">ISIN</span>
                    <span>{selectedBond.isin}</span>
                  </div>
                  {selectedBond.currency && (
                    <div className="bond-info-row">
                      <span className="bond-info-label">Валюта</span>
                      <span>{selectedBond.currency}</span>
                    </div>
                  )}
                  {selectedBond.nominal && (
                    <div className="bond-info-row">
                      <span className="bond-info-label">Номинал</span>
                      <span>{Number(selectedBond.nominal).toLocaleString("ru-RU")} {selectedBond.currency}</span>
                    </div>
                  )}
                  {selectedBond.aci_value && (
                    <div className="bond-info-row">
                      <span className="bond-info-label">НКД</span>
                      <span>{Number(selectedBond.aci_value).toLocaleString("ru-RU")} {selectedBond.currency}</span>
                    </div>
                  )}
                  {selectedBond.coupon_amount && (
                    <div className="bond-info-row">
                      <span className="bond-info-label">Размер купона</span>
                      <span>{selectedBond.coupon_amount}</span>
                    </div>
                  )}
                  {selectedBond.coupon_type && (
                    <div className="bond-info-row">
                      <span className="bond-info-label">Тип купона</span>
                      <span>{selectedBond.coupon_type}</span>
                    </div>
                  )}
                  {selectedBond.coupon_quantity_per_year && (
                    <div className="bond-info-row">
                      <span className="bond-info-label">Купонов в год</span>
                      <span>{selectedBond.coupon_quantity_per_year}</span>
                    </div>
                  )}
                  {selectedBond.next_coupon_date && (
                    <div className="bond-info-row">
                      <span className="bond-info-label">Следующий купон</span>
                      <span>{selectedBond.next_coupon_date}</span>
                    </div>
                  )}
                  {selectedBond.maturity_date && (
                    <div className="bond-info-row">
                      <span className="bond-info-label">Дата погашения</span>
                      <span>{selectedBond.maturity_date}</span>
                    </div>
                  )}
                  {selectedBond.country_of_risk_name && (
                    <div className="bond-info-row">
                      <span className="bond-info-label">Страна</span>
                      <span>{selectedBond.country_of_risk_name}</span>
                    </div>
                  )}
                  {selectedBond.sector && (
                    <div className="bond-info-row">
                      <span className="bond-info-label">Сектор</span>
                      <span>{selectedBond.sector}</span>
                    </div>
                  )}
                  {selectedBond.exchange && (
                    <div className="bond-info-row">
                      <span className="bond-info-label">Биржа</span>
                      <span>{selectedBond.exchange}</span>
                    </div>
                  )}
                  {(selectedBond.floating_coupon || selectedBond.amortization || selectedBond.perpetual) && (
                    <div className="bond-info-row">
                      <span className="bond-info-label">Особенности</span>
                      <span>
                        {[
                          selectedBond.floating_coupon && "Плавающий купон",
                          selectedBond.amortization && "Амортизация",
                          selectedBond.perpetual && "Бессрочная",
                        ].filter(Boolean).join(", ")}
                      </span>
                    </div>
                  )}
                  <div className="bond-info-row">
                    <span className="bond-info-label">Покупка / Продажа</span>
                    <span>
                      {selectedBond.buy_available ? "✅" : "❌"} / {selectedBond.sell_available ? "✅" : "❌"}
                    </span>
                  </div>
                </div>
                <div className="modal-actions">
                  <button className="btn-primary" onClick={() => setSelectedBond(null)}>
                    Закрыть
                  </button>
                </div>
              </>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
