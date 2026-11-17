import { useCallback, useEffect, useRef, useState, type ReactNode } from 'react'
import {
  CandlestickSeries,
  ColorType,
  createChart,
  type CandlestickData,
  type ISeriesApi,
  type UTCTimestamp,
} from 'lightweight-charts'

const API_BASE = (import.meta.env.VITE_API_BASE_URL ?? '').replace(/\/$/, '')
const TIMEFRAMES = ['1m', '5m', '15m', '1H', '4H', '1D'] as const

type MarketData = {
  symbol: string
  last_price: number
  mark_price: number | null
  bid_price: number
  ask_price: number
  volume_24h: number
  funding_rate: number | null
  open_interest: number | null
  high_24h: number
  low_24h: number
  timestamp: string
}

type Candle = {
  time: number
  open: number
  high: number
  low: number
  close: number
  volume: number
}

type AiDecision = {
  decision: 'LONG' | 'SHORT' | 'WAIT'
  market_regime: string
  timeframe: string
  entry_type: string
  entry_price: number
  stop_loss: number
  take_profit: number
  risk_reward_ratio: number
  reasoning: string[]
  invalidation: string
  valid_for_minutes: number
}

type AiAnalysis = {
  decision: AiDecision
  symbol: string
  model: string
  generated_at: string
}

type PaperAccount = {
  initial_balance: number
  balance: number
  equity: number
  available_balance: number
  used_margin: number
  unrealized_pnl: number
  realized_pnl: number
  total_pnl: number
  margin_per_order: number
  leverage: number
  max_loss_percent: number
  max_leverage: number
  auto_trade_enabled: boolean
  allow_multiple_positions: boolean
}

type PaperSettings = {
  margin_per_order: number
  leverage: number
  max_loss_percent: number
  auto_trade_enabled: boolean
  allow_multiple_positions: boolean
  analysis_timeframe: string
}

type PaperPosition = {
  id: string
  direction: 'LONG' | 'SHORT'
  entry_price: number
  current_price: number
  stop_loss: number
  take_profit: number
  quantity: number
  leverage: number
  margin: number
  pnl: number
  opened_at: string
}

type ClosedPaperTrade = {
  id: string
  direction: 'LONG' | 'SHORT'
  entry_price: number
  exit_price: number
  quantity: number
  pnl: number
  close_reason: string
  closed_at: string
}

const formatPrice = (value?: number | null) =>
  value == null ? '—' : `$${value.toLocaleString('vi-VN', { maximumFractionDigits: 2 })}`
const formatMoney = (value: number) => `$${value.toLocaleString('vi-VN', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`

function App() {
  const [market, setMarket] = useState<MarketData | null>(null)
  const [marketError, setMarketError] = useState('')
  const [isConnected, setIsConnected] = useState(false)
  const [lastUpdated, setLastUpdated] = useState<string | null>(null)
  const [timeframe, setTimeframe] = useState<(typeof TIMEFRAMES)[number]>('15m')
  const [refreshKey, setRefreshKey] = useState(0)
  const [tickerLoading, setTickerLoading] = useState(true)
  const [analysis, setAnalysis] = useState<AiAnalysis | null>(null)
  const [analysisError, setAnalysisError] = useState('')
  const [analyzing, setAnalyzing] = useState(false)
  const [paperAccount, setPaperAccount] = useState<PaperAccount | null>(null)
  const [paperPositions, setPaperPositions] = useState<PaperPosition[]>([])
  const [paperTrades, setPaperTrades] = useState<ClosedPaperTrade[]>([])
  const [paperSettings, setPaperSettings] = useState<PaperSettings | null>(null)
  const [paperError, setPaperError] = useState('')
  const [paperNotice, setPaperNotice] = useState('')
  const [paperLoading, setPaperLoading] = useState(true)
  const [marginInput, setMarginInput] = useState('100')
  const [leverageInput, setLeverageInput] = useState('10')
  const [maxLossInput, setMaxLossInput] = useState('5')
  const paperConfigInitialized = useRef(false)
  const [stopLossInput, setStopLossInput] = useState('')
  const [takeProfitInput, setTakeProfitInput] = useState('')

  const refreshTicker = useCallback(async () => {
    try {
      const response = await fetch(`${API_BASE}/api/market/ticker`)
      if (!response.ok) {
        throw new Error(`Không lấy được giá thị trường (HTTP ${response.status})`)
      }
      const data = (await response.json()) as MarketData
      setMarket(data)
      setLastUpdated(new Date().toLocaleTimeString('vi-VN'))
      setIsConnected(true)
      setMarketError('')
    } catch (error) {
      setIsConnected(false)
      setMarketError(error instanceof Error ? error.message : 'Không thể kết nối máy chủ dữ liệu')
    } finally {
      setTickerLoading(false)
    }
  }, [])

  const analyzeMarket = async () => {
    setAnalyzing(true)
    setAnalysisError('')
    try {
      const response = await fetch(
        `${API_BASE}/api/ai/analyze?timeframe=${encodeURIComponent(timeframe)}`,
        { method: 'POST' },
      )
      const payload = await response.json()
      if (!response.ok) {
        throw new Error(payload.error ?? `Phân tích thất bại (HTTP ${response.status})`)
      }
      setAnalysis(payload as AiAnalysis)
    } catch (error) {
      setAnalysisError(error instanceof Error ? error.message : 'Không thể chạy phân tích AI')
    } finally {
      setAnalyzing(false)
    }
  }

  const refreshPaperTrading = useCallback(async () => {
    try {
      const [accountResponse, positionsResponse, tradesResponse, settingsResponse] = await Promise.all([
        fetch(`${API_BASE}/api/simulation/accounts`),
        fetch(`${API_BASE}/api/simulation/positions`),
        fetch(`${API_BASE}/api/simulation/trades`),
        fetch(`${API_BASE}/api/simulation/settings`),
      ])
      if (!accountResponse.ok || !positionsResponse.ok || !tradesResponse.ok || !settingsResponse.ok) {
        throw new Error('Không tải được dữ liệu tài khoản mô phỏng.')
      }
      const [accountData, positionsData, tradesData, settingsData] = await Promise.all([
        accountResponse.json(),
        positionsResponse.json(),
        tradesResponse.json(),
        settingsResponse.json(),
      ])
      setPaperAccount(accountData.account as PaperAccount)
      setPaperPositions(positionsData.positions as PaperPosition[])
      setPaperTrades(tradesData.trades as ClosedPaperTrade[])
      setPaperSettings({
        ...settingsData.settings,
        analysis_timeframe: settingsData.analysis_timeframe,
      } as PaperSettings)
      if (!paperConfigInitialized.current) {
        setMarginInput(String(settingsData.settings.margin_per_order))
        setLeverageInput(String(settingsData.settings.leverage))
        setMaxLossInput(String(settingsData.settings.max_loss_percent))
        paperConfigInitialized.current = true
      }
      setPaperError('')
    } catch (error) {
      setPaperError(error instanceof Error ? error.message : 'Lỗi khi tải dữ liệu mô phỏng.')
    } finally {
      setPaperLoading(false)
    }
  }, [])

  const openPaperPosition = async (
    direction: 'LONG' | 'SHORT',
    stopLoss = Number(stopLossInput),
    takeProfit = Number(takeProfitInput),
  ) => {
    setPaperError('')
    setPaperNotice('')
    try {
      const response = await fetch(`${API_BASE}/api/simulation/orders`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          direction,
          stop_loss: stopLoss,
          take_profit: takeProfit,
          margin_amount: Number(marginInput),
          leverage: Number(leverageInput),
          max_loss_percent: Number(maxLossInput),
        }),
      })
      const result = await response.json()
      if (!response.ok) {
        throw new Error(result.error ?? `Không mở được vị thế (HTTP ${response.status})`)
      }
      setPaperNotice(`Đã mở vị thế ${direction === 'LONG' ? 'MUA' : 'BÁN'} mô phỏng tại ${formatPrice(result.position.entry_price)}.`)
      setStopLossInput('')
      setTakeProfitInput('')
      await refreshPaperTrading()
    } catch (error) {
      setPaperError(error instanceof Error ? error.message : 'Không thể mở vị thế mô phỏng.')
    }
  }

  const saveAutoTradingSettings = async (enabled: boolean) => {
    setPaperError('')
    setPaperNotice('')
    try {
      const response = await fetch(`${API_BASE}/api/simulation/settings`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          margin_per_order: Number(marginInput),
          leverage: Number(leverageInput),
          max_loss_percent: Number(maxLossInput),
          auto_trade_enabled: enabled,
        }),
      })
      const result = await response.json()
      if (!response.ok) {
        throw new Error(result.error ?? `Không lưu được cấu hình (HTTP ${response.status})`)
      }
      setPaperSettings({
        ...result.settings,
        analysis_timeframe: result.analysis_timeframe,
      } as PaperSettings)
      setPaperNotice(enabled
        ? `Đã bật AI; phân tích sau mỗi nến ${result.analysis_timeframe} đóng và chỉ vào lệnh khi có tín hiệu.`
        : 'Đã tắt AI tự vào lệnh.')
      await refreshPaperTrading()
    } catch (error) {
      setPaperError(error instanceof Error ? error.message : 'Không thể cập nhật cấu hình giao dịch.')
    }
  }

  const closePaperPosition = async (positionId: string) => {
    setPaperError('')
    setPaperNotice('')
    try {
      const response = await fetch(`${API_BASE}/api/simulation/positions/${encodeURIComponent(positionId)}`, {
        method: 'DELETE',
      })
      const result = await response.json()
      if (!response.ok) {
        throw new Error(result.error ?? `Không đóng được vị thế (HTTP ${response.status})`)
      }
      setPaperNotice(`Đã đóng vị thế. Lãi/lỗ: ${formatMoney(result.trade.pnl)}.`)
      await refreshPaperTrading()
    } catch (error) {
      setPaperError(error instanceof Error ? error.message : 'Không thể đóng vị thế mô phỏng.')
    }
  }

  useEffect(() => {
    void refreshTicker()
    const timer = window.setInterval(() => void refreshTicker(), 5_000)
    return () => window.clearInterval(timer)
  }, [refreshTicker, refreshKey])

  useEffect(() => {
    void refreshPaperTrading()
    const timer = window.setInterval(() => void refreshPaperTrading(), 5_000)
    return () => window.clearInterval(timer)
  }, [refreshPaperTrading])

  const validOrderSettings =
    Number.isFinite(Number(marginInput)) &&
    Number(marginInput) > 0 &&
    Number.isInteger(Number(leverageInput)) &&
    Number(leverageInput) >= 1 &&
    Number(leverageInput) <= (paperAccount?.max_leverage ?? 20) &&
    Number.isFinite(Number(maxLossInput)) &&
    Number(maxLossInput) >= 0.1 &&
    Number(maxLossInput) <= 100
  const validManualLevels =
    validOrderSettings &&
    Number.isFinite(Number(stopLossInput)) &&
    Number(stopLossInput) > 0 &&
    Number.isFinite(Number(takeProfitInput)) &&
    Number(takeProfitInput) > 0

  return (
    <main className="min-h-screen bg-slate-950 text-slate-100">
      <div className="mx-auto max-w-7xl p-4 sm:p-6">
        <header className="mb-6 flex flex-wrap items-center justify-between gap-4 border-b border-slate-800 pb-4">
          <div>
            <p className="text-xs uppercase tracking-[0.3em] text-cyan-400">BTC Futures AI Lab</p>
            <h1 className="mt-2 text-2xl font-bold text-white sm:text-3xl">Bảng điều khiển giao dịch mô phỏng</h1>
          </div>
          <div className="flex items-center gap-3">
            <span
              className={`rounded-full border px-3 py-1 text-sm font-medium ${
                isConnected
                  ? 'border-emerald-500/40 bg-emerald-500/10 text-emerald-300'
                  : 'border-rose-500/40 bg-rose-500/10 text-rose-300'
              }`}
            >
              {isConnected ? 'Đang nhận dữ liệu OKX' : tickerLoading && !market ? 'Đang kết nối…' : 'Mất kết nối OKX'}
            </span>
            <button
              type="button"
              onClick={() => {
                setTickerLoading(true)
                setRefreshKey((key) => key + 1)
              }}
              className="rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-sm hover:border-cyan-500 hover:text-cyan-300"
            >
              Làm mới
            </button>
          </div>
        </header>

        {marketError && (
          <div role="alert" className="mb-5 rounded-xl border border-rose-500/30 bg-rose-500/10 p-3 text-sm text-rose-200">
            {marketError}. Kiểm tra backend và kết nối Internet, hoặc nhấn “Làm mới” để thử lại.
          </div>
        )}

        <section className="mb-6 grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
          <MetricCard label="Giá gần nhất" value={formatPrice(market?.last_price)} tone="green" />
          <MetricCard
            label="Khối lượng 24 giờ"
            value={market ? `${market.volume_24h.toLocaleString('vi-VN')} BTC` : '—'}
            tone="cyan"
          />
          <MetricCard label="Funding rate" value={market?.funding_rate == null ? 'Chưa có dữ liệu' : `${(market.funding_rate * 100).toFixed(4)}%`} tone="amber" />
          <MetricCard
            label="Cao / thấp 24 giờ"
            value={market ? `${formatPrice(market.high_24h)} / ${formatPrice(market.low_24h)}` : '—'}
            tone="violet"
          />
        </section>

        <section className="mb-6 grid gap-6 xl:grid-cols-[1.5fr_0.9fr]">
          <Panel title="Biểu đồ giá">
            <div className="mb-4 flex flex-wrap items-center justify-between gap-3">
              <div>
                <p className="text-xs uppercase tracking-[0.2em] text-slate-400">Hợp đồng</p>
                <h2 className="mt-1 text-xl font-semibold text-white">{market?.symbol ?? 'BTC-USDT-SWAP'}</h2>
                {lastUpdated && <p className="mt-1 text-xs text-slate-500">Cập nhật lúc {lastUpdated} · tự làm mới mỗi 5 giây</p>}
              </div>
              <div className="flex flex-wrap gap-2" aria-label="Chọn khung thời gian">
                {TIMEFRAMES.map((item) => (
                  <button
                    key={item}
                    type="button"
                    aria-pressed={timeframe === item}
                    onClick={() => setTimeframe(item)}
                    className={`rounded border px-3 py-1.5 text-sm ${
                      timeframe === item
                        ? 'border-cyan-500/50 bg-cyan-500/10 text-cyan-300'
                        : 'border-slate-700 bg-slate-800 text-slate-300 hover:border-slate-500'
                    }`}
                  >
                    {item}
                  </button>
                ))}
              </div>
            </div>
            <TradingChart timeframe={timeframe} refreshKey={refreshKey} />
          </Panel>

          <div className="space-y-6">
            <Panel title="Phân tích AI">
              {analysis ? (
                <div className="space-y-4">
                  <div className="flex items-center justify-between rounded-xl border border-slate-800 bg-slate-950/60 p-3">
                    <span className="text-sm text-slate-400">Tín hiệu ({analysis.decision.timeframe})</span>
                    <span className={`font-semibold ${analysis.decision.decision === 'LONG' ? 'text-emerald-300' : analysis.decision.decision === 'SHORT' ? 'text-rose-300' : 'text-amber-300'}`}>
                      {analysis.decision.decision === 'LONG' ? 'MUA' : analysis.decision.decision === 'SHORT' ? 'BÁN' : 'CHỜ'}
                    </span>
                  </div>
                  {analysis.decision.decision !== 'WAIT' && (
                    <div className="grid grid-cols-2 gap-3 text-sm">
                      <InfoRow label="Điểm vào" value={formatPrice(analysis.decision.entry_price)} />
                      <InfoRow label="Cắt lỗ" value={formatPrice(analysis.decision.stop_loss)} />
                      <InfoRow label="Chốt lời" value={formatPrice(analysis.decision.take_profit)} />
                      <InfoRow label="Tỷ lệ lời/rủi ro" value={`${analysis.decision.risk_reward_ratio.toFixed(2)}R`} />
                    </div>
                  )}
                  <div className="rounded-xl border border-slate-800 bg-slate-950/50 p-3">
                    <p className="mb-2 text-xs uppercase tracking-[0.2em] text-slate-400">Nhận định</p>
                    <ul className="space-y-2 text-sm text-slate-300">
                      {analysis.decision.reasoning.map((reason, index) => <li key={`${index}-${reason}`}>• {reason}</li>)}
                    </ul>
                    {analysis.decision.invalidation && (
                      <p className="mt-3 text-xs text-slate-400">Điều kiện vô hiệu: {analysis.decision.invalidation}</p>
                    )}
                  </div>
                  {analysis.decision.decision !== 'WAIT' && (
                    <button
                      type="button"
                      onClick={() => {
                        if (analysis.decision.decision !== 'WAIT') {
                          void openPaperPosition(
                            analysis.decision.decision,
                            analysis.decision.stop_loss,
                            analysis.decision.take_profit,
                          )
                        }
                      }}
                      disabled={!validOrderSettings}
                      className="w-full rounded-xl bg-emerald-400 px-3 py-2 font-semibold text-slate-950 hover:bg-emerald-300 disabled:cursor-not-allowed disabled:opacity-40"
                    >
                      Mở vị thế mô phỏng theo tín hiệu
                    </button>
                  )}
                  <p className="text-xs text-slate-500">
                    Mô hình {analysis.model} · {new Date(analysis.generated_at).toLocaleString('vi-VN')}. Tín hiệu chỉ để tham khảo, chưa đặt lệnh.
                  </p>
                </div>
              ) : (
                <p className="rounded-xl border border-slate-800 bg-slate-950/50 p-4 text-sm leading-6 text-slate-300">
                  Chưa có kết quả. Nhấn nút bên dưới để phân tích nến OKX bằng Gemini.
                </p>
              )}
              {analysisError && (
                <p role="alert" className="mt-3 rounded-lg border border-rose-500/30 bg-rose-500/10 p-3 text-sm text-rose-200">
                  {analysisError}
                </p>
              )}
              <button
                type="button"
                onClick={() => void analyzeMarket()}
                disabled={analyzing}
                className="mt-4 w-full rounded-xl bg-cyan-400 px-3 py-2 font-semibold text-slate-950 hover:bg-cyan-300 disabled:cursor-wait disabled:opacity-60"
              >
                {analyzing ? 'Đang phân tích…' : 'Phân tích thị trường'}
              </button>
            </Panel>

            <Panel title="Tài khoản mô phỏng">
              <div className="grid grid-cols-2 gap-3 text-sm">
                <InfoRow label="Vốn ban đầu" value={paperAccount ? formatMoney(paperAccount.initial_balance) : '—'} />
                <InfoRow label="Giá trị tài khoản" value={paperAccount ? formatMoney(paperAccount.equity) : '—'} />
                <InfoRow label="Khả dụng" value={paperAccount ? formatMoney(paperAccount.available_balance) : '—'} />
                <InfoRow label="Ký quỹ" value={paperAccount ? formatMoney(paperAccount.used_margin) : '—'} />
                <InfoRow label="Lãi/lỗ chưa chốt" value={paperAccount ? formatMoney(paperAccount.unrealized_pnl) : '—'} />
                <InfoRow label="Lãi/lỗ đã chốt" value={paperAccount ? formatMoney(paperAccount.realized_pnl) : '—'} />
              </div>
              <div className="mt-5 space-y-3 border-t border-slate-800 pt-4">
                <p className="text-sm font-medium text-slate-200">Vốn và đòn bẩy mỗi lệnh</p>
                <div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
                  <label className="text-xs text-slate-400">
                    Tiền ký quỹ (USDT)
                    <input
                      type="number"
                      min="1"
                      step="1"
                      value={marginInput}
                      onChange={(event) => setMarginInput(event.target.value)}
                      className="mt-1 w-full rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-white outline-none focus:border-cyan-500"
                    />
                  </label>
                  <label className="text-xs text-slate-400">
                    Đòn bẩy (tối đa {paperAccount?.max_leverage ?? 20}x)
                    <input
                      type="number"
                      min="1"
                      max={paperAccount?.max_leverage ?? 20}
                      step="1"
                      value={leverageInput}
                      onChange={(event) => setLeverageInput(event.target.value)}
                      className="mt-1 w-full rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-white outline-none focus:border-cyan-500"
                    />
                  </label>
                  <label className="text-xs text-slate-400">
                    Lỗ tối đa (% ký quỹ)
                    <input
                      type="number"
                      min="0.1"
                      max="100"
                      step="0.1"
                      value={maxLossInput}
                      onChange={(event) => setMaxLossInput(event.target.value)}
                      className="mt-1 w-full rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-white outline-none focus:border-cyan-500"
                    />
                  </label>
                </div>
                <p className="text-xs text-slate-500">
                  Mỗi lệnh dùng {marginInput || '0'} USDT × {leverageInput || '0'}x; mức lỗ tối đa là {maxLossInput || '0'}% ký quỹ. Bot phân tích khi nến {paperSettings?.analysis_timeframe ?? '15m'} đóng.
                </p>
                <div className="grid grid-cols-2 gap-3">
                  <button
                    type="button"
                    disabled={!validOrderSettings || paperSettings?.auto_trade_enabled}
                    onClick={() => void saveAutoTradingSettings(true)}
                    className="rounded-lg bg-cyan-400 px-3 py-2 text-sm font-semibold text-slate-950 hover:bg-cyan-300 disabled:cursor-not-allowed disabled:opacity-40"
                  >
                    {paperSettings?.auto_trade_enabled ? 'AI tự vào lệnh: ĐANG BẬT' : 'Bật AI tự vào lệnh'}
                  </button>
                  <button
                    type="button"
                    disabled={!paperSettings?.auto_trade_enabled}
                    onClick={() => void saveAutoTradingSettings(false)}
                    className="rounded-lg border border-slate-700 px-3 py-2 text-sm text-slate-200 hover:border-rose-400 hover:text-rose-300 disabled:cursor-not-allowed disabled:opacity-40"
                  >
                    Tắt AI tự vào lệnh
                  </button>
                </div>
                <p className="text-xs text-slate-500">
                  Khi bật, AI chỉ phân tích nến đã đóng. Có tín hiệu LONG/SHORT thì mở lệnh mô phỏng; TP theo AI, còn điểm dừng lỗ là mức chặt hơn giữa SL của AI và giới hạn lỗ bạn đặt. Sau khi lệnh đóng, bot chờ nến đóng tiếp theo rồi phân tích lại. Có thể giữ nhiều vị thế.
                </p>
                <p className="border-t border-slate-800 pt-3 text-sm font-medium text-slate-200">Hoặc mở lệnh thủ công</p>
                <div className="grid grid-cols-2 gap-3">
                  <label className="text-xs text-slate-400">
                    Cắt lỗ
                    <input
                      type="number"
                      inputMode="decimal"
                      value={stopLossInput}
                      onChange={(event) => setStopLossInput(event.target.value)}
                      placeholder={market ? `Ví dụ: ${market.last_price.toFixed(2)}` : 'Nhập giá'}
                      className="mt-1 w-full rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-white outline-none focus:border-cyan-500"
                    />
                  </label>
                  <label className="text-xs text-slate-400">
                    Chốt lời
                    <input
                      type="number"
                      inputMode="decimal"
                      value={takeProfitInput}
                      onChange={(event) => setTakeProfitInput(event.target.value)}
                      placeholder={market ? `Ví dụ: ${market.last_price.toFixed(2)}` : 'Nhập giá'}
                      className="mt-1 w-full rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-white outline-none focus:border-cyan-500"
                    />
                  </label>
                </div>
                <p className="text-xs text-slate-500">
                  Giá vào lấy từ OKX tại thời điểm đặt lệnh. Khối lượng = tiền ký quỹ × đòn bẩy ÷ giá vào.
                </p>
                <div className="grid grid-cols-2 gap-3">
                  <button
                    type="button"
                    disabled={!validManualLevels || !market}
                    onClick={() => void openPaperPosition('LONG')}
                    className="rounded-lg bg-emerald-500 px-3 py-2 text-sm font-semibold text-slate-950 hover:bg-emerald-400 disabled:cursor-not-allowed disabled:opacity-40"
                  >
                    Mở LONG
                  </button>
                  <button
                    type="button"
                    disabled={!validManualLevels || !market}
                    onClick={() => void openPaperPosition('SHORT')}
                    className="rounded-lg bg-rose-500 px-3 py-2 text-sm font-semibold text-white hover:bg-rose-400 disabled:cursor-not-allowed disabled:opacity-40"
                  >
                    Mở SHORT
                  </button>
                </div>
              </div>
              {paperError && <p role="alert" className="mt-3 rounded-lg bg-rose-500/10 p-2 text-sm text-rose-200">{paperError}</p>}
              {paperNotice && <p role="status" className="mt-3 rounded-lg bg-emerald-500/10 p-2 text-sm text-emerald-200">{paperNotice}</p>}
              {paperLoading && <p className="mt-3 text-xs text-slate-500">Đang tải tài khoản mô phỏng…</p>}
            </Panel>
          </div>
        </section>

        <section className="mb-6 grid gap-6 xl:grid-cols-[1.1fr_0.9fr]">
          <Panel title="Vị thế đang mở">
            <div className="overflow-x-auto rounded-xl border border-slate-800">
              <table className="min-w-full text-left text-sm text-slate-200">
                <thead className="bg-slate-950/80 text-slate-400">
                  <tr>
                    {['Chiều', 'Giá vào', 'Giá hiện tại', 'Khối lượng', 'Chốt lời', 'Cắt lỗ', 'Lãi / lỗ', 'Thao tác'].map((heading) => (
                      <th key={heading} className="px-3 py-2">{heading}</th>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  {paperPositions.length === 0 ? (
                    <tr>
                      <td colSpan={8} className="px-3 py-6 text-center text-slate-500">
                        Chưa có vị thế mô phỏng.
                      </td>
                    </tr>
                  ) : paperPositions.map((position) => (
                    <tr key={position.id} className="border-t border-slate-800">
                      <td className={`px-3 py-3 font-medium ${position.direction === 'LONG' ? 'text-emerald-400' : 'text-rose-400'}`}>
                        {position.direction}
                      </td>
                      <td className="px-3 py-3">{formatPrice(position.entry_price)}</td>
                      <td className="px-3 py-3">{formatPrice(position.current_price)}</td>
                      <td className="px-3 py-3">{position.quantity.toFixed(5)} BTC</td>
                      <td className="px-3 py-3">{formatPrice(position.take_profit)}</td>
                      <td className="px-3 py-3">{formatPrice(position.stop_loss)}</td>
                      <td className={`px-3 py-3 ${position.pnl >= 0 ? 'text-emerald-400' : 'text-rose-400'}`}>
                        {formatMoney(position.pnl)}
                      </td>
                      <td className="px-3 py-3">
                        <button
                          type="button"
                          onClick={() => void closePaperPosition(position.id)}
                          className="rounded border border-slate-700 px-2 py-1 text-xs hover:border-rose-400 hover:text-rose-300"
                        >
                          Đóng
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </Panel>

          <Panel title="Hiệu suất tài khoản">
            <div className="space-y-4">
              <InfoRow label="Tổng lãi/lỗ" value={paperAccount ? formatMoney(paperAccount.total_pnl) : '—'} />
              <InfoRow label="Số lệnh đã vào" value={(paperTrades.length + paperPositions.length).toString()} />
              <InfoRow label="Đã đóng / đang mở" value={`${paperTrades.length} / ${paperPositions.length}`} />
              <InfoRow label="Lãi đã chốt" value={paperAccount ? formatMoney(paperAccount.realized_pnl) : '—'} />
              <InfoRow label="Lãi/lỗ vị thế mở" value={paperAccount ? formatMoney(paperAccount.unrealized_pnl) : '—'} />
              <InfoRow
                label="Lệnh thắng / thua"
                value={`${paperTrades.filter((trade) => trade.pnl > 0).length} / ${paperTrades.filter((trade) => trade.pnl < 0).length}`}
              />
              <InfoRow
                label="Tỷ lệ thắng"
                value={paperTrades.length
                  ? `${((paperTrades.filter((trade) => trade.pnl > 0).length / paperTrades.length) * 100).toFixed(1)}%`
                  : '—'}
              />
              <p className="text-xs text-slate-500">Dữ liệu được lưu trong bộ nhớ backend và sẽ đặt lại khi backend khởi động lại.</p>
            </div>
          </Panel>
        </section>

        <section className="grid gap-6 lg:grid-cols-3">
          <Panel title="Thông tin thị trường">
            <div className="space-y-3 text-sm text-slate-300">
              <InfoRow label="Giá mua tốt nhất" value={formatPrice(market?.bid_price)} />
              <InfoRow label="Giá bán tốt nhất" value={formatPrice(market?.ask_price)} />
              <InfoRow label="Giá đánh dấu" value={formatPrice(market?.mark_price)} />
              <InfoRow label="Thời điểm dữ liệu" value={market?.timestamp ? new Date(market.timestamp).toLocaleString('vi-VN') : '—'} />
            </div>
          </Panel>
          <Panel title="Quy tắc mô phỏng">
            <div className="space-y-3 text-sm text-slate-300">
              <InfoRow label="Chế độ" value="Chỉ giao dịch mô phỏng" />
              <InfoRow label="AI tự giao dịch" value={paperSettings?.auto_trade_enabled ? `Bật · theo nến ${paperSettings.analysis_timeframe} đã đóng` : 'Tắt'} />
              <InfoRow label="Lỗ tối đa mỗi lệnh" value={paperSettings ? `${paperSettings.max_loss_percent}% ký quỹ` : '—'} />
              <InfoRow label="Cho phép nhiều vị thế" value={paperSettings?.allow_multiple_positions ? 'Có' : 'Không'} />
              <InfoRow label="Múi giờ" value="Asia/Ho_Chi_Minh" />
            </div>
          </Panel>
          <Panel title="Giao dịch gần đây">
            <div className="space-y-3 text-sm text-slate-300">
              {paperTrades.length === 0 ? (
                <p className="rounded-xl border border-dashed border-slate-800 p-4 text-center text-slate-500">
                  Chưa có giao dịch mô phỏng.
                </p>
              ) : paperTrades.slice(-5).reverse().map((trade) => (
                <div key={trade.id} className="flex items-center justify-between rounded-lg border border-slate-800 bg-slate-950/50 px-3 py-2">
                  <div>
                    <div className={`font-medium ${trade.direction === 'LONG' ? 'text-emerald-400' : 'text-rose-400'}`}>
                      {trade.direction} · {trade.close_reason === 'MANUAL' ? 'Đóng thủ công' : trade.close_reason === 'TAKE_PROFIT' ? 'Chốt lời' : trade.close_reason === 'MAX_LOSS' ? 'Chạm lỗ tối đa' : 'Cắt lỗ theo AI'}
                    </div>
                    <div className="text-xs text-slate-500">{new Date(trade.closed_at).toLocaleString('vi-VN')}</div>
                  </div>
                  <div className={trade.pnl >= 0 ? 'font-semibold text-emerald-400' : 'font-semibold text-rose-400'}>
                    {formatMoney(trade.pnl)}
                  </div>
                </div>
              ))}
            </div>
          </Panel>
        </section>
      </div>
    </main>
  )
}

function MetricCard({ label, value, tone }: { label: string; value: string; tone: 'green' | 'cyan' | 'amber' | 'violet' }) {
  const toneMap = {
    green: 'border-emerald-500/30 bg-emerald-500/10',
    cyan: 'border-cyan-500/30 bg-cyan-500/10',
    amber: 'border-amber-500/30 bg-amber-500/10',
    violet: 'border-violet-500/30 bg-violet-500/10',
  }

  return (
    <div className={`rounded-2xl border p-4 ${toneMap[tone]}`}>
      <p className="text-xs uppercase tracking-[0.2em] text-slate-400">{label}</p>
      <p className="mt-3 text-xl font-semibold text-white">{value}</p>
    </div>
  )
}

function Panel({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="rounded-2xl border border-slate-800 bg-slate-900/80 p-4 shadow-xl shadow-slate-950/30">
      <h3 className="mb-4 text-lg font-semibold text-white">{title}</h3>
      {children}
    </section>
  )
}

function InfoRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex justify-between gap-4 border-b border-slate-800 pb-2">
      <span className="text-slate-400">{label}</span>
      <span className="text-right font-medium text-slate-100">{value}</span>
    </div>
  )
}

function TradingChart({ timeframe, refreshKey }: { timeframe: string; refreshKey: number }) {
  const chartContainerRef = useRef<HTMLDivElement | null>(null)
  const seriesRef = useRef<ISeriesApi<'Candlestick'> | null>(null)
  const [candles, setCandles] = useState<CandlestickData<UTCTimestamp>[]>([])
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    if (!chartContainerRef.current) return

    const chart = createChart(chartContainerRef.current, {
      layout: {
        background: { type: ColorType.Solid, color: '#020817' },
        textColor: '#cbd5e1',
      },
      width: chartContainerRef.current.clientWidth,
      height: 360,
      grid: {
        vertLines: { color: '#1e293b' },
        horzLines: { color: '#1e293b' },
      },
      rightPriceScale: { borderColor: '#334155' },
      timeScale: { borderColor: '#334155', timeVisible: true },
    })

    const series = chart.addSeries(CandlestickSeries, {
      upColor: '#34d399',
      downColor: '#fb7185',
      borderVisible: false,
      wickUpColor: '#34d399',
      wickDownColor: '#fb7185',
    })
    seriesRef.current = series

    const resizeObserver = new ResizeObserver((entries) => {
      const width = entries[0]?.contentRect.width
      if (width) chart.applyOptions({ width })
    })
    resizeObserver.observe(chartContainerRef.current)

    return () => {
      resizeObserver.disconnect()
      seriesRef.current = null
      chart.remove()
    }
  }, [])

  useEffect(() => {
    let active = true
    const fetchCandles = async () => {
      try {
        const response = await fetch(`${API_BASE}/api/market/candles?timeframe=${encodeURIComponent(timeframe)}`)
        if (!response.ok) {
          throw new Error(`Không lấy được dữ liệu nến (HTTP ${response.status})`)
        }
        const payload = (await response.json()) as { candles: Candle[] }
        const data = payload.candles
          .map((candle) => ({
            time: candle.time as UTCTimestamp,
            open: candle.open,
            high: candle.high,
            low: candle.low,
            close: candle.close,
          }))
          .sort((left, right) => Number(left.time) - Number(right.time))

        if (!active) return
        setCandles(data)
        setError('')
      } catch (fetchError) {
        if (!active) return
        setError(fetchError instanceof Error ? fetchError.message : 'Không thể tải dữ liệu nến')
      } finally {
        if (active) setLoading(false)
      }
    }

    setLoading(true)
    void fetchCandles()
    const timer = window.setInterval(() => void fetchCandles(), 10_000)
    return () => {
      active = false
      window.clearInterval(timer)
    }
  }, [timeframe, refreshKey])

  useEffect(() => {
    if (candles.length > 0) {
      seriesRef.current?.setData(candles)
    }
  }, [candles])

  return (
    <div className="relative">
      <div ref={chartContainerRef} className="h-[360px] w-full" />
      {loading && candles.length === 0 && (
        <div className="absolute inset-0 flex items-center justify-center bg-slate-950/70 text-sm text-slate-300">
          Đang tải dữ liệu nến…
        </div>
      )}
      {!loading && candles.length === 0 && (
        <div className="absolute inset-0 flex flex-col items-center justify-center gap-2 bg-slate-950/70 px-4 text-center text-sm text-slate-300">
          <span>{error || 'Chưa có dữ liệu nến để hiển thị.'}</span>
          <span className="text-xs text-slate-500">Biểu đồ tự thử tải lại mỗi 10 giây.</span>
        </div>
      )}
      {error && candles.length > 0 && (
        <div className="absolute left-2 top-2 rounded bg-amber-950/90 px-2 py-1 text-xs text-amber-200">
          Mất cập nhật: {error}
        </div>
      )}
    </div>
  )
}

export default App
