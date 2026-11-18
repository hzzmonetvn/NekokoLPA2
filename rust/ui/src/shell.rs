//! Dioxus 0.6 SSR HTML fragment & initial HTML5 document rendering with safe DOM delegation.

use dioxus::prelude::VirtualDom;
use nlpa_core::i18n::resolve_locale;

use crate::components::app::{app_root, AppRootProps};
use crate::models::{locale_code, RenderMeta, UiModel};
use crate::theme::{ThemeTokens, APP_CSS};

/// Renders the root UI fragment (`<div class="nlpa-shell ...">...</div>`) via Dioxus SSR.
#[must_use]
pub fn render_fragment(model: &UiModel) -> String {
    let mut vdom = VirtualDom::new_with_props(
        app_root,
        AppRootProps {
            model: model.clone(),
        },
    );
    vdom.rebuild_in_place();
    dioxus_ssr::render(&vdom)
}

/// Builds the full [`RenderMeta`] JSON payload for JNI/WebView updates.
#[must_use]
pub fn render_meta(model: &UiModel) -> RenderMeta {
    let tokens = ThemeTokens::resolve(
        model.theme_style,
        model.color_mode,
        model.system_dark,
        model.wallpaper_enabled,
        model.wallpaper_opacity,
    );
    RenderMeta {
        html: render_fragment(model),
        theme_style: tokens.theme_style_wire().to_string(),
        dark: tokens.is_dark,
        wallpaper_active: tokens.wallpaper_active,
        wallpaper_opacity: tokens.wallpaper_opacity,
        status_bar_black: tokens.status_bar_black,
    }
}

/// Renders the full initial HTML5 document loaded into Android's `WebView`.
#[must_use]
pub fn render_document(model: &UiModel) -> String {
    let lang = locale_code(resolve_locale(model.locale, "en"));
    let fragment = render_fragment(model);
    format!(
        r#"<!DOCTYPE html>
<html lang="{lang}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no, viewport-fit=cover">
<title>NekokoLPA</title>
<style>{APP_CSS}</style>
</head>
<body>
<div id="app">{fragment}</div>
<script>{WEBVIEW_BRIDGE_JS}</script>
</body>
</html>"#
    )
}

const WEBVIEW_BRIDGE_JS: &str = r#"
(function() {
  var liveTimer = null;

  function resolveHost() {
    if (window.AndroidHost && typeof window.AndroidHost.dispatch === 'function') {
      return window.AndroidHost;
    }
    if (window.AndroidJsHost && typeof window.AndroidJsHost.dispatch === 'function') {
      return window.AndroidJsHost;
    }
    return null;
  }

  window.__nlpaDispatch = function(actionObj) {
    var host = resolveHost();
    if (!host || !actionObj) return;
    try {
      var res = host.dispatch(JSON.stringify(actionObj));
      if (typeof res === 'string' && res.length > 0) {
        window.__nlpaPatch(res);
      }
    } catch (e) {}
  };

  window.__nlpaPatch = function(payload) {
    var html = '';
    if (typeof payload === 'string') {
      var trimmed = payload.trim();
      if (trimmed.charAt(0) === '{') {
        try {
          var parsed = JSON.parse(trimmed);
          html = parsed && typeof parsed.html === 'string' ? parsed.html : '';
        } catch (_) {
          html = payload;
        }
      } else {
        html = payload;
      }
    } else if (payload && typeof payload.html === 'string') {
      html = payload.html;
    }
    if (!html) return;

    var container = document.getElementById('app');
    if (!container) return;

    var active = document.activeElement;
    var activeId = active && active.id ? active.id : null;
    var selStart = null;
    var selEnd = null;
    if (active && typeof active.selectionStart === 'number') {
      selStart = active.selectionStart;
      selEnd = active.selectionEnd;
    }
    var mainEl = container.querySelector('.nlpa-main');
    var modalEl = container.querySelector('.nlpa-modal');
    var mainScroll = mainEl ? mainEl.scrollTop : 0;
    var modalScroll = modalEl ? modalEl.scrollTop : 0;
    var winY = window.scrollY || 0;

    container.innerHTML = html;

    var nextMain = container.querySelector('.nlpa-main');
    if (nextMain) nextMain.scrollTop = mainScroll;
    var nextModal = container.querySelector('.nlpa-modal');
    if (nextModal) nextModal.scrollTop = modalScroll;
    if (winY > 0) window.scrollTo(0, winY);

    if (activeId) {
      var target = document.getElementById(activeId);
      if (target) {
        target.focus();
        if (selStart !== null && typeof target.setSelectionRange === 'function') {
          try {
            target.setSelectionRange(selStart, selEnd !== null ? selEnd : selStart);
          } catch (_) {}
        }
      }
    }
  };

  function collectMappedValues(spec, payload, mode) {
    if (!spec) return;
    var pairs = spec.split(',');
    for (var i = 0; i < pairs.length; i++) {
      var parts = pairs[i].split(':');
      if (parts.length !== 2) continue;
      var prop = parts[0].trim();
      var domId = parts[1].trim();
      var el = document.getElementById(domId);
      if (!el) continue;
      if (mode === 'check') {
        payload[prop] = !!el.checked;
      } else if (mode === 'int') {
        var parsedInt = parseInt(el.value, 10);
        if (!isNaN(parsedInt)) payload[prop] = parsedInt;
      } else {
        payload[prop] = el.value != null ? String(el.value) : '';
      }
    }
  }

  document.addEventListener('click', function(evt) {
    var btn = evt.target && evt.target.closest ? evt.target.closest('[data-action]') : null;
    if (!btn || btn.disabled) return;
    var payload = { action: btn.getAttribute('data-action') };
    var attrs = btn.attributes;
    for (var i = 0; i < attrs.length; i++) {
      var name = attrs[i].name;
      var val = attrs[i].value;
      if (name.indexOf('data-p-') === 0) {
        payload[name.slice(7)] = val;
      } else if (name.indexOf('data-b-') === 0) {
        payload[name.slice(7)] = val === 'true';
      } else if (name.indexOf('data-n-') === 0) {
        var n = parseInt(val, 10);
        if (!isNaN(n)) payload[name.slice(7)] = n;
      }
    }
    collectMappedValues(btn.getAttribute('data-from-inputs'), payload, 'value');
    collectMappedValues(btn.getAttribute('data-from-checks'), payload, 'check');
    collectMappedValues(btn.getAttribute('data-int-inputs'), payload, 'int');
    window.__nlpaDispatch(payload);
  });

  function buildControlPayload(el, actionAttr) {
    var action = el.getAttribute(actionAttr);
    var prop = el.getAttribute('data-prop') || 'value';
    if (!action) return null;
    var payload = { action: action };
    var attrs = el.attributes;
    for (var i = 0; i < attrs.length; i++) {
      var name = attrs[i].name;
      if (name.indexOf('data-extra-') === 0) {
        payload[name.slice(11)] = attrs[i].value;
      }
    }
    if (el.type === 'checkbox') {
      payload[prop] = !!el.checked;
    } else if (el.getAttribute('data-kind') === 'int') {
      var iv = parseInt(el.value, 10);
      if (isNaN(iv)) return null;
      payload[prop] = iv;
    } else if (el.getAttribute('data-kind') === 'float') {
      var fv = parseFloat(el.value);
      if (isNaN(fv)) return null;
      payload[prop] = fv;
    } else {
      payload[prop] = el.value != null ? String(el.value) : '';
    }
    return payload;
  }

  document.addEventListener('change', function(evt) {
    var el = evt.target;
    if (!el || !el.getAttribute) return;
    if (liveTimer) {
      clearTimeout(liveTimer);
      liveTimer = null;
    }
    var payload = buildControlPayload(el, 'data-change-action');
    if (payload) window.__nlpaDispatch(payload);
  });

  document.addEventListener('input', function(evt) {
    var el = evt.target;
    if (!el || !el.getAttribute) return;
    if (el.getAttribute('data-immediate') === 'css-alpha') {
      var shell = document.querySelector('.nlpa-shell');
      var v = parseFloat(el.value);
      if (shell && !isNaN(v)) {
        shell.style.setProperty('--nlpa-glass-alpha', v.toFixed(2));
      }
      var lbl = document.getElementById('nlpa-opacity-label');
      if (lbl && !isNaN(v)) {
        lbl.textContent = Math.round(v * 100) + '%';
      }
    }
    var action = el.getAttribute('data-live-action');
    if (!action) return;
    if (liveTimer) clearTimeout(liveTimer);
    liveTimer = setTimeout(function() {
      var payload = buildControlPayload(el, 'data-live-action');
      if (payload) window.__nlpaDispatch(payload);
    }, 240);
  });
})();
"#;
