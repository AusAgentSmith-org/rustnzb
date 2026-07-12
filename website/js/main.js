// rustnzb.dev — fills the article Date header and upgrades download links
// with the latest GitHub release. Every link works without JS: it falls
// back to the GitHub releases page.

(function () {
    'use strict';

    // NNTP-style date header (RFC 5322-ish), purely decorative.
    var dateEl = document.querySelector('.js-date');
    if (dateEl) {
        dateEl.textContent = new Date().toUTCString().replace('GMT', '+0000');
    }

    function formatSize(bytes) {
        if (!bytes) return '';
        var mb = bytes / (1024 * 1024);
        return mb >= 1 ? mb.toFixed(1) + ' MB' : Math.round(bytes / 1024) + ' KB';
    }

    fetch('https://api.github.com/repos/AusAgentSmith-org/rustnzb/releases/latest', {
        headers: { Accept: 'application/vnd.github+json' }
    })
        .then(function (r) { return r.ok ? r.json() : Promise.reject(); })
        .then(function (release) {
            var tag = release.tag_name;
            if (!tag) return;

            document.querySelectorAll('.js-ver').forEach(function (el) {
                el.textContent = tag;
            });

            var note = document.querySelector('.js-relnote');
            if (note && release.published_at) {
                note.textContent = '— released ' + release.published_at.slice(0, 10);
            }

            var latestBtn = document.querySelector('.js-latest');
            if (latestBtn) {
                latestBtn.href = release.html_url;
            }

            var assets = release.assets || [];
            document.querySelectorAll('.js-asset').forEach(function (link) {
                var suffix = link.dataset.suffix;
                var prefix = link.dataset.prefix;
                var asset = assets.find(function (a) {
                    return suffix ? a.name.endsWith(suffix)
                        : prefix ? a.name.indexOf(prefix) === 0
                        : false;
                });
                if (!asset) return;
                link.href = asset.browser_download_url;
                var file = link.querySelector('.dl-file');
                if (file) file.textContent = asset.name;
                var size = link.querySelector('.js-size');
                if (size) size.textContent = formatSize(asset.size);
            });
        })
        .catch(function () { /* static fallback links remain */ });
})();
