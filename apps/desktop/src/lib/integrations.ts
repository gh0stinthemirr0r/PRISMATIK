export type IntegrationStatus = "not-connected" | "checking" | "validated" | "error";

export type CredentialField = {
  id: "apiKey" | "apiSecret" | "contactEmail";
  label: string;
  type: "password" | "email" | "text";
  help: string;
  required: boolean;
};

export type IntegrationDefinition = {
  id: string;
  name: string;
  category: "Market data" | "Macro & filings" | "Broker" | "Information";
  accent: string;
  description: string;
  capabilities: string[];
  credentialFields: CredentialField[];
  modes?: Array<{ id: string; label: string; help: string }>;
  docsUrl: string;
  privacy: string;
  cadence: string;
  readiness: "live-adapter" | "adapter-next" | "research" | "execution-gated";
  venueClass: "crypto" | "equities" | "macro" | "filings" | "prediction" | "information";
  region?: "US" | "Global" | "Europe" | "Asia-Pacific" | "Emerging markets";
  assetClasses?: string[];
};

export type SavedIntegration = {
  providerId: string;
  status: Exclude<IntegrationStatus, "checking">;
  mode?: string;
  validatedAt?: string;
  evidence?: string;
  message?: string;
};

type CatalogSeed = Pick<IntegrationDefinition, "id" | "name" | "category" | "description" | "capabilities" | "docsUrl" | "venueClass" | "region" | "assetClasses"> & { accent?: string; readiness?: IntegrationDefinition["readiness"] };

const catalog = (seed: CatalogSeed): IntegrationDefinition => ({
  accent: seed.accent ?? "#7894b8",
  credentialFields: [],
  privacy: "Catalog entry only. Activation requires a native governed adapter, entitlement review, and source policy.",
  cadence: "Provider and entitlement dependent",
  readiness: seed.readiness ?? "research",
  ...seed,
});

const GLOBAL_CATALOG: IntegrationDefinition[] = [
  catalog({ id:"databento", name:"Databento", category:"Market data", description:"Normalized and raw market data for equities, options, futures, and global venues.", capabilities:["Tick data","Order books","Futures","Options"], docsUrl:"https://databento.com/docs", venueClass:"equities", region:"Global", assetClasses:["Equities","Options","Futures"] }),
  catalog({ id:"twelve-data", name:"Twelve Data", category:"Market data", description:"Global equities, ETFs, forex, crypto, commodities, and technical time series.", capabilities:["Global equities","FX","Crypto","Commodities"], docsUrl:"https://twelvedata.com/documentation", venueClass:"equities", region:"Global", assetClasses:["Equities","FX","Crypto","Commodities"] }),
  catalog({ id:"eodhd", name:"EODHD", category:"Market data", description:"International end-of-day, intraday, fundamentals, corporate actions, and exchange reference data.", capabilities:["International EOD","Intraday","Fundamentals","Corporate actions"], docsUrl:"https://eodhd.com/financial-apis/", venueClass:"equities", region:"Global", assetClasses:["Equities","ETFs","Funds"] }),
  catalog({ id:"tiingo", name:"Tiingo", category:"Market data", description:"US and international securities, fundamentals, news, forex, and crypto data.", capabilities:["Equities","Fundamentals","News","FX & crypto"], docsUrl:"https://www.tiingo.com/documentation/general/overview", venueClass:"equities", region:"Global", assetClasses:["Equities","FX","Crypto"] }),
  catalog({ id:"intrinio", name:"Intrinio", category:"Market data", description:"US securities prices, fundamentals, options, estimates, and institutional datasets.", capabilities:["US equities","Options","Fundamentals","Estimates"], docsUrl:"https://docs.intrinio.com/documentation", venueClass:"equities", region:"US", assetClasses:["Equities","Options"] }),
  catalog({ id:"dxfeed", name:"dxFeed", category:"Market data", description:"Multi-asset real-time and historical feeds across equities, options, futures, indices, and forex.", capabilities:["Real-time feeds","Options","Futures","Indices"], docsUrl:"https://kb.dxfeed.com/en/data-services.html", venueClass:"equities", region:"Global", assetClasses:["Equities","Options","Futures","FX"] }),
  catalog({ id:"nasdaq-data-link", name:"Nasdaq Data Link", category:"Market data", description:"Financial, economic, alternative, and exchange datasets distributed through Nasdaq Data Link.", capabilities:["Datasets","Fundamentals","Alternative data","Time series"], docsUrl:"https://docs.data.nasdaq.com/", venueClass:"equities", region:"Global", assetClasses:["Equities","Macro","Alternative"] }),
  catalog({ id:"cme", name:"CME Group", category:"Market data", description:"Futures and options reference, settlements, delayed data, and licensed real-time feeds.", capabilities:["Futures","Options on futures","Settlements","Reference data"], docsUrl:"https://www.cmegroup.com/market-data.html", venueClass:"equities", region:"Global", assetClasses:["Futures","Options","Rates","Commodities"] }),
  catalog({ id:"opra", name:"OPRA", category:"Market data", description:"US listed-options consolidated quote and trade information through licensed distributors.", capabilities:["Options quotes","Options trades","NBBO","Series reference"], docsUrl:"https://www.opraplan.com/", venueClass:"equities", region:"US", assetClasses:["Options"] }),
  catalog({ id:"finra", name:"FINRA", category:"Macro & filings", description:"TRACE fixed-income trades, short interest, OTC transparency, and regulatory datasets.", capabilities:["TRACE","Short interest","OTC transparency","Reference files"], docsUrl:"https://developer.finra.org/", venueClass:"macro", region:"US", assetClasses:["Bonds","OTC equities"] }),
  catalog({ id:"treasury", name:"US Treasury Fiscal Data", category:"Macro & filings", description:"Debt, interest rates, auctions, fiscal operations, and government finance datasets.", capabilities:["Treasury rates","Debt","Auctions","Fiscal data"], docsUrl:"https://fiscaldata.treasury.gov/api-documentation/", venueClass:"macro", region:"US", assetClasses:["Rates","Sovereign debt","Macro"] }),
  catalog({ id:"world-bank", name:"World Bank", category:"Macro & filings", description:"Global development, trade, population, climate, and economic indicators.", capabilities:["Global indicators","Country data","Development","Trade"], docsUrl:"https://datahelpdesk.worldbank.org/knowledgebase/topics/125589-developer-information", venueClass:"macro", region:"Global", assetClasses:["Macro"] }),
  catalog({ id:"imf", name:"IMF Data", category:"Macro & filings", description:"International financial statistics, balance of payments, reserves, and macroeconomic datasets.", capabilities:["IFS","Balance of payments","Reserves","Country macro"], docsUrl:"https://www.imf.org/en/Data", venueClass:"macro", region:"Global", assetClasses:["Macro","FX","Sovereign"] }),
  catalog({ id:"ecb", name:"European Central Bank", category:"Macro & filings", description:"Euro-area rates, monetary aggregates, banking, FX, and securities statistics.", capabilities:["Policy rates","Yield curves","FX","Banking statistics"], docsUrl:"https://data.ecb.europa.eu/help/api/overview", venueClass:"macro", region:"Europe", assetClasses:["Rates","FX","Macro"] }),
  catalog({ id:"eurostat", name:"Eurostat", category:"Macro & filings", description:"Official European economic, trade, industry, labor, and regional statistics.", capabilities:["EU macro","Trade","Industry","Labor"], docsUrl:"https://ec.europa.eu/eurostat/web/user-guides/data-browser/api-data-access", venueClass:"macro", region:"Europe", assetClasses:["Macro"] }),
  catalog({ id:"un-comtrade", name:"UN Comtrade", category:"Macro & filings", description:"International merchandise and services trade flows by reporter, partner, and commodity.", capabilities:["Bilateral trade","Commodity flows","Monthly data","Country coverage"], docsUrl:"https://comtradeplus.un.org/", venueClass:"macro", region:"Global", assetClasses:["Trade","Macro"] }),
  catalog({ id:"oanda", name:"OANDA", category:"Broker", description:"Global foreign-exchange pricing, candles, accounts, and controlled trading workflows.", capabilities:["FX pricing","Candles","Accounts","Orders"], docsUrl:"https://developer.oanda.com/rest-live-v20/introduction/", venueClass:"equities", region:"Global", assetClasses:["FX","CFDs"], readiness:"execution-gated" }),
  catalog({ id:"schwab", name:"Charles Schwab", category:"Broker", description:"US brokerage accounts, market data, options chains, positions, and controlled order routing.", capabilities:["Accounts","US equities","Options","Orders"], docsUrl:"https://developer.schwab.com/", venueClass:"equities", region:"US", assetClasses:["Equities","Options","Funds"], readiness:"execution-gated" }),
  catalog({ id:"tradier", name:"Tradier", category:"Broker", description:"US equity and options market data, paper workflows, accounts, and order routing.", capabilities:["Equities","Options chains","Paper trading","Orders"], docsUrl:"https://documentation.tradier.com/", venueClass:"equities", region:"US", assetClasses:["Equities","Options"], readiness:"execution-gated" }),
  catalog({ id:"bybit", name:"Bybit", category:"Market data", description:"Crypto spot and derivatives order books, trades, funding, open interest, and liquidations.", capabilities:["Spot","Perpetuals","Funding","Liquidations"], docsUrl:"https://bybit-exchange.github.io/docs/v5/intro", venueClass:"crypto", region:"Global", assetClasses:["Crypto"] }),
  catalog({ id:"okx", name:"OKX", category:"Market data", description:"Crypto spot, margin, futures, perpetuals, options, and public market data.", capabilities:["Spot","Futures","Options","Order books"], docsUrl:"https://www.okx.com/docs-v5/en/", venueClass:"crypto", region:"Global", assetClasses:["Crypto"] }),
  catalog({ id:"deribit", name:"Deribit", category:"Market data", description:"Crypto options and futures order books, volatility surfaces, trades, and instruments.", capabilities:["Crypto options","Futures","Volatility","Greeks"], docsUrl:"https://docs.deribit.com/", venueClass:"crypto", region:"Global", assetClasses:["Crypto options","Crypto futures"] }),
  catalog({ id:"bitstamp", name:"Bitstamp", category:"Market data", description:"Crypto spot instruments, order books, transactions, OHLC, and streaming market data.", capabilities:["Spot","Order books","Trades","OHLC"], docsUrl:"https://www.bitstamp.net/api/", venueClass:"crypto", region:"Global", assetClasses:["Crypto"] }),
  catalog({ id:"gemini", name:"Gemini", category:"Market data", description:"Crypto spot symbols, order books, trades, candles, and market-data streams.", capabilities:["Spot","Order books","Trades","Candles"], docsUrl:"https://docs.gemini.com/", venueClass:"crypto", region:"US", assetClasses:["Crypto"] }),
  catalog({ id:"lseg", name:"LSEG Data & Analytics", category:"Market data", description:"Institutional global market, reference, estimates, news, and fundamental data.", capabilities:["Global pricing","Reference data","Estimates","News"], docsUrl:"https://developers.lseg.com/", venueClass:"equities", region:"Global", assetClasses:["Multi-asset"] }),
  catalog({ id:"ice", name:"ICE Data Services", category:"Market data", description:"Global exchange, fixed-income, reference, evaluated pricing, and market data services.", capabilities:["Exchange data","Fixed income","Evaluated pricing","Reference data"], docsUrl:"https://developer.ice.com/", venueClass:"equities", region:"Global", assetClasses:["Multi-asset","Bonds"] }),
  catalog({ id:"jpx", name:"Japan Exchange Group", category:"Market data", description:"Japanese cash equities, derivatives, indices, reference, and historical market data.", capabilities:["Japan equities","Derivatives","Indices","Reference data"], docsUrl:"https://www.jpx.co.jp/english/markets/paid-info-equities/index.html", venueClass:"equities", region:"Asia-Pacific", assetClasses:["Equities","Futures","Options"] }),
  catalog({ id:"hkex", name:"Hong Kong Exchanges", category:"Market data", description:"Hong Kong securities and derivatives market data, reference files, and issuer information.", capabilities:["HK equities","Derivatives","Issuer filings","Reference data"], docsUrl:"https://www.hkex.com.hk/Services/Market-Data-Services", venueClass:"equities", region:"Asia-Pacific", assetClasses:["Equities","Futures","Options"] }),
  catalog({ id:"sgx", name:"Singapore Exchange", category:"Market data", description:"Singapore securities, derivatives, commodities, FX futures, and reference data.", capabilities:["Singapore equities","Derivatives","Commodities","FX futures"], docsUrl:"https://www.sgx.com/research-education/derivatives-market-data", venueClass:"equities", region:"Asia-Pacific", assetClasses:["Equities","Futures","FX"] }),
  catalog({ id:"asx", name:"Australian Securities Exchange", category:"Market data", description:"Australian equities, derivatives, indices, announcements, and reference information.", capabilities:["Australia equities","Derivatives","Announcements","Indices"], docsUrl:"https://www.asx.com.au/connectivity-and-data/information-services", venueClass:"equities", region:"Asia-Pacific", assetClasses:["Equities","Futures","Options"] }),
  catalog({ id:"nse-india", name:"NSE India", category:"Market data", description:"Indian equities, derivatives, indices, corporate disclosures, and market statistics.", capabilities:["India equities","Derivatives","Indices","Disclosures"], docsUrl:"https://www.nseindia.com/market-data", venueClass:"equities", region:"Emerging markets", assetClasses:["Equities","Futures","Options"] }),
  catalog({ id:"lse", name:"London Stock Exchange", category:"Market data", description:"UK and international securities, order-book, reference, issuer, and index data.", capabilities:["UK equities","International securities","Reference data","Issuer data"], docsUrl:"https://www.londonstockexchange.com/market-data", venueClass:"equities", region:"Europe", assetClasses:["Equities","Bonds","Funds"] }),
  catalog({ id:"euronext", name:"Euronext", category:"Market data", description:"Pan-European equities, ETFs, bonds, derivatives, commodities, and indices.", capabilities:["European equities","Derivatives","Bonds","Indices"], docsUrl:"https://www.euronext.com/en/data", venueClass:"equities", region:"Europe", assetClasses:["Equities","Futures","Options","Bonds"] }),
  catalog({ id:"deutsche-boerse", name:"Deutsche Börse", category:"Market data", description:"Xetra, Börse Frankfurt, Eurex, indices, reference, and historical datasets.", capabilities:["German equities","Eurex derivatives","Indices","Reference data"], docsUrl:"https://www.deutsche-boerse.com/dbg-en/our-company/know-how/market-data", venueClass:"equities", region:"Europe", assetClasses:["Equities","Futures","Options"] }),
];

export const INTEGRATIONS: IntegrationDefinition[] = [
  {
    id: "coingecko",
    name: "CoinGecko",
    category: "Market data",
    accent: "#8ac53f",
    description: "Crypto quotes, market breadth, asset metadata, categories, and historical candles.",
    capabilities: ["Crypto quotes", "OHLC history", "Categories", "Exchange breadth"],
    credentialFields: [
      {
        id: "apiKey",
        label: "Demo API key",
        type: "password",
        help: "Sent as x-cg-demo-api-key from the desktop process.",
        required: true,
      },
    ],
    docsUrl: "https://docs.coingecko.com/demo/reference/authentication",
    privacy: "The credential remains only in native process memory for governed refreshes and is never persisted.",
    cadence: "Interactive budget · provider limits apply",
    readiness: "live-adapter",
    venueClass: "crypto",
  },
  {
    id: "alpha-vantage",
    name: "Alpha Vantage",
    category: "Market data",
    accent: "#f4b942",
    description: "Global equity quotes, time series, fundamentals, FX, commodities, and indicators.",
    capabilities: ["Equity quotes", "Time series", "Symbol search", "Fundamentals"],
    credentialFields: [
      {
        id: "apiKey",
        label: "API key",
        type: "password",
        help: "Validated against the GLOBAL_QUOTE response contract.",
        required: true,
      },
    ],
    docsUrl: "https://www.alphavantage.co/documentation/",
    privacy: "The key is transmitted to Alpha Vantage for validation and is not persisted by PRISMATIK.",
    cadence: "Plan-dependent request budget",
    readiness: "research",
    venueClass: "equities",
  },
  {
    id: "fred",
    name: "FRED / ALFRED",
    category: "Macro & filings",
    accent: "#4aa8ff",
    description: "Economic series and vintage-aware macro context from the St. Louis Fed.",
    capabilities: ["Macro series", "Release metadata", "Economic observations", "Vintage planning"],
    credentialFields: [
      {
        id: "apiKey",
        label: "32-character API key",
        type: "password",
        help: "Validated against the DGS10 series metadata endpoint.",
        required: true,
      },
    ],
    docsUrl: "https://fred.stlouisfed.org/docs/api/fred/",
    privacy: "The key remains only in native process memory for governed macro refreshes and is not persisted.",
    cadence: "Release-aware · series-specific",
    readiness: "live-adapter",
    venueClass: "macro",
  },
  {
    id: "sec-edgar",
    name: "SEC EDGAR",
    category: "Macro & filings",
    accent: "#aa8cff",
    description: "Company submissions and XBRL facts from the SEC's unauthenticated data APIs.",
    capabilities: ["Submissions", "10-K / 10-Q / 8-K", "XBRL company facts", "Filing history"],
    credentialFields: [
      {
        id: "contactEmail",
        label: "Contact email",
        type: "email",
        help: "SEC fair-access policy requires a declared User-Agent contact.",
        required: true,
      },
    ],
    docsUrl: "https://www.sec.gov/search-filings/edgar-application-programming-interfaces",
    privacy: "Your contact is sent in the SEC request User-Agent, retained only in native process memory, and never written to browser storage.",
    cadence: "Fair-access governed",
    readiness: "live-adapter",
    venueClass: "filings",
  },
  {
    id: "alpaca",
    name: "Alpaca",
    category: "Broker",
    accent: "#ffd23f",
    description: "Account-aware market data and a guarded paper or live trading connection.",
    capabilities: ["Account status", "Positions", "Orders", "Equity & option market data"],
    credentialFields: [
      {
        id: "apiKey",
        label: "API key ID",
        type: "password",
        help: "Sent as APCA-API-KEY-ID.",
        required: true,
      },
      {
        id: "apiSecret",
        label: "API secret",
        type: "password",
        help: "Sent as APCA-API-SECRET-KEY.",
        required: true,
      },
    ],
    modes: [
      { id: "paper", label: "Paper", help: "Recommended. Validation reads market data only; no order is ever submitted." },
      { id: "live", label: "Live", help: "Connection check only. No order is submitted." },
    ],
    docsUrl: "https://docs.alpaca.markets/us/docs/authentication",
    privacy: "Credentials remain in native process memory for this session and are never written to browser storage or to disk. Reconnect after restarting.",
    cadence: "Streaming + request budgets",
    readiness: "live-adapter",
    venueClass: "equities",
  },
  {
    id: "coinbase",
    name: "Coinbase Advanced",
    category: "Market data",
    accent: "#4f7cff",
    description: "Public Advanced Trade products and market-data reachability.",
    capabilities: ["Product catalog", "Public market data", "WebSocket planning"],
    credentialFields: [],
    docsUrl: "https://docs.cdp.coinbase.com/coinbase-app/advanced-trade-apis/rest-api",
    privacy: "This connection uses public scope and stores no Coinbase credential.",
    cadence: "Public market feed",
    readiness: "adapter-next",
    venueClass: "crypto",
  },
  {
    id: "gdelt",
    name: "GDELT",
    category: "Information",
    accent: "#f06cff",
    description: "Global multilingual event and document discovery for the information corpus.",
    capabilities: ["Global discovery", "Document search", "Language breadth", "Source topology"],
    credentialFields: [],
    docsUrl: "https://blog.gdeltproject.org/gdelt-doc-2-0-api-debuts/",
    privacy: "Connectivity validation only. Enabling collection still requires a registered source policy.",
    cadence: "Discovery updates approximately every 15 minutes",
    readiness: "research",
    venueClass: "information",
  },
  {
    id: "finnhub",
    name: "Finnhub",
    category: "Market data",
    accent: "#35d399",
    description: "Equity candles and quote fallback through PRISMATIK's existing Layer-2 adapter.",
    capabilities: ["Equity bars", "Quote fallback", "Symbol coverage"],
    credentialFields: [{ id: "apiKey", label: "API token", type: "password", help: "Used by the governed Finnhub adapter.", required: true }],
    docsUrl: "https://finnhub.io/docs/api",
    privacy: "The token remains only in native process memory for governed equity refreshes and is never persisted.",
    cadence: "Plan-dependent request budget",
    readiness: "live-adapter",
    venueClass: "equities",
  },
  {
    id: "polygon",
    name: "Polygon.io",
    category: "Market data",
    accent: "#8b5cf6",
    description: "US equities, options, indices, forex, and crypto reference and market data.",
    capabilities: ["Trades & quotes", "Aggregates", "Options", "Reference data"],
    credentialFields: [{ id: "apiKey", label: "API key", type: "password", help: "Requires a governed adapter and source policy.", required: true }],
    docsUrl: "https://polygon.io/docs",
    privacy: "Catalog entry only; no request is sent until the adapter and terms profile are approved.",
    cadence: "Entitlement-dependent",
    readiness: "research",
    venueClass: "equities",
  },
  {
    id: "cftc",
    name: "CFTC",
    category: "Macro & filings",
    accent: "#ff9f43",
    description: "Commitments of Traders positioning through the existing public-data adapter.",
    capabilities: ["COT reports", "Dealer positioning", "Historical releases"],
    credentialFields: [],
    docsUrl: "https://publicreporting.cftc.gov/",
    privacy: "Public structured data; collection still records observation time and source policy.",
    cadence: "Weekly release-aware",
    readiness: "adapter-next",
    venueClass: "macro",
  },
  {
    id: "polymarket",
    name: "Polymarket",
    category: "Market data",
    accent: "#60a5fa",
    description: "Prediction-market probabilities, liquidity, spreads, and order-book history.",
    capabilities: ["Event probabilities", "Order books", "Liquidity", "Resolution metadata"],
    credentialFields: [],
    docsUrl: "https://docs.polymarket.com/",
    privacy: "Research catalog entry pending endpoint, terms, and resolution-provenance review.",
    cadence: "Streaming candidate",
    readiness: "research",
    venueClass: "prediction",
  },
  {
    id: "kalshi",
    name: "Kalshi",
    category: "Market data",
    accent: "#10b981",
    description: "Regulated event contracts with venue metadata, order books, and settlement evidence.",
    capabilities: ["Event contracts", "Order books", "Settlement", "Cross-venue comparison"],
    credentialFields: [{ id: "apiKey", label: "API key", type: "password", help: "Activation requires a governed adapter and venue review.", required: true }],
    docsUrl: "https://docs.kalshi.com/",
    privacy: "Catalog entry only; trading and account scopes remain disabled.",
    cadence: "Streaming candidate",
    readiness: "execution-gated",
    venueClass: "prediction",
  },
  {
    id: "kraken",
    name: "Kraken",
    category: "Market data",
    accent: "#7c3aed",
    description: "Crypto spot and derivatives market data with a future paper-first execution boundary.",
    capabilities: ["Spot books", "Trades", "OHLC", "Derivatives"],
    credentialFields: [],
    docsUrl: "https://docs.kraken.com/api/",
    privacy: "Public market data is the proposed first scope; private trading is gated.",
    cadence: "WebSocket candidate",
    readiness: "research",
    venueClass: "crypto",
  },
  {
    id: "binance",
    name: "Binance",
    category: "Market data",
    accent: "#f0b90b",
    description: "Spot and derivatives order books, trades, funding, open interest, and liquidations.",
    capabilities: ["Order books", "Funding", "Open interest", "Liquidations"],
    credentialFields: [],
    docsUrl: "https://developers.binance.com/docs/binance-spot-api-docs",
    privacy: "Research catalog entry; jurisdiction, terms, and source policy must be resolved first.",
    cadence: "WebSocket candidate",
    readiness: "research",
    venueClass: "crypto",
  },
  {
    id: "interactive-brokers",
    name: "Interactive Brokers",
    category: "Broker",
    accent: "#ef4444",
    description: "Multi-asset brokerage candidate for account, market-data, and controlled execution workflows.",
    capabilities: ["Multi-asset data", "Positions", "Orders", "Reconciliation"],
    credentialFields: [],
    docsUrl: "https://www.interactivebrokers.com/campus/ibkr-api-page/",
    privacy: "Execution remains blocked until account, entitlement, and operational controls are approved.",
    cadence: "Session and entitlement dependent",
    readiness: "execution-gated",
    venueClass: "equities",
  },
  ...GLOBAL_CATALOG,
];

export const INTEGRATION_STORAGE_KEY = "prismatik-integrations-v1";
