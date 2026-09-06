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
 * PostgreSQL's anonymous sharing user has no readable ActiveConnection and
 * cannot re-share its tunnel. Guacamole 1.5.5 still asks that tunnel for sharing
 * profiles, producing a 404 during an otherwise working shared-view load.
 * Declare the empty capability before that request, only for our embedded
 * sharing viewer and its current authenticated shared-user identity. Other
 * users retain the original discovery call and all of its failures.
 */
(function installAgentBrowserSharedViewerCapabilities() {
    'use strict';

    if (!window.angular || window.parent === window
            || typeof window.name !== 'string'
            || window.name.indexOf('agent-browser-guacamole-share:') !== 0
            || !window.location || !/^[^.]+-share\./.test(window.location.hostname))
        return;

    window.angular.module('rest').config(['$provide', function ($provide) {
        $provide.decorator('tunnelService', ['$delegate', '$injector', function ($delegate, $injector) {
            var getSharingProfiles = $delegate.getSharingProfiles;
            $delegate.getSharingProfiles = function getAvailableSharingProfiles() {
                var authentication = $injector.get('authenticationService');
                if (authentication.getDataSource() === 'postgresql-shared'
                        && authentication.isAnonymous())
                    return $injector.get('$q').when({});
                return getSharingProfiles.apply(this, arguments);
            };
            return $delegate;
        }]);
    }]);
}());

/*
 * Embedded Guacamole must leave initial keyboard focus with its host dashboard.
 * Chromium blocks declarative autofocus in a cross-origin frame. Register after
 * templates.js on its existing module so this runs after the upstream cache
 * population, before any text-input directive links during Angular bootstrap.
 * Only that template's autofocus attribute changes; explicit focus and input
 * handlers remain available, and standalone Guacamole keeps its own default.
 */
(function installAgentBrowserEmbeddedTextInputTemplate() {
    'use strict';

    if (window.parent === window || !window.angular)
        return;

    window.angular.module('templates-main').run(['$templateCache', function ($templateCache) {
        var key = 'app/textInput/templates/guacTextInput.html';
        var template = $templateCache.get(key);
        if (typeof template !== 'string')
            return;
        $templateCache.put(key, template.replace(
            /(<textarea\b[^>]*?)\sautofocus(?:=(?:"[^"]*"|'[^']*'|[^\s>]+))?(?=\s|>)/g,
            '$1'
        ));
    }]);
}());

/*
 * Guacamole 1.5.5 also focuses its text target from the synchronous directive
 * controller, independently of the template's autofocus attribute. Suppress
 * only that embedded instance's initialization call. Restore its exact focus
 * property before returning (including exceptions), so subsequent pointer,
 * keyboard and IME interaction uses Guacamole's unmodified handlers. Do not
 * patch HTMLElement globally or move focus back after a remote key can escape.
 */
(function installAgentBrowserEmbeddedTextInputFocus() {
    'use strict';

    if (window.parent === window || !window.angular)
        return;

    window.angular.module('textInput').config(['$provide', function ($provide) {
        $provide.decorator('guacTextInputDirective', ['$delegate', '$injector', function ($delegate, $injector) {
            $delegate.forEach(function (directive) {
                var controller = directive.controller;
                if (!controller)
                    return;
                directive.controller = ['$scope', '$element', function ($scope, $element) {
                    var target = $element.find('.target')[0];
                    var descriptor = target && Object.getOwnPropertyDescriptor(target, 'focus');
                    if (target)
                        Object.defineProperty(target, 'focus', { configurable: true, value: function () {} });
                    try {
                        return $injector.invoke(controller, this, { $scope: $scope, $element: $element });
                    }
                    finally {
                        if (target) {
                            if (descriptor)
                                Object.defineProperty(target, 'focus', descriptor);
                            else
                                delete target.focus;
                        }
                    }
                }];
            });
            return $delegate;
        }]);
    }]);
}());

/*
 * Guacamole forwards and cancels display mouse events. That cancellation can
 * leave focus on the host dashboard even after a real remote click. Transfer
 * keyboard focus to the text-input target only during the trusted gesture;
 * initial loading and synthetic events must not steal host focus.
 */
(function installAgentBrowserDisplayKeyboardFocus() {
    'use strict';

    if (window.parent === window || !window.angular || !window.document)
        return;

    window.document.addEventListener('mousedown', function focusDisplayKeyboard(event) {
        if (!event.isTrusted || !event.target || typeof event.target.closest !== 'function'
                || !event.target.closest('.client-main .display'))
            return;

        var target = window.document.querySelector('.text-input textarea.target');
        if (target && !target.disabled)
            target.focus({ preventScroll: true });
    }, true);
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
