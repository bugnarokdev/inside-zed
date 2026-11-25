/* inside-zed site runtime: theme, mobile nav, EN/FR switching, TOC highlighting.
 *
 * Translation works on text nodes keyed by their exact English source text.
 * A key that is not in the dictionary simply stays English, so a missing entry
 * degrades to the authored copy instead of rendering an empty element. Nothing
 * here rewrites markup, so inline <code>/<strong>/<a> keep their structure.
 */
(function () {
  "use strict";

  var LANG_KEY = "ized-lang";
  var THEME_KEY = "ized-theme";
  var dict = null;
  var dictPromise = null;

  function store(key, value) {
    try {
      if (value === null) localStorage.removeItem(key);
      else localStorage.setItem(key, value);
    } catch (e) {
      /* private mode: the choice just will not persist */
    }
  }

  function read(key) {
    try {
      return localStorage.getItem(key);
    } catch (e) {
      return null;
    }
  }

  /* ---------------- theme ---------------- */

  function currentTheme() {
    var forced = document.documentElement.getAttribute("data-theme");
    if (forced === "dark" || forced === "light") return forced;
    return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  }

  function applyTheme(theme) {
    document.documentElement.setAttribute("data-theme", theme);
  }

  function initTheme() {
    var saved = read(THEME_KEY);
    if (saved === "dark" || saved === "light") applyTheme(saved);

    var btn = document.querySelector("button[data-theme]");
    if (!btn) return;
    btn.addEventListener("click", function () {
      var next = currentTheme() === "dark" ? "light" : "dark";
      applyTheme(next);
      store(THEME_KEY, next);
    });
  }

  /* ---------------- mobile nav ---------------- */

  function initMenu() {
    var btn = document.querySelector("[data-menu]");
    if (!btn) return;
    btn.addEventListener("click", function () {
      document.body.classList.toggle("nav-open");
    });
    document.addEventListener("keydown", function (e) {
      if (e.key === "Escape") document.body.classList.remove("nav-open");
    });
  }

  /* ---------------- translation ---------------- */

  function loadDict() {
    if (dictPromise) return dictPromise;
    dictPromise = fetch("./assets/i18n-fr.json")
      .then(function (r) {
        if (!r.ok) throw new Error("i18n " + r.status);
        return r.json();
      })
      .then(function (json) {
        dict = json;
        return json;
      })
      .catch(function () {
        dict = null;
        return null;
      });
    return dictPromise;
  }

  /* Collapse runs of whitespace, including the non-breaking spaces that HTML
   * entities turn into, so a DOM text node matches the key captured at build
   * time. Without this, "a&nbsp;b" never matches a key written as "a b". */
  function norm(text) {
    return String(text).replace(/[\s\u00a0]+/g, " ").trim();
  }

  function translateNode(node, table) {
    var raw = node.nodeValue;
    if (!raw) return;
    var key = norm(raw);
    if (!key) return;
    var hit = table[key];
    if (typeof hit !== "string" || hit === key) return;
    var lead = raw.slice(0, raw.length - raw.replace(/^\s+/, "").length);
    var tail = raw.slice(raw.replace(/\s+$/, "").length);
    node.nodeValue = lead + hit + tail;
  }

  var SKIP = { SCRIPT: 1, STYLE: 1, PRE: 1, CODE: 1, KBD: 1 };

  function translate(root, table, reverse) {
    var walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT, {
      acceptNode: function (n) {
        var p = n.parentNode;
        if (!p || SKIP[p.nodeName]) return NodeFilter.FILTER_REJECT;
        return n.nodeValue && n.nodeValue.trim()
          ? NodeFilter.FILTER_ACCEPT
          : NodeFilter.FILTER_REJECT;
      },
    });
    var nodes = [];
    while (walker.nextNode()) nodes.push(walker.currentNode);
    for (var i = 0; i < nodes.length; i++) translateNode(nodes[i], table);

    // attributes that carry reader-visible copy
    var tagged = root.querySelectorAll("[data-i18n-attr]");
    for (var j = 0; j < tagged.length; j++) {
      var el = tagged[j];
      var pairs = el.getAttribute("data-i18n-attr").split(",");
      for (var k = 0; k < pairs.length; k++) {
        var attr = pairs[k].trim();
        if (!attr) continue;
        var val = el.getAttribute(attr);
        if (val && table[val.trim()]) el.setAttribute(attr, table[val.trim()]);
      }
    }
  }

  /* Remember the original English so switching back is exact. */
  var original = null;

  function snapshot() {
    original = [];
    var walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT, null);
    while (walker.nextNode()) {
      var n = walker.currentNode;
      if (n.nodeValue && n.nodeValue.trim()) original.push([n, n.nodeValue]);
    }
  }

  function restore() {
    if (!original) return;
    for (var i = 0; i < original.length; i++) original[i][0].nodeValue = original[i][1];
  }

  function setLang(lang) {
    var buttons = document.querySelectorAll("[data-lang]");
    for (var i = 0; i < buttons.length; i++) {
      buttons[i].setAttribute("aria-pressed", buttons[i].dataset.lang === lang ? "true" : "false");
    }
    document.documentElement.setAttribute("lang", lang);
    store(LANG_KEY, lang);

    if (lang === "fr") {
      loadDict().then(function (table) {
        if (!table) return;
        if (!original) snapshot();
        restore();
        translate(document.body, table, false);
      });
    } else {
      restore();
    }
  }

  function initLang() {
    var saved = read(LANG_KEY) === "fr" ? "fr" : "en";
    var buttons = document.querySelectorAll("[data-lang]");
    for (var i = 0; i < buttons.length; i++) {
      buttons[i].setAttribute("aria-pressed", buttons[i].dataset.lang === saved ? "true" : "false");
      buttons[i].addEventListener("click", function () {
        setLang(this.dataset.lang);
      });
    }
    if (saved === "fr") setLang("fr");
  }

  /* ---------------- TOC scroll spy ---------------- */

  function initToc() {
    var links = document.querySelectorAll(".toc a");
    if (!links.length) return;
    var targets = [];
    for (var i = 0; i < links.length; i++) {
      var id = links[i].getAttribute("href").slice(1);
      var el = document.getElementById(id);
      if (el) targets.push([el, links[i]]);
    }
    if (!targets.length) return;

    var active = null;
    function onScroll() {
      var best = null;
      for (var i = 0; i < targets.length; i++) {
        var top = targets[i][0].getBoundingClientRect().top;
        if (top - 96 <= 0) best = targets[i];
      }
      if (!best) best = targets[0];
      if (best[1] === active) return;
      if (active) active.classList.remove("active");
      best[1].classList.add("active");
      active = best[1];
    }
    window.addEventListener("scroll", onScroll, { passive: true });
    onScroll();
  }

  function init() {
    initTheme();
    initMenu();
    initLang();
    initToc();
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", init);
  } else {
    init();
  }
})();
