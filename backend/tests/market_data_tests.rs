#[test]
fn parse_okx_ticker_snapshot() {
    let payload = r#"{ "data": [{ "last": "68500.5", "markPx": "68490.5", "bidPx": "68495", "askPx": "68505", "volCcy24h": "2500.0", "high24h": "70200", "low24h": "64800" }] }"#;
    let value: serde_json::Value = serde_json::from_str(payload).unwrap();
    let item = &value["data"][0];

    assert_eq!(item["last"].as_str().unwrap(), "68500.5");
    assert_eq!(item["bidPx"].as_str().unwrap(), "68495");
}

#[test]
fn indicator_snapshot_is_consistent() {
    let prices = vec![100.0, 101.0, 102.0, 103.0, 104.0, 105.0];
    let snapshot = btc_futures_ai_lab_backend::analysis::build_snapshot(&prices);
    assert!(snapshot.ema_20 > 0.0);
    assert!(
        snapshot.regime.contains("Bullish")
            || snapshot.regime.contains("Sideways")
            || snapshot.regime.contains("Bearish")
    );
}

#[test]
fn market_snapshot_uses_supplied_ohlcv_values() {
    let closes: Vec<f64> = (0..30).map(|index| 100.0 + index as f64).collect();
    let highs: Vec<f64> = closes.iter().map(|price| price + 2.0).collect();
    let lows: Vec<f64> = closes.iter().map(|price| price - 2.0).collect();
    let mut volumes = vec![1.0; closes.len()];
    *volumes.last_mut().unwrap() = 3.0;

    let snapshot = btc_futures_ai_lab_backend::analysis::build_market_snapshot(
        &closes, &highs, &lows, &volumes,
    );

    assert_eq!(snapshot.atr_14, Some(4.0));
    assert!(snapshot.relative_volume.unwrap() > 2.0);
    assert_eq!(snapshot.support, Some(108.0));
    assert_eq!(snapshot.resistance, Some(131.0));
}
