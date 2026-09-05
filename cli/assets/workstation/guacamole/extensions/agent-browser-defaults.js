/*
 * Applies agent-browser's Guacamole input default once per browser origin.
 * The migration marker allows later user-selected input methods to persist.
 */
(function applyAgentBrowserGuacamoleDefaults() {
    'use strict';

    var preferencesKey = 'GUAC_PREFERENCES';
    var migrationKey = 'AGENT_BROWSER_GUAC_DEFAULTS_VERSION';
    var migrationVersion = '1';

    try {
        var storage = window.localStorage;
        if (!storage || storage.getItem(migrationKey) === migrationVersion)
            return;

        var storedPreferences = storage.getItem(preferencesKey);
        var preferences = storedPreferences ? JSON.parse(storedPreferences) : {};
        if (!preferences || typeof preferences !== 'object' || Array.isArray(preferences))
            preferences = {};

        preferences.inputMethod = 'text';
        storage.setItem(preferencesKey, JSON.stringify(preferences));
        storage.setItem(migrationKey, migrationVersion);
    }
    catch (ignore) {
        // Guacamole already tolerates unavailable browser-local storage.
    }
}());

/*
 * A restricted sharing key is redeemed inside the sibling Guacamole origin,
 * which the dashboard cannot inspect. Report only the attempt identity and
 * bounded outcome so the parent can discard a rejected key and run a fresh
 * primary election. Tokens, keys, URLs, response bodies, and credentials must
 * never cross the frame boundary.
 */
(function installAgentBrowserShareAuthSignal() {
    'use strict';

    var frameNamePrefix = 'agent-browser-guacamole-share:';
    var attemptId = typeof window.name === 'string' && window.name.indexOf(frameNamePrefix) === 0
        ? window.name.substring(frameNamePrefix.length)
        : '';
    if (!attemptId || window.parent === window)
        return;

    var labels = window.location.hostname.split('.');
    var shareSuffix = '-share';
    if (!labels[0] || labels[0].slice(-shareSuffix.length) !== shareSuffix)
        return;
    labels[0] = labels[0].slice(0, -shareSuffix.length);
    if (!labels[0])
        return;

    var parentOrigin = window.location.protocol + '//' + labels.join('.')
        + (window.location.port ? ':' + window.location.port : '');
    var lastOutcome = '';

    var reportStatus = function reportStatus(status) {
        var outcome = status >= 200 && status < 300
            ? 'ready'
            : (status === 400 || status === 401 || status === 403 ? 'share_key_rejected' : '');
        if (!outcome || outcome === lastOutcome)
            return;
        lastOutcome = outcome;
        window.parent.postMessage({
            type: 'agent-browser-guacamole-share-auth',
            attemptId: attemptId,
            outcome: outcome
        }, parentOrigin);
    };

    var isTokenEndpoint = function isTokenEndpoint(method, url) {
        if (String(method || 'GET').toUpperCase() !== 'POST')
            return false;
        try {
            return /\/guacamole\/api\/tokens\/?$/.test(new URL(String(url), window.location.href).pathname);
        }
        catch (ignore) {
            return false;
        }
    };

    var hasShareKey = function hasShareKey(body) {
        if (typeof body === 'string')
            return /(?:^|&)key=[^&]+(?:&|$)/.test(body);
        if (body && typeof body.get === 'function') {
            try {
                return Boolean(body.get('key'));
            }
            catch (ignore) {
                return false;
            }
        }
        return false;
    };

    if (window.XMLHttpRequest && window.XMLHttpRequest.prototype) {
        var nativeOpen = window.XMLHttpRequest.prototype.open;
        var nativeSend = window.XMLHttpRequest.prototype.send;
        window.XMLHttpRequest.prototype.open = function open(method, url) {
            this.__agentBrowserShareTokenEndpoint = isTokenEndpoint(method, url);
            return nativeOpen.apply(this, arguments);
        };
        window.XMLHttpRequest.prototype.send = function send() {
            if (this.__agentBrowserShareTokenEndpoint && hasShareKey(arguments[0])) {
                this.addEventListener('loadend', function onShareTokenLoadEnd() {
                    reportStatus(this.status);
                }, { once: true });
            }
            return nativeSend.apply(this, arguments);
        };
    }

    if (typeof window.fetch === 'function') {
        var nativeFetch = window.fetch;
        window.fetch = function fetch(input, init) {
            var method = init && init.method
                ? init.method
                : (input && typeof input === 'object' && input.method ? input.method : 'GET');
            var url = input && typeof input === 'object' && input.url ? input.url : input;
            var observesTokenRequest = isTokenEndpoint(method, url) && hasShareKey(init && init.body);
            return nativeFetch.apply(this, arguments).then(function onResponse(response) {
                if (observesTokenRequest)
                    reportStatus(response.status);
                return response;
            });
        };
    }
}());
