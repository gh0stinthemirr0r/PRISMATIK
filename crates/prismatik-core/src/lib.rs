pub enum AssetType {
    Stock,
    Crypto,
    Forex,
    Commodity,
}

#[derive(Debug, Clone)]
pub struct MarketAsset {
    pub symbol: String,
    pub asset_type: AssetType,
    pub exchange: String,
    pub precision: u32,
}

#[derive(Debug, Clone)]
pub struct Tick {
    pub asset: MarketAsset,
    pub price: f64,
    pub volume: f64,
    pub timestamp: u64,
    pub bid: Option<f64>,
    pub ask: Option<f64>,
}

pub trait TickStreamer {
    fn next_tick(&mut self) -> Option<Tick>;
}

pub struct MockStreamer {
    pub asset: MarketAsset,
    pub current_price: f64,
}

impl MockStreamer {
    pub fn new(asset: MarketAsset, initial_price: f64) -> Self {
        Self {
            asset,
            current_price: initial_price,
        }
    }
}

impl TickStreamer for MockStreamer {
    fn next_tick(&mut self) -> Option<Tick> {
        // Simulate a small price movement
        self.current_price += (rand::random::<f64>() - 0.5);
        Some(Tick {
            asset: self.asset.clone(),
            price: self.current_price,
            volume: rand::random::<f64>() * 100.0,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            bid: Some(self.current_price - 0.1),
            ask: Some(self.current_price + 0.1),
        })
    }
}

pub struct DataManager {
    pub assets: Vec<MarketAsset>,
    pub streams: Vec<Box<dyn TickStreamer>>,
}

impl DataManager {
    pub fn new() -> Self {
        Self {
            assets: Vec::new(),
            streams: Vec::new(),
        }
    }

    pub fn add_asset(&mut self, asset: MarketAsset) {
        self.assets.push(asset);
    }

    pub fn add_streamer(&mut self, streamer: Box<dyn TickStreamer>) {
        self.streams.push(streamer);
    }

    pub fn update_all_ticks(&mut self) -> Vec<Tick> {
        let mut ticks = Vec::new();
        for streamer in &mut self.streams {
            if let Some(tick) = streamer.next_tick() {
                ticks.push(tick);
            }
        }
        ticks
    }
}
