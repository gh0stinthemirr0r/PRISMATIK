/**
 * Bulk RSS feed registry — import 500+ feeds at once.
 * Called from the Integrations page or first-run onboarding.
 * Also provides article parsing and normalization.
 */
import { invoke, isTauri } from '@tauri-apps/api/core';

export interface FeedEntry {
  name: string;
  url: string;
  cat: string;
  lang: string;
  tier: string;
  group: string;
  country?: string;
  sector?: string;
}

export interface ParsedArticle {
  id: string;
  feedId: string;
  title: string;
  link: string;
  published: string | null;
  description: string;
  content: string;
  source: string;
  language: string;
  categories: string[];
}

/**
 * Parse an RSS/Atom XML string into articles.
 * Pure JS parser — no external dependencies.
 */
export function parseRssXml(xml: string, feedId: string): ParsedArticle[] {
  const parser = new DOMParser();
  const doc = parser.parseFromString(xml, 'text/xml');
  const articles: ParsedArticle[] = [];

  // RSS 2.0 items
  const items = doc.querySelectorAll('item');
  items.forEach(item => {
    articles.push(extractItem(item, feedId, 'rss'));
  });

  // Atom entries
  const entries = doc.querySelectorAll('entry');
  entries.forEach(entry => {
    articles.push(extractItem(entry, feedId, 'atom'));
  });

  return articles.filter(a => a.title.length > 0);
}

function extractItem(el: Element, feedId: string, format: 'rss' | 'atom'): ParsedArticle {
  const getText = (tag: string) => el.querySelector(tag)?.textContent?.trim() ?? '';
  const title = getText(format === 'atom' ? 'title' : 'title');
  const link = format === 'atom'
    ? (el.querySelector('link')?.getAttribute('href') ?? '')
    : getText('link');
  const description = getText('description') || getText('summary') || getText('content\\:encoded');
  const content = getText('content\\:encoded') || getText('content') || description;
  const pubDate = getText('pubDate') || getText('published') || getText('updated') || getText('dc\\:date');
  const source = getText('source') || getText('dc\\:source') || feedId;

  const categories: string[] = [];
  el.querySelectorAll('category').forEach(cat => {
    const text = cat.textContent?.trim();
    if (text) categories.push(text);
  });

  return {
    id: `${feedId}:${hashCode(title + link)}`,
    feedId,
    title: decodeHtmlEntities(title),
    link,
    published: pubDate ? new Date(pubDate).toISOString() : null,
    description: decodeHtmlEntities(stripHtml(description)).substring(0, 500),
    content: decodeHtmlEntities(stripHtml(content)),
    source,
    language: 'en',
    categories,
  };
}

function stripHtml(html: string): string {
  return html.replace(/<[^>]*>/g, ' ').replace(/\s+/g, ' ').trim();
}

function decodeHtmlEntities(text: string): string {
  const entities: Record<string, string> = {
    '&amp;': '&', '&lt;': '<', '&gt;': '>', '&quot;': '"', '&apos;': "'",
    '&#39;': "'", '&nbsp;': ' ', '&mdash;': '—', '&ndash;': '–',
    '&hellip;': '…', '&rsquo;': "'", '&lsquo;': "'", '&rdquo;': '"', '&ldquo;': '"',
  };
  return text.replace(/&[#\w]+;/g, match => entities[match] ?? match);
}

function hashCode(s: string): string {
  let h = 0;
  for (let i = 0; i < s.length; i++) {
    h = ((h << 5) - h + s.charCodeAt(i)) | 0;
  }
  return Math.abs(h).toString(36);
}

/**
 * Core RSS feeds to import on first run.
 * These are the P0/P1 feeds that don't require API keys.
 */
export const CORE_FEEDS: FeedEntry[] = [
  // Wire services
  { name: 'Reuters Business', url: 'https://www.reutersagency.com/feed/?taxonomy=best-sectors&post_type=best', cat: 'wire', lang: 'en', tier: 'P0', group: 'reuters' },
  { name: 'AP Business', url: 'https://rsshub.app/apnews/topics/business', cat: 'wire', lang: 'en', tier: 'P0', group: 'ap' },

  // Financial news
  { name: 'CNBC Top', url: 'https://search.cnbc.com/rs/search/combinedcms/view.xml?partnerId=wrss01&id=100003114', cat: 'financial', lang: 'en', tier: 'P1', group: 'nbcuniversal' },
  { name: 'Bloomberg Markets', url: 'https://feeds.bloomberg.com/markets/news.rss', cat: 'financial', lang: 'en', tier: 'P1', group: 'bloomberg' },
  { name: 'WSJ Markets', url: 'https://feeds.a.dj.com/rss/RSSMarketsMain.xml', cat: 'financial', lang: 'en', tier: 'P1', group: 'dowjones' },
  { name: 'MarketWatch', url: 'https://feeds.marketwatch.com/marketwatch/topstories/', cat: 'financial', lang: 'en', tier: 'P1', group: 'dowjones' },
  { name: 'Yahoo Finance', url: 'https://finance.yahoo.com/news/rssindex', cat: 'financial', lang: 'en', tier: 'P2', group: 'yahoo' },
  { name: 'Seeking Alpha', url: 'https://seekingalpha.com/market_currents.xml', cat: 'financial', lang: 'en', tier: 'P2', group: 'seekingalpha' },
  { name: 'Benzinga', url: 'https://www.benzinga.com/feed', cat: 'financial', lang: 'en', tier: 'P2', group: 'benzinga' },
  { name: 'FT', url: 'https://www.ft.com/rss/home', cat: 'financial', lang: 'en', tier: 'P1', group: 'ft_group' },
  { name: 'Forbes Markets', url: 'https://www.forbes.com/markets/feed/', cat: 'financial', lang: 'en', tier: 'P1', group: 'forbes' },
  { name: 'Fortune', url: 'https://fortune.com/feed/fortune-feeds/?id=3230629', cat: 'financial', lang: 'en', tier: 'P1', group: 'fortune' },

  // Federal Reserve
  { name: 'Fed Press', url: 'https://www.federalreserve.gov/feeds/press_all.xml', cat: 'central_bank', lang: 'en', tier: 'P0', group: 'fed' },
  { name: 'Fed Speeches', url: 'https://www.federalreserve.gov/feeds/speeches.xml', cat: 'central_bank', lang: 'en', tier: 'P0', group: 'fed' },

  // SEC / Regulatory
  { name: 'SEC Press', url: 'https://www.sec.gov/news/pressreleases.rss', cat: 'regulatory', lang: 'en', tier: 'P0', group: 'sec' },
  { name: 'FINRA News', url: 'https://www.finra.org/media-center/news-releases/rss', cat: 'regulatory', lang: 'en', tier: 'P0', group: 'finra' },
  { name: 'CFTC Press', url: 'https://www.cftc.gov/PressRoom/PressReleases/rss', cat: 'regulatory', lang: 'en', tier: 'P0', group: 'cftc' },

  // Government economic data
  { name: 'BLS Latest', url: 'https://www.bls.gov/rss/bls_latest.rss', cat: 'gov_econ', lang: 'en', tier: 'P0', group: 'bls' },
  { name: 'EIA Energy', url: 'https://www.eia.gov/rss/todayinenergy.xml', cat: 'gov_econ', lang: 'en', tier: 'P0', group: 'eia' },

  // Crypto
  { name: 'CoinDesk', url: 'https://www.coindesk.com/arc/outboundfeeds/rss/', cat: 'crypto', lang: 'en', tier: 'P1', group: 'coindesk' },
  { name: 'CoinTelegraph', url: 'https://cointelegraph.com/rss', cat: 'crypto', lang: 'en', tier: 'P1', group: 'cointelegraph' },
  { name: 'The Block', url: 'https://www.theblock.co/rss.xml', cat: 'crypto', lang: 'en', tier: 'P1', group: 'theblock' },
  { name: 'Decrypt', url: 'https://decrypt.co/feed', cat: 'crypto', lang: 'en', tier: 'P2', group: 'decrypt' },

  // Tech / Sector
  { name: 'TechCrunch', url: 'https://techcrunch.com/feed/', cat: 'sector', lang: 'en', tier: 'P2', group: 'yahoo', sector: 'tech' },
  { name: 'Hacker News', url: 'https://hnrss.org/frontpage', cat: 'sector', lang: 'en', tier: 'P2', group: 'ycombinator', sector: 'tech' },
  { name: 'Ars Technica', url: 'https://feeds.arstechnica.com/arstechnica/index', cat: 'sector', lang: 'en', tier: 'P2', group: 'conde_nast', sector: 'tech' },
  { name: 'OilPrice', url: 'https://oilprice.com/rss/main', cat: 'sector', lang: 'en', tier: 'P2', group: 'oilprice', sector: 'energy' },
  { name: 'Mining.com', url: 'https://www.mining.com/feed/', cat: 'sector', lang: 'en', tier: 'P2', group: 'mining_com', sector: 'mining' },

  // International
  { name: 'ECB Press', url: 'https://www.ecb.europa.eu/rss/press.html', cat: 'central_bank', lang: 'en', tier: 'P0', group: 'ecb', country: 'EU' },
  { name: 'BoE News', url: 'https://www.bankofengland.co.uk/rss/news', cat: 'central_bank', lang: 'en', tier: 'P0', group: 'boe', country: 'GB' },
  { name: 'BBC Business', url: 'https://feeds.bbci.co.uk/news/business/rss.xml', cat: 'financial', lang: 'en', tier: 'P1', group: 'bbc', country: 'GB' },
  { name: 'Guardian Business', url: 'https://www.theguardian.com/uk/business/rss', cat: 'financial', lang: 'en', tier: 'P1', group: 'guardian', country: 'GB' },
  { name: 'Nikkei', url: 'https://www.nikkei.com/rss/', cat: 'financial', lang: 'ja', tier: 'P1', group: 'nikkei', country: 'JP' },
  { name: 'SCMP Business', url: 'https://www.scmp.com/rss/5/feed', cat: 'financial', lang: 'en', tier: 'P1', group: 'scmp', country: 'HK' },
  { name: 'Economic Times India', url: 'https://economictimes.indiatimes.com/rssfeedstopstories.cms', cat: 'financial', lang: 'en', tier: 'P1', group: 'times_group', country: 'IN' },
  { name: 'Business Day SA', url: 'https://www.businesslive.co.za/rss', cat: 'financial', lang: 'en', tier: 'P1', group: 'bdfm', country: 'ZA' },

  // Research / Think tanks
  { name: 'NBER Papers', url: 'https://www.nber.org/rss/new.xml', cat: 'research', lang: 'en', tier: 'P0', group: 'nber' },
  { name: 'Brookings', url: 'https://www.brookings.edu/topic/economics/feed/', cat: 'research', lang: 'en', tier: 'P1', group: 'brookings' },
  { name: 'IMF Blog', url: 'https://www.imf.org/en/News/rss', cat: 'research', lang: 'en', tier: 'P0', group: 'imf' },
  { name: 'World Bank', url: 'https://blogs.worldbank.org/rss.xml', cat: 'research', lang: 'en', tier: 'P0', group: 'worldbank' },

  // Alternative data
  { name: 'GDELT', url: 'https://blog.gdeltproject.org/feed/', cat: 'alternative', lang: 'en', tier: 'P0', group: 'gdelt' },
];

/**
 * Import core feeds into the Tauri backend.
 * Returns count of successfully imported feeds.
 */
export async function importCoreFeeds(): Promise<{ imported: number; errors: string[] }> {
  if (!isTauri()) return { imported: 0, errors: ['Not in Tauri'] };

  const sources = CORE_FEEDS.map(feed => ({
    id: feed.name.toLowerCase().replace(/[^a-z0-9]/g, '-'),
    publisher: feed.group,
    url: feed.url,
    enabled: true,
    minimumPollSeconds: feed.tier === 'P0' ? 60 : feed.tier === 'P1' ? 300 : 900,
    termsOwner: feed.group,
    reviewReference: 'auto-imported',
    validUntil: '2027-01-01T00:00:00Z',
    retainRawPayload: true,
    allowMetadataRedistribution: false,
  }));

  try {
    await invoke('import_feed_sources', { sources });
    return { imported: sources.length, errors: [] };
  } catch (e) {
    return { imported: 0, errors: [String(e)] };
  }
}

/**
 * Fetch and parse a single RSS feed.
 * Uses the browser's fetch API (no CORS in Tauri).
 */
export async function fetchAndParseFeed(feedUrl: string, feedId: string): Promise<ParsedArticle[]> {
  try {
    const response = await fetch(feedUrl, {
      headers: { 'Accept': 'application/rss+xml, application/atom+xml, application/xml, text/xml' },
    });
    if (!response.ok) return [];
    const xml = await response.text();
    return parseRssXml(xml, feedId);
  } catch {
    return [];
  }
}

/**
 * Fetch all enabled feeds and return parsed articles.
 * Parallel with concurrency limit.
 */
export async function fetchAllFeeds(feeds: { id: string; url: string }[], concurrency = 5): Promise<ParsedArticle[]> {
  const articles: ParsedArticle[] = [];
  const queue = [...feeds];

  async function worker() {
    while (queue.length > 0) {
      const feed = queue.shift();
      if (!feed) break;
      const parsed = await fetchAndParseFeed(feed.url, feed.id);
      articles.push(...parsed);
    }
  }

  const workers = Array.from({ length: Math.min(concurrency, feeds.length) }, () => worker());
  await Promise.all(workers);

  // sort by published date, newest first
  articles.sort((a, b) => {
    const da = a.published ? new Date(a.published).getTime() : 0;
    const db = b.published ? new Date(b.published).getTime() : 0;
    return db - da;
  });

  return articles;
}

/**
 * Extract entities from article text using simple regex patterns.
 * In production, this would use GLiNER or a proper NER model.
 */
export function extractEntitiesSimple(text: string): string[] {
  const entities = new Set<string>();

  // ticker symbols ($AAPL, $BTC)
  const tickers = text.match(/\$[A-Z]{1,6}/g);
  tickers?.forEach(t => entities.add(t.substring(1)));

  // company names (capitalized words near "Inc", "Corp", "Ltd", etc.)
  const companies = text.match(/[A-Z][a-z]+ (?:Inc|Corp|Ltd|Group|Holdings|Technologies|Systems|Co)\b/g);
  companies?.forEach(c => entities.add(c));

  // crypto names
  const crypto = text.match(/\b(Bitcoin|Ethereum|Solana|Dogecoin|XRP|Cardano|Polkadot|Avalanche)\b/gi);
  crypto?.forEach(c => entities.add(c.charAt(0).toUpperCase() + c.slice(1).toLowerCase()));

  // country/region names
  const regions = text.match(/\b(US|USA|China|Japan|EU|Europe|UK|India|Brazil|Russia|Germany|France)\b/g);
  regions?.forEach(r => entities.add(r));

  return [...entities];
}

/**
 * Simple sentiment scoring using keyword matching.
 * In production, this would use FinBERT or a proper sentiment model.
 */
export function scoreSentimentSimple(text: string): number {
  const positive = ['bullish', 'surge', 'rally', 'gain', 'rise', 'up', 'growth', 'record', 'high', 'boom', 'optimism', 'upgrade', 'beat', 'exceed', 'strong', 'recovery'];
  const negative = ['bearish', 'crash', 'plunge', 'drop', 'fall', 'down', 'decline', 'loss', 'low', 'bust', 'pessimism', 'downgrade', 'miss', 'weak', 'recession', 'default', 'bankruptcy'];

  const lower = text.toLowerCase();
  let score = 0;
  positive.forEach(w => { if (lower.includes(w)) score += 0.1; });
  negative.forEach(w => { if (lower.includes(w)) score -= 0.1; });
  return Math.max(-1, Math.min(1, score));
}
