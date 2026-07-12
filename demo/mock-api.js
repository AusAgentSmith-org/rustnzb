/**
 * Mock API interceptor for rustnzb demo.
 * Intercepts XMLHttpRequest calls to /api/* and /dav/* and returns mock data.
 * Must be loaded BEFORE Angular boots.
 */
(function () {
  'use strict';

  // Ensure demo user appears logged in
  if (!localStorage.getItem('access_token')) {
    localStorage.setItem('access_token', 'demo-token-mock');
    localStorage.setItem('refresh_token', 'demo-refresh-mock');
  }

  // ---------- Mock Data ----------

  var logSeq = 0;
  var startTime = new Date('2026-07-11T18:42:00Z').getTime();

  var tokenResponse = {
    access_token: 'demo-token-mock', refresh_token: 'demo-refresh-mock',
    token_type: 'Bearer', expires_in: 3600
  };

  var servers = [
    {
      id: 'primary', name: 'Northstar Primary', host: 'primary.demo.invalid', port: 563,
      ssl: true, verify_cert: true, username: 'demo', password: '', connections: 24,
      priority: 0, enabled: true, optional: false, pipelining: 16,
      ramp_up_delay_ms: 75, connect_timeout_secs: 15, retention_days: 6200,
      compression: true, proxy_url: null
    },
    {
      id: 'fill', name: 'Bluefin Fill', host: 'fill.demo.invalid', port: 563,
      ssl: true, verify_cert: true, username: 'demo', password: '', connections: 8,
      priority: 1, enabled: true, optional: true, pipelining: 8,
      ramp_up_delay_ms: 120, connect_timeout_secs: 12, retention_days: 4300,
      compression: true, proxy_url: null
    }
  ];

  var serverStats = [
    { server_id: 'primary', server_name: 'Northstar Primary', total_bytes: 4821993103360, today_bytes: 155692564480, week_bytes: 902748733440, month_bytes: 2711198105600, total_ok: 9812442, today_ok: 301122, week_ok: 1844290, month_ok: 5518802, total_fail: 8421, today_fail: 212, week_fail: 1519, month_fail: 4810, last_active: '2026-07-11T18:48:35Z' },
    { server_id: 'fill', server_name: 'Bluefin Fill', total_bytes: 318901452800, today_bytes: 5211422720, week_bytes: 44560220160, month_bytes: 129922760704, total_ok: 648210, today_ok: 10542, week_ok: 90118, month_ok: 263480, total_fail: 1198, today_fail: 31, week_fail: 194, month_fail: 512, last_active: '2026-07-11T18:48:31Z' }
  ];

  var jobs = [
    {
      id: 'q1', name: 'Orbital.District.S02E07.2160p.WEB-DL.DDP5.1.HDR.x265-FAKE', category: 'tv',
      status: 'downloading', priority: 2, total_bytes: 8589934592, downloaded_bytes: 5755256176,
      file_count: 88, files_completed: 58, article_count: 11824, articles_downloaded: 7921,
      articles_failed: 3, added_at: '2026-07-11T18:35:00Z', completed_at: null,
      speed_bps: 68157440, error_message: null,
      server_stats: [{ server_id: 'primary', server_name: 'Northstar Primary', articles_downloaded: 7812, articles_failed: 2, bytes_downloaded: 5672796160 }, { server_id: 'fill', server_name: 'Bluefin Fill', articles_downloaded: 109, articles_failed: 1, bytes_downloaded: 82460016 }]
    },
    {
      id: 'q2', name: 'Harbor.Unit.S01E09.1080p.WEB.H264-FAKE', category: 'tv',
      status: 'downloading', priority: 1, total_bytes: 3435973836, downloaded_bytes: 1217629380,
      file_count: 36, files_completed: 12, article_count: 4712, articles_downloaded: 1670,
      articles_failed: 0, added_at: '2026-07-11T18:39:00Z', completed_at: null,
      speed_bps: 33554432, error_message: null,
      server_stats: [{ server_id: 'primary', server_name: 'Northstar Primary', articles_downloaded: 1670, articles_failed: 0, bytes_downloaded: 1217629380 }]
    },
    {
      id: 'q3', name: 'Clockwork.Coast.S03E04.1080p.BLURAY.DTS.x264-FAKE', category: 'tv',
      status: 'queued', priority: 1, total_bytes: 5368709120, downloaded_bytes: 0,
      file_count: 55, files_completed: 0, article_count: 7362, articles_downloaded: 0,
      articles_failed: 0, added_at: '2026-07-11T18:41:00Z', completed_at: null,
      speed_bps: 0, error_message: null, server_stats: []
    },
    {
      id: 'q4', name: 'Signal.Zero.S01E01-E02.2160p.WEB-DL.DV.H265-FAKE', category: 'tv',
      status: 'verifying', priority: 3, total_bytes: 12884901888, downloaded_bytes: 12884901888,
      file_count: 129, files_completed: 129, article_count: 17694, articles_downloaded: 17694,
      articles_failed: 11, added_at: '2026-07-11T18:02:00Z', completed_at: null,
      speed_bps: 0, error_message: null,
      server_stats: [{ server_id: 'primary', server_name: 'Northstar Primary', articles_downloaded: 17431, articles_failed: 9, bytes_downloaded: 12693231411 }, { server_id: 'fill', server_name: 'Bluefin Fill', articles_downloaded: 263, articles_failed: 2, bytes_downloaded: 191670477 }]
    },
    {
      id: 'q5', name: 'Paper.Moons.S04E10.1080p.WEB-DL.AAC2.0.H264-FAKE', category: 'tv',
      status: 'paused', priority: 0, total_bytes: 2684354560, downloaded_bytes: 1342177280,
      file_count: 28, files_completed: 14, article_count: 3681, articles_downloaded: 1840,
      articles_failed: 0, added_at: '2026-07-11T17:50:00Z', completed_at: null,
      speed_bps: 0, error_message: null, server_stats: []
    }
  ];

  function historyEntry(id, name, category, status, bytes, started, completed, error) {
    var ok = status === 'completed';
    return {
      id: id, name: name, category: category, status: status, total_bytes: bytes,
      downloaded_bytes: ok ? bytes : Math.round(bytes * 0.72), added_at: started,
      completed_at: completed, output_dir: ok ? '/downloads/' + category + '/' + name : '',
      stages: ok ? [
        { name: 'Download', status: 'completed', message: 'All articles received', duration_secs: 134 },
        { name: 'Par2 Verify', status: 'completed', message: 'All files intact', duration_secs: 18 },
        { name: 'Extract', status: 'completed', message: 'Archive extracted', duration_secs: 42 },
        { name: 'Cleanup', status: 'completed', message: 'Temporary files removed', duration_secs: 3 }
      ] : [
        { name: 'Download', status: 'completed', message: 'Partial article set received', duration_secs: 206 },
        { name: 'Par2 Verify', status: 'failed', message: 'Missing 18 recovery blocks', duration_secs: 31 }
      ],
      error_message: error || null,
      server_stats: [{ server_id: 'primary', server_name: 'Northstar Primary', articles_downloaded: 8120, articles_failed: ok ? 2 : 196, bytes_downloaded: ok ? bytes : Math.round(bytes * 0.7) }],
      has_nzb_data: true, duration_secs: ok ? 197 : 237,
      average_speed_bps: ok ? 75497472 : 24117248,
      articles_served: ok ? 8120 : 5930, articles_missing: ok ? 2 : 196
    };
  }

  var history = [
    historyEntry('h1', 'Lighthouse.Nine.S02E06.2160p.WEB-DL.HDR.x265-FAKE', 'tv', 'completed', 9663676416, '2026-07-11T16:22:00Z', '2026-07-11T16:25:17Z'),
    historyEntry('h2', 'Juniper.Files.S01E08.1080p.WEB.H264-FAKE', 'tv', 'completed', 3006477107, '2026-07-11T15:58:00Z', '2026-07-11T16:00:31Z'),
    historyEntry('h3', 'The.Glass.Archipelago.2026.2160p.BLURAY.x265-FAKE', 'movies', 'completed', 19327352832, '2026-07-11T14:42:00Z', '2026-07-11T14:48:44Z'),
    historyEntry('h4', 'Cedar.Court.S05E03.1080p.WEB-DL.H264-FAKE', 'tv', 'completed', 2899102924, '2026-07-10T22:12:00Z', '2026-07-10T22:14:20Z'),
    historyEntry('h5', 'Neon.Naturalists.S01E05.1080p.WEB.H264-FAKE', 'documentaries', 'failed', 4294967296, '2026-07-10T20:02:00Z', '2026-07-10T20:05:57Z', 'Par2 repair failed: insufficient recovery data'),
    historyEntry('h6', 'Quiet.Atlas.2025.1080p.BLURAY.DTS.x264-FAKE', 'movies', 'completed', 12884901888, '2026-07-10T17:31:00Z', '2026-07-10T17:35:12Z'),
    historyEntry('h7', 'Saffron.Skies.S03E11.1080p.WEB-DL.H264-FAKE', 'tv', 'completed', 3758096384, '2026-07-10T12:07:00Z', '2026-07-10T12:09:09Z'),
    historyEntry('h8', 'Deep.Field.Notes.E04.2160p.WEB-DL.H265-FAKE', 'documentaries', 'completed', 7516192768, '2026-07-09T23:48:00Z', '2026-07-09T23:51:28Z'),
    historyEntry('h9', 'Night.Bus.Sessions.Vol.12.FLAC-FAKE', 'music', 'completed', 1288490189, '2026-07-09T19:20:00Z', '2026-07-09T19:20:49Z'),
    historyEntry('h10', 'Mapmakers.Handbook.Third.Edition-FAKE', 'books', 'completed', 644245094, '2026-07-09T08:14:00Z', '2026-07-09T08:14:37Z'),
    historyEntry('h11', 'Copper.Lake.S01E03.2160p.WEB-DL.DV.H265-FAKE', 'tv', 'failed', 10737418240, '2026-07-08T21:03:00Z', '2026-07-08T21:07:18Z', 'Download failed: 342 articles are outside server retention'),
    historyEntry('h12', 'Small.Hours.2024.1080p.BLURAY.x264-FAKE', 'movies', 'completed', 15032385536, '2026-07-08T16:42:00Z', '2026-07-08T16:47:26Z')
  ];

  var groups = [
    { id: 1, name: 'alt.binaries.tv.fictional', description: 'Fictional television releases', subscribed: true, article_count: 18422, first_article: 1, last_article: 18422, last_scanned: 18412, last_updated: '2026-07-11T18:44:00Z', created_at: '2026-06-01T00:00:00Z', unread_count: 10 },
    { id: 2, name: 'alt.binaries.movies.fictional', description: 'Fictional film releases', subscribed: true, article_count: 9188, first_article: 1, last_article: 9188, last_scanned: 9188, last_updated: '2026-07-11T18:40:00Z', created_at: '2026-06-01T00:00:00Z', unread_count: 4 },
    { id: 3, name: 'alt.binaries.documentaries.demo', description: 'Demo documentary posts', subscribed: true, article_count: 4210, first_article: 1, last_article: 4210, last_scanned: 4210, last_updated: '2026-07-11T18:32:00Z', created_at: '2026-06-01T00:00:00Z', unread_count: 0 },
    { id: 4, name: 'alt.binaries.audio.demo', description: 'Demo audio posts', subscribed: false, article_count: 12501, first_article: 1, last_article: 12501, last_scanned: 0, last_updated: null, created_at: '2026-06-01T00:00:00Z', unread_count: 0 }
  ];

  var headers = [
    ['Orbital District S02E08 2160p WEB-DL DDP5.1 HDR x265-FAKE', 'atlas@demo.invalid', 7423911936],
    ['Harbor Unit S01E10 1080p WEB H264-FAKE', 'northstar@demo.invalid', 3274912563],
    ['Clockwork Coast S03E05 1080p BLURAY DTS x264-FAKE', 'poster42@demo.invalid', 5213235200],
    ['Paper Moons S04E11 1080p WEB-DL AAC2.0 H264-FAKE', 'luna@demo.invalid', 2818572288],
    ['Signal Zero S01E03 2160p WEB-DL DV H265-FAKE', 'relay@demo.invalid', 6442450944]
  ].map(function (h, i) {
    return { id: i + 1, group_id: 1, article_num: 18422 - i, subject: h[0], author: h[1], date: '2026-07-11 18:' + (44 - i), message_id: '<demo-' + (i + 1) + '@rustnzb.invalid>', references_: '', bytes: h[2], lines: 12000 + i * 700, read: i > 1, downloaded_at: '' };
  });

  var periodMonth = { downloads: 184, completed: 179, failed: 5, bytes_downloaded: 2711198105600, total_duration_secs: 48120, average_speed_bps: 73610035, fastest_download_bps: 126877696, news_server_hits: 5789600, articles_served: 5784780, articles_missing: 4820 };

  var dailyStatistics = [];
  for (var dayOffset = 0; dayOffset < 30; dayOffset++) {
    var day = new Date(Date.UTC(2026, 6, 11 - dayOffset));
    var downloads = 4 + ((dayOffset * 7 + 3) % 9);
    var failed = dayOffset % 11 === 0 ? 1 : 0;
    var served = 104000 + downloads * 13721 + dayOffset * 911;
    var missing = 38 + ((dayOffset * 47) % 206);
    dailyStatistics.push({
      date: day.toISOString().slice(0, 10),
      downloads: downloads,
      completed: downloads - failed,
      failed: failed,
      bytes_downloaded: downloads * 12884901888 + ((dayOffset * 2147483648) % 17179869184),
      total_duration_secs: downloads * 241 + dayOffset * 13,
      average_speed_bps: 62914560 + ((dayOffset * 5767168) % 39845888),
      fastest_download_bps: 104857600 + ((dayOffset * 4194304) % 33554432),
      news_server_hits: served + missing,
      articles_served: served,
      articles_missing: missing
    });
  }

  var MOCK = {
    '/api/auth/status': { auth_enabled: true, setup_required: false },
    '/api/auth/login': tokenResponse,
    '/api/auth/setup': tokenResponse,
    '/api/auth/refresh': tokenResponse,

    '/api/status': {
      version: '1.3.2-demo', speed_bps: 101711872, speed_limit_bps: 0,
      queue_size: 5,
      disk_space_free: 827854929920, disk_space_total: 2199023255552,
      min_free_space_bytes: 10737418240, paused: false, pause_remaining_secs: null,
      webdav_available: true, webdav_enabled: true
    },

    '/api/queue': {
      jobs: jobs, total: 5, speed_bps: 101711872, paused: false
    },

    '/api/history': {
      entries: history
    },

    '/api/config/categories': [
      { name: 'tv', output_dir: '/downloads/tv', post_processing: 3 },
      { name: 'movies', output_dir: '/downloads/movies', post_processing: 3 },
      { name: 'documentaries', output_dir: '/downloads/documentaries', post_processing: 3 },
      { name: 'music', output_dir: '/downloads/music', post_processing: 2 },
      { name: 'books', output_dir: '/downloads/books', post_processing: 2 }
    ],

    '/api/config/servers': servers,
    '/api/config/servers/stats': serverStats,
    '/api/config': { general: { data_dir: '/config', download_dir: '/downloads', complete_dir: '/downloads/complete', incomplete_dir: '/downloads/incomplete', watch_dir: '/watch', temp_dir: '/data/tmp', log_dir: '/data/logs' } },

    '/api/config/speed-limit': { speed_limit_bps: 0 },
    '/api/config/max-active-downloads': { max_active_downloads: 3 },
    '/api/config/history-retention': { retention: 90 },
    '/api/config/dav': { enabled: true, auto_send_all: false, category_rules: ['tv', 'movies'], username: 'demo', password: null, api_key: 'demo••••••••key' },

    '/api/config/rss-feeds': [
      { name: 'Fictional TV Releases', url: 'https://indexer.demo.invalid/rss?apikey=hidden', poll_interval_secs: 900, category: 'tv', filter_regex: 'S\\d{2}E\\d{2}', enabled: true, auto_download: true },
      { name: 'Demo Cinema', url: 'https://cinema.demo.invalid/feed', poll_interval_secs: 1800, category: 'movies', filter_regex: '2160p|1080p', enabled: true, auto_download: false },
      { name: 'Documentary Watch', url: 'https://docs.demo.invalid/rss', poll_interval_secs: 3600, category: 'documentaries', filter_regex: null, enabled: false, auto_download: false },
      { name: 'Lossless Listening', url: 'https://audio.demo.invalid/lossless', poll_interval_secs: 2700, category: 'music', filter_regex: 'FLAC|24bit', enabled: true, auto_download: false },
      { name: 'Weekend Reads', url: 'https://books.demo.invalid/new', poll_interval_secs: 7200, category: 'books', filter_regex: 'EPUB|MOBI|PDF', enabled: true, auto_download: false }
    ],
    '/api/rss/rules': [
      { id: 'r1', name: 'New fictional episodes', feed_names: ['Fictional TV Releases'], category: 'tv', priority: 2, match_regex: 'S\\d{2}E\\d{2}.*(2160p|1080p)', enabled: true },
      { id: 'r2', name: 'Demo films in 4K', feed_names: ['Demo Cinema'], category: 'movies', priority: 1, match_regex: '2160p.*(WEB-DL|BLURAY)', enabled: true },
      { id: 'r3', name: 'Lossless albums', feed_names: ['Lossless Listening'], category: 'music', priority: 0, match_regex: '(FLAC|24bit).*(WEB|CD)', enabled: true },
      { id: 'r4', name: 'Technical ebooks', feed_names: ['Weekend Reads'], category: 'books', priority: -1, match_regex: '(Handbook|Guide|Reference).*(EPUB|PDF)', enabled: false }
    ],
    '/api/rss/items': [
      { id: 'i1', feed_name: 'Fictional TV Releases', title: 'Orbital.District.S02E08.2160p.WEB-DL.HDR.x265-FAKE', url: 'https://demo.invalid/i1.nzb', published_at: '2026-07-11T18:40:00Z', downloaded: false, category: 'tv', size_bytes: 7423911936 },
      { id: 'i2', feed_name: 'Fictional TV Releases', title: 'Harbor.Unit.S01E09.1080p.WEB.H264-FAKE', url: 'https://demo.invalid/i2.nzb', published_at: '2026-07-11T18:15:00Z', downloaded: true, category: 'tv', size_bytes: 3435973836 },
      { id: 'i3', feed_name: 'Demo Cinema', title: 'Glass.Archipelago.2026.2160p.BLURAY.x265-FAKE', url: 'https://demo.invalid/i3.nzb', published_at: '2026-07-11T17:30:00Z', downloaded: true, category: 'movies', size_bytes: 19327352832 },
      { id: 'i4', feed_name: 'Documentary Watch', title: 'Neon.Naturalists.S01E06.1080p.WEB.H264-FAKE', url: 'https://demo.invalid/i4.nzb', published_at: '2026-07-10T22:00:00Z', downloaded: false, category: 'documentaries', size_bytes: 4080218931 },
      { id: 'i5', feed_name: 'Fictional TV Releases', title: 'Clockwork.Coast.S03E05.1080p.BLURAY.DTS.x264-FAKE', url: 'https://demo.invalid/i5.nzb', published_at: '2026-07-10T20:44:00Z', downloaded: false, category: 'tv', size_bytes: 5213235200 },
      { id: 'i6', feed_name: 'Demo Cinema', title: 'Quiet.Atlas.2025.1080p.BLURAY.DTS.x264-FAKE', url: 'https://demo.invalid/i6.nzb', published_at: '2026-07-10T17:20:00Z', downloaded: true, category: 'movies', size_bytes: 12884901888 },
      { id: 'i7', feed_name: 'Lossless Listening', title: 'Night.Bus.Sessions.Vol.13.24bit.FLAC.WEB-FAKE', url: 'https://demo.invalid/i7.nzb', published_at: '2026-07-10T14:05:00Z', downloaded: false, category: 'music', size_bytes: 1717986918 },
      { id: 'i8', feed_name: 'Weekend Reads', title: 'Practical.Signal.Processing.Handbook.2026.PDF-FAKE', url: 'https://demo.invalid/i8.nzb', published_at: '2026-07-10T11:30:00Z', downloaded: false, category: 'books', size_bytes: 188743680 },
      { id: 'i9', feed_name: 'Documentary Watch', title: 'Deep.Field.Notes.E05.2160p.WEB-DL.H265-FAKE', url: 'https://demo.invalid/i9.nzb', published_at: '2026-07-09T23:15:00Z', downloaded: false, category: 'documentaries', size_bytes: 8053063680 },
      { id: 'i10', feed_name: 'Fictional TV Releases', title: 'Paper.Moons.S04E11.1080p.WEB-DL.H264-FAKE', url: 'https://demo.invalid/i10.nzb', published_at: '2026-07-09T21:42:00Z', downloaded: true, category: 'tv', size_bytes: 2818572288 },
      { id: 'i11', feed_name: 'Lossless Listening', title: 'Amber.Terminal.Live.2025.FLAC.CD-FAKE', url: 'https://demo.invalid/i11.nzb', published_at: '2026-07-09T16:10:00Z', downloaded: true, category: 'music', size_bytes: 966367641 },
      { id: 'i12', feed_name: 'Weekend Reads', title: 'The.Container.Field.Guide.4th.Edition.EPUB-FAKE', url: 'https://demo.invalid/i12.nzb', published_at: '2026-07-09T08:00:00Z', downloaded: false, category: 'books', size_bytes: 50331648 }
    ],

    '/api/groups': { groups: groups, total: groups.length, limit: 100, offset: 0 },
    '/api/statistics': {
      generated_at: '2026-07-11T18:48:40Z',
      lifetime: { downloads: 1241, completed: 1212, failed: 29, bytes_downloaded: 5140894556160, total_duration_secs: 391020, average_speed_bps: 70254592, fastest_download_bps: 137363456, news_server_hits: 10470072, articles_served: 10460652, articles_missing: 9420 },
      today: { downloads: 12, completed: 11, failed: 1, bytes_downloaded: 160904007680, total_duration_secs: 2801, average_speed_bps: 80530636, fastest_download_bps: 126877696, news_server_hits: 312007, articles_served: 311764, articles_missing: 243 },
      week: { downloads: 47, completed: 45, failed: 2, bytes_downloaded: 947308953600, total_duration_secs: 12004, average_speed_bps: 77489766, fastest_download_bps: 131072000, news_server_hits: 1936121, articles_served: 1934408, articles_missing: 1713 },
      month: periodMonth, servers: serverStats,
      daily: dailyStatistics
    },
    '/api/dav/status': { queue: [{ job_name: 'Signal.Zero.S01E01-E02.2160p.WEB-DL.DV.H265-FAKE', queued_at: '2026-07-11T18:46:00Z' }], history: [{ job_name: 'Neon.Naturalists.S01E05.1080p.WEB.H264-FAKE', status: 'failed', fail_message: 'NZB no longer available from indexer', completed_at: '2026-07-10T20:08:00Z' }] },
    '/api/setup/status': { has_servers: false },
    '/api/setup/import-sabnzbd-api': {
      servers: [{ name: 'Imported Demo Server', host: 'import.demo.invalid', port: 563, username: 'demo', password: null, password_masked: false, connections: 20, priority: 0, ssl: true, enabled: true, optional: false }],
      categories: [{ name: 'tv', output_dir: '/downloads/tv', post_processing: 3 }, { name: 'movies', output_dir: '/downloads/movies', post_processing: 3 }],
      general: { speed_limit_bps: 0, complete_dir: '/downloads', incomplete_dir: '/downloads/incomplete' },
      rss_feeds: [{ name: 'Imported Fictional Feed', url: 'https://feed.demo.invalid/rss', poll_interval_secs: 900, category: 'tv' }],
      warnings: ['Demo preview only — no live SABnzbd instance was contacted.'], skipped_fields: []
    },
    '/api/setup/import-sabnzbd': {
      servers: [{ name: 'Imported Demo Server', host: 'import.demo.invalid', port: 563, username: 'demo', password: null, password_masked: false, connections: 20, priority: 0, ssl: true, enabled: true, optional: false }],
      categories: [{ name: 'tv', output_dir: '/downloads/tv', post_processing: 3 }],
      general: { speed_limit_bps: 0, complete_dir: '/downloads', incomplete_dir: '/downloads/incomplete' },
      rss_feeds: [], warnings: ['Demo preview only — the selected file was not uploaded.'], skipped_fields: []
    },
    '/api/setup/apply': { status: true }
  };

  function generateLogs(afterSeq) {
    var catalog = [
      ['INFO', 'rustnzb::startup', 'rustnzb 1.3.2-demo started; data directory /config'],
      ['INFO', 'nzb_web::server', 'HTTP API listening on 0.0.0.0:9090'],
      ['INFO', 'nzb_news::pool', 'Northstar Primary: established 24 TLS connections'],
      ['INFO', 'nzb_news::pool', 'Bluefin Fill: established 8 TLS connections'],
      ['INFO', 'nzb_web::queue', 'Restored 5 jobs from the persistent queue'],
      ['DEBUG', 'nzb_core::db', 'SQLite WAL checkpoint complete in 3 ms'],
      ['INFO', 'nzb_web::rss_monitor', 'Polling Fictional TV Releases'],
      ['INFO', 'nzb_web::rss_monitor', 'Feed returned 18 items; 2 new, 1 matched download rules'],
      ['INFO', 'nzb_web::rss_monitor', 'Auto-enqueued Harbor.Unit.S01E09.1080p.WEB.H264-FAKE'],
      ['INFO', 'nzb_dispatch::engine', 'Article 7922/11824 downloaded from Northstar Primary (750 KB)'],
      ['DEBUG', 'nzb_decode::yenc', 'Decoded 750 KB in 0.12 ms using AVX2'],
      ['INFO', 'nzb_dispatch::engine', 'Article 1671/4712 downloaded from Northstar Primary (750 KB)'],
      ['INFO', 'nzb_web::queue', 'Speed 97.0 MB/s (Northstar Primary 88.4 MB/s; Bluefin Fill 8.6 MB/s)'],
      ['INFO', 'nzb_core::assembler', 'Completed orbital.district.s02e07.part059.rar (100 MB)'],
      ['WARN', 'nzb_news::server', 'Article unavailable on Northstar Primary; cascading to Bluefin Fill'],
      ['INFO', 'nzb_news::server', 'Bluefin Fill supplied missing article in 184 ms'],
      ['INFO', 'nzb_postproc::par2', 'Signal.Zero: PAR2 verification 64% complete'],
      ['DEBUG', 'nzb_postproc::par2', 'Verified 83 of 129 files; no repair required so far'],
      ['INFO', 'nzb_web::queue', 'Queue: 2 downloading, 1 queued, 1 verifying, 1 paused'],
      ['INFO', 'nzb_web::disk', '771.0 GB free on /downloads'],
      ['INFO', 'nzb_web::rss_monitor', 'Polling Demo Cinema'],
      ['INFO', 'nzb_web::rss_monitor', 'Feed returned 25 items; 3 new, manual grab mode'],
      ['INFO', 'nzb_web::rss_monitor', 'Polling Lossless Listening'],
      ['INFO', 'nzb_web::rss_monitor', 'Rule Lossless albums matched Night.Bus.Sessions.Vol.13'],
      ['DEBUG', 'nzb_news::pool', 'Keepalive completed for 32 connections'],
      ['INFO', 'nzb_core::assembler', 'Writing orbital.district.s02e07.part060.rar (100 MB)'],
      ['WARN', 'nzb_web::queue', 'Paper.Moons remains paused by user request'],
      ['INFO', 'nzb_postproc::par2', 'Signal.Zero: all 129 files verified'],
      ['INFO', 'nzb_postproc::extract', 'Signal.Zero: extracting 117 archive volumes'],
      ['DEBUG', 'nzb_postproc::extract', 'Extraction throughput 412.7 MB/s'],
      ['INFO', 'nzb_web::statistics', 'Recorded 160.9 GB downloaded in the last 24 hours'],
      ['INFO', 'nzb_web::rss_monitor', 'Polling Weekend Reads'],
      ['WARN', 'nzb_web::rss_monitor', 'Weekend Reads took 2.8 s to respond'],
      ['INFO', 'nzb_web::rss_monitor', 'Feed returned 9 items; 1 new item retained for review'],
      ['DEBUG', 'nzb_core::cache', 'Article cache: 384 entries, 91.4% hit rate'],
      ['INFO', 'nzb_web::dav', 'Media library index refreshed: 4 releases available'],
      ['ERROR', 'nzb_news::server', 'Demo retention check rejected 1 old article on Northstar Primary'],
      ['INFO', 'nzb_news::server', 'Retention fallback succeeded on Bluefin Fill'],
      ['INFO', 'nzb_web::queue', 'Current aggregate speed 101.7 MB/s'],
      ['DEBUG', 'nzb_web::metrics', 'Published queue and NNTP metrics snapshot']
    ];
    var entries = [];
    var base = afterSeq || 0;
    var count = base === 0 ? catalog.length : 3;
    for (var i = 0; i < count; i++) {
      var seq = base + i + 1;
      var ts = new Date(startTime + seq * 1200).toISOString();
      var sample = catalog[(seq - 1) % catalog.length];
      entries.push({
        seq: seq,
        timestamp: ts,
        level: sample[0],
        target: sample[1],
        message: sample[2]
      });
    }
    logSeq = entries[entries.length - 1].seq;
    return { entries: entries };
  }

  function generateHistoryLogs(id) {
    var entries = [];
    var msgs = [
      { level: 'INFO', msg: 'Download started' },
      { level: 'INFO', msg: 'Connected to Northstar Primary (24 connections)' },
      { level: 'INFO', msg: 'Downloading articles: 0/15000' },
      { level: 'INFO', msg: 'Speed: 85.2 MB/s' },
      { level: 'INFO', msg: 'Downloading articles: 7500/15000 (50%)' },
      { level: 'INFO', msg: 'Downloading articles: 15000/15000 (100%)' },
      { level: 'INFO', msg: 'Download complete, starting verification' },
      { level: 'INFO', msg: 'Par2 verify: all files intact' },
      { level: 'INFO', msg: 'Extracting archives...' },
      { level: 'INFO', msg: 'Extracted 1 archive to /downloads/tv' },
      { level: 'INFO', msg: 'Post-processing complete' }
    ];
    for (var i = 0; i < msgs.length; i++) {
      entries.push({
        seq: i + 1,
        timestamp: new Date(startTime - 86400000 + i * 300000).toISOString(),
        level: msgs[i].level,
        message: msgs[i].msg
      });
    }
    return { entries: entries };
  }

  function davXml(path) {
    var releases = [
      'Lighthouse.Nine.S02E06.2160p.WEB-DL.HDR.x265-FAKE',
      'Juniper.Files.S01E08.1080p.WEB.H264-FAKE',
      'The.Glass.Archipelago.2026.2160p.BLURAY.x265-FAKE',
      'Signal.Zero.S01E01-E02.2160p.WEB-DL.DV.H265-FAKE'
    ];
    function response(href, name, isDir, size, type) {
      return '<d:response><d:href>' + href + '</d:href><d:propstat><d:prop>' +
        '<d:displayname>' + name + '</d:displayname>' +
        '<d:resourcetype>' + (isDir ? '<d:collection/>' : '') + '</d:resourcetype>' +
        '<d:getcontentlength>' + (size || 0) + '</d:getcontentlength>' +
        '<d:getcontenttype>' + (type || '') + '</d:getcontenttype>' +
        '</d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response>';
    }
    var rows = [];
    if (path === '/dav/content' || path === '/dav/content/') {
      rows.push(response('/content/', 'content', true));
      releases.forEach(function (name) {
        rows.push(response('/content/' + encodeURIComponent(name) + '/', name, true));
      });
    } else {
      var name = decodeURIComponent(path.split('/').filter(Boolean).pop() || 'Release');
      var base = '/content/' + encodeURIComponent(name) + '/';
      rows.push(response(base, name, true));
      rows.push(response(base + encodeURIComponent(name + '.mkv'), name + '.mkv', false, 6871947673, 'video/x-matroska'));
      rows.push(response(base + 'poster.jpg', 'poster.jpg', false, 1482752, 'image/jpeg'));
      rows.push(response(base + 'release.nfo', 'release.nfo', false, 8192, 'text/plain'));
    }
    return '<?xml version="1.0" encoding="utf-8"?><d:multistatus xmlns:d="DAV:">' + rows.join('') + '</d:multistatus>';
  }

  // ---------- XHR Interceptor ----------

  var RealXHR = window.XMLHttpRequest;

  function MockXHR() {
    this._real = new RealXHR();
    this._method = '';
    this._url = '';
    this._requestHeaders = {};
    this._responseHeaders = {};
    this._intercepted = false;
    this._mockResponse = '';
    this._mockStatus = 200;

    // Copy event handler properties
    this.onreadystatechange = null;
    this.onload = null;
    this.onerror = null;
    this.onabort = null;
    this.ontimeout = null;
    this.onloadend = null;
    this.onloadstart = null;
    this.onprogress = null;
    this.upload = this._real.upload;
  }

  MockXHR.prototype.open = function (method, url) {
    this._method = method;
    this._url = url;

    // Check if this URL should be intercepted
    var path = typeof url === 'string' ? new URL(url, window.location.origin).pathname : '';
    if (path.indexOf('/api/') === 0 || path.indexOf('/dav/') === 0) {
      this._intercepted = true;
      if (method === 'PROPFIND' && path.indexOf('/dav/') === 0) {
        this._mockResponse = davXml(path);
        this._responseHeaders = { 'content-type': 'application/xml; charset=utf-8' };
        this._mockStatus = 200;
      } else {
        var response = this._resolveResponse(method, path + (String(url).indexOf('?') >= 0 ? '?' + String(url).split('?')[1] : ''));
        this._mockResponse = JSON.stringify(response === undefined ? { success: true } : response);
        this._responseHeaders = { 'content-type': 'application/json' };
        this._mockStatus = 200;
      }
    } else {
      this._real.open.apply(this._real, arguments);
    }
  };

  MockXHR.prototype._resolveResponse = function (method, url) {
    // Strip query string for matching
    var path = url.split('?')[0];
    var qs = url.indexOf('?') !== -1 ? url.split('?')[1] : '';

    if (path === '/api/logs' && method === 'GET') {
      var afterSeq = 0;
      if (qs) {
        var match = qs.match(/after_seq=(\d+)/);
        if (match) afterSeq = parseInt(match[1], 10);
      }
      return generateLogs(afterSeq);
    }

    if (MOCK[path] !== undefined && (method === 'GET' || path.indexOf('/api/auth/') === 0 || path.indexOf('/api/setup/') === 0)) {
      return MOCK[path];
    }

    if (method === 'GET' && /^\/api\/history\/[^/]+$/.test(path)) {
      return history.find(function (entry) { return entry.id === path.split('/')[3]; }) || history[0];
    }

    // History item logs
    if (method === 'GET' && /^\/api\/history\/[^/]+\/logs$/.test(path)) {
      var id = path.split('/')[3];
      return generateHistoryLogs(id);
    }

    if (method === 'GET' && /^\/api\/groups\/\d+\/status$/.test(path)) {
      var groupId = Number(path.split('/')[3]);
      var group = groups.find(function (item) { return item.id === groupId; }) || groups[0];
      return { group_id: group.id, name: group.name, last_scanned: group.last_scanned, last_article: group.last_article, new_available: Math.max(0, group.last_article - group.last_scanned), total_headers: group.article_count, unread_count: group.unread_count, last_updated: group.last_updated };
    }

    if (method === 'GET' && /^\/api\/groups\/\d+\/headers$/.test(path)) {
      return { headers: headers, total: headers.length, limit: 50, offset: 0 };
    }

    if (method === 'GET' && path.indexOf('/api/articles/') === 0) {
      return { body: 'This is a safe demo article preview.\n\nSubject metadata and sample payload details are shown here without connecting to a real Usenet server.\n\nGenerated exclusively for the rustnzb interactive demo.' };
    }

    if (method === 'POST' && path === '/api/queue/pause') {
      MOCK['/api/status'].paused = true;
      MOCK['/api/status'].speed_bps = 0;
      MOCK['/api/queue'].paused = true;
      return { success: true };
    }

    if (method === 'POST' && path === '/api/queue/resume') {
      MOCK['/api/status'].paused = false;
      MOCK['/api/status'].speed_bps = 101711872;
      MOCK['/api/queue'].paused = false;
      return { success: true };
    }

    if (method === 'POST' && path === '/api/groups/refresh') {
      return { success: true, message: 'Demo newsgroup list refreshed' };
    }

    if (method === 'POST' && /\/headers\/download$/.test(path)) {
      return { success: true, message: 'Selected demo headers added to Downloads' };
    }

    if (method === 'POST' && /\/test$/.test(path)) {
      return { success: true, message: 'Connection successful (demo)' };
    }

    // For POST/PUT/DELETE, return success
    if (method !== 'GET') {
      return { success: true };
    }

    return undefined;
  };

  MockXHR.prototype.send = function () {
    if (!this._intercepted) {
      this._real.send.apply(this._real, arguments);
      return;
    }

    var self = this;

    // Simulate async response
    setTimeout(function () {
      Object.defineProperty(self, 'readyState', { value: 4, writable: true, configurable: true });
      Object.defineProperty(self, 'status', { value: self._mockStatus, writable: true, configurable: true });
      Object.defineProperty(self, 'statusText', { value: 'OK', writable: true, configurable: true });
      Object.defineProperty(self, 'responseText', { value: self._mockResponse, writable: true, configurable: true });
      Object.defineProperty(self, 'response', { value: self._mockResponse, writable: true, configurable: true });
      if (!self._responseHeaders['content-type']) self._responseHeaders = { 'content-type': 'application/json' };

      if (typeof self.onreadystatechange === 'function') {
        self.onreadystatechange(new Event('readystatechange'));
      }
      if (typeof self.onload === 'function') {
        self.onload(new ProgressEvent('load'));
      }
      if (typeof self.onloadend === 'function') {
        self.onloadend(new ProgressEvent('loadend'));
      }

      // Dispatch events for Angular's zone detection
      try {
        self.dispatchEvent(new Event('readystatechange'));
        self.dispatchEvent(new ProgressEvent('load'));
        self.dispatchEvent(new ProgressEvent('loadend'));
      } catch (e) {
        // dispatchEvent may not work on plain object
      }
    }, 15);
  };

  MockXHR.prototype.setRequestHeader = function (name, value) {
    this._requestHeaders[name.toLowerCase()] = value;
    if (!this._intercepted) {
      this._real.setRequestHeader(name, value);
    }
  };

  MockXHR.prototype.getResponseHeader = function (name) {
    if (this._intercepted) {
      return this._responseHeaders[name.toLowerCase()] || null;
    }
    return this._real.getResponseHeader(name);
  };

  MockXHR.prototype.getAllResponseHeaders = function () {
    if (this._intercepted) {
      var result = '';
      for (var key in this._responseHeaders) {
        result += key + ': ' + this._responseHeaders[key] + '\r\n';
      }
      return result;
    }
    return this._real.getAllResponseHeaders();
  };

  MockXHR.prototype.abort = function () {
    if (!this._intercepted) {
      this._real.abort();
    }
  };

  MockXHR.prototype.addEventListener = function () {
    if (!this._intercepted) {
      this._real.addEventListener.apply(this._real, arguments);
    } else {
      // Store listeners for intercepted requests
      if (!this._listeners) this._listeners = {};
      var type = arguments[0];
      var fn = arguments[1];
      if (!this._listeners[type]) this._listeners[type] = [];
      this._listeners[type].push(fn);
    }
  };

  MockXHR.prototype.removeEventListener = function () {
    if (!this._intercepted) {
      this._real.removeEventListener.apply(this._real, arguments);
    }
  };

  MockXHR.prototype.dispatchEvent = function (event) {
    if (this._listeners && this._listeners[event.type]) {
      var fns = this._listeners[event.type];
      for (var i = 0; i < fns.length; i++) {
        fns[i].call(this, event);
      }
    }
  };

  MockXHR.prototype.overrideMimeType = function () {
    if (!this._intercepted) {
      this._real.overrideMimeType.apply(this._real, arguments);
    }
  };

  // Proxy readonly properties from real XHR for non-intercepted requests
  ['readyState', 'status', 'statusText', 'responseText', 'response', 'responseType',
   'responseURL', 'responseXML', 'timeout', 'withCredentials'].forEach(function (prop) {
    var descriptor = {
      get: function () {
        if (this._intercepted) return undefined;
        return this._real[prop];
      },
      set: function (val) {
        if (!this._intercepted) {
          this._real[prop] = val;
        }
      },
      configurable: true
    };
    Object.defineProperty(MockXHR.prototype, prop, descriptor);
  });

  // Copy static properties
  MockXHR.UNSENT = 0;
  MockXHR.OPENED = 1;
  MockXHR.HEADERS_RECEIVED = 2;
  MockXHR.LOADING = 3;
  MockXHR.DONE = 4;

  // Replace global XMLHttpRequest
  window.XMLHttpRequest = MockXHR;

  console.log('[rustnzb demo] Mock API layer active');
})();
