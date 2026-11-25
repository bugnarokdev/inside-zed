#!/usr/bin/env python3
"""Single-source site generator for inside-zed.

Reads canonical page sources from site/content/*.html, wraps them in the shared
documentation shell (grouped sidebar, central prose, right-hand table of
contents, light/dark theme, EN/FR switch, bracketed navigation, footer with the
project logo) and writes docs/*.html, which is what GitHub Pages serves.

Page sources stay as authored HTML: the generator lifts out the parts it needs
(breadcrumb, title, lede, article body) and never rewrites the prose, so measured
findings keep their exact wording.

    python3 site/build.py           # write docs/
    python3 site/build.py --check   # fail if docs/ is stale
"""

from __future__ import annotations

import html
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CONTENT = ROOT / "site" / "content"
ASSETS = ROOT / "site" / "assets"
DOCS = ROOT / "docs"

# Sidebar structure in reading order. A page must exist to be linked.
NAV: list[tuple[str, list[str]]] = [
    ("Get started", ["index"]),
    ("The engine", ["architecture", "text-engine", "text-engine#rope", "edit-lifecycle", "locator"]),
    ("Collaboration", ["collaboration-lab"]),
    ("Evidence", ["invariants-and-adversarial-testing", "measurements", "runtime"]),
]

# slug -> (english label, one-line english description for the landing grid)
PAGE_META: dict[str, tuple[str, str]] = {
    "index": ("Overview", ""),
    "architecture": ("Architecture", "How the crates fit together."),
    "text-engine": ("Text engine", "Ropes, sums, coordinates."),
    "edit-lifecycle": ("Edit lifecycle", "From keystroke to transaction."),
    "locator": ("Locator", "Finding symbols across buffers."),
    "collaboration-lab": ("Collaboration lab", "Multiplayer under load."),
    "invariants-and-adversarial-testing": ("Engineering audit", "What the tests actually prove."),
    "measurements": ("Measurements", "Numbers, with their method."),
    "runtime": ("Runtime", "Scheduling and the foreground executor."),
}

PAGES = [p for _, items in NAV for p in items if "#" not in p and p != "index"]

# label overrides for anchored sub-entries in the sidebar
ANCHOR_LABEL = {
    "text-engine#rope": "SumTree",
}

UPSTREAM = "https://github.com/zed-industries/zed"
REPO = "https://github.com/bugnarokdev/inside-zed"

UI = {
    "en": {
        "on_this_page": "On this page",
        "upstream": "Upstream",
        "repo": "Repository",
        "lang_label": "Language",
        "theme_label": "Toggle theme",
        "menu_label": "Menu",
        "prev": "Previous",
        "next": "Next",
        "independent": "Independent engineering study of Zed's Rust engine. Not affiliated with Zed Industries.",
        "study": "Study",
        "sources": "Sources",
        "code": "Code",
        "tagline": "Every claim is labelled VERIFIED, ANALYSIS or HYPOTHESIS.",
    },
    "fr": {
        "on_this_page": "Sur cette page",
        "upstream": "Amont",
        "repo": "Dépôt",
        "lang_label": "Langue",
        "theme_label": "Changer de thème",
        "menu_label": "Menu",
        "prev": "Précédent",
        "next": "Suivant",
        "independent": "Étude d'ingénierie indépendante du moteur Rust de Zed. Sans affiliation avec Zed Industries.",
        "study": "Étude",
        "sources": "Sources",
        "code": "Code",
        "tagline": "Chaque affirmation est étiquetée VERIFIED, ANALYSIS ou HYPOTHESIS.",
    },
}


def _dark_vars() -> str:
    return """\
    --bg:#0c0c0e; --surface:#161618; --elevated:#1c1c1f;
    --text:#ededef; --text-2:#c7c7cc; --muted:#a1a1a6; --dim:#6b6b73;
    --accent:#6b8fff; --accent-soft:#1a2140; --accent-hover:#8ea9ff;
    --border:#38383a; --border-muted:#26262a;
    --verified:#4ade80; --verified-bg:#0f2018; --verified-line:#2a5c3f;
    --analysis:#7cb8ff; --analysis-bg:#0d1620; --analysis-line:#2a4a70;
    --hypothesis:#f0bd76; --hypothesis-bg:#1a150d; --hypothesis-line:#5c4a24;
    --code-bg:#161618; --code-text:#8ce8c0; --marker:#6b6b73;
    --logo:url(./assets/bugnarok-dark.png);"""


DARK = _dark_vars()

STYLE = """\
:root{
  color-scheme:light;
  --bg:#fff; --surface:#f7f7f8; --elevated:#fff;
  --text:#0f0f11; --text-2:#3c3c43; --muted:#6b6b73; --dim:#8a8a92;
  --accent:#3b5cf6; --accent-soft:#eef1fe; --accent-hover:#2a44cc;
  --border:#dcdce2; --border-muted:#e8e8ee;
  --verified:#15803d; --verified-bg:#f0fbf4; --verified-line:#a7ddbc;
  --analysis:#1d4ed8; --analysis-bg:#f2f6fe; --analysis-line:#b3c9f2;
  --hypothesis:#92400e; --hypothesis-bg:#fdf8ee; --hypothesis-line:#e7cfa0;
  --code-bg:#f7f7f8; --code-text:#0f5f45; --marker:#8a8a92;
  --logo:url(./assets/bugnarok-light.png);
  --sidebar:272px; --toc:200px; --content:740px; --edge:28px;
  --mono:ui-monospace,SFMono-Regular,"SF Mono",Menlo,Consolas,"Liberation Mono",monospace;
  --sans:-apple-system,BlinkMacSystemFont,"Segoe UI",Inter,Roboto,Helvetica,Arial,sans-serif;
  --ease:cubic-bezier(.2,.8,.2,1);
}
html[data-theme="dark"]{
  color-scheme:dark;
__DARK__
}
@media (prefers-color-scheme:dark){
  html:not([data-theme="light"]){
    color-scheme:dark;
__DARKINDENT__
  }
}
*{box-sizing:border-box}
html{scroll-behavior:smooth;scroll-padding-top:76px;-webkit-text-size-adjust:100%}
body{margin:0;background:var(--bg);color:var(--text);font:16px/1.7 var(--sans);
  -webkit-font-smoothing:antialiased;text-rendering:optimizeLegibility;
  display:flex;flex-direction:column;min-height:100vh}
a{color:var(--accent);text-decoration:none}
a:hover{text-decoration:underline;text-underline-offset:3px}
code,kbd,pre{font-family:var(--mono)}

/* ---------- top bar ---------- */
.topbar{position:sticky;top:0;z-index:40;height:56px;display:flex;align-items:center;
  gap:14px;padding:0 var(--edge);
  background:color-mix(in srgb,var(--bg) 86%,transparent);
  backdrop-filter:blur(16px) saturate(160%);border-bottom:1px solid var(--border-muted)}
.brand{display:flex;align-items:center;gap:9px;font:700 12.5px/1 var(--mono);
  letter-spacing:.14em;color:var(--text);white-space:nowrap}
.brand:hover{text-decoration:none;color:var(--accent)}
.brand .dot{width:6px;height:6px;border-radius:50%;background:var(--accent);flex:none;
  box-shadow:0 0 0 3px color-mix(in srgb,var(--accent) 18%,transparent)}
.topbar .spacer{flex:1}
.tb-link{color:var(--muted);font:500 12.5px/1 var(--sans);white-space:nowrap}
.tb-link:hover{color:var(--accent);text-decoration:none}
.tb-btn{display:inline-flex;align-items:center;justify-content:center;gap:6px;
  width:32px;height:32px;padding:0;border:1px solid var(--border);border-radius:7px;
  background:transparent;color:var(--muted);cursor:pointer;
  transition:background .15s var(--ease),color .15s var(--ease)}
.tb-btn:hover{background:var(--surface);color:var(--text)}
.lang{display:flex;border:1px solid var(--border);border-radius:7px;overflow:hidden}
.lang button{border:0;background:transparent;color:var(--dim);
  font:600 10.5px/1 var(--mono);letter-spacing:.08em;padding:9px 8px;cursor:pointer;
  transition:background .15s var(--ease),color .15s var(--ease)}
.lang button:hover{color:var(--text)}
.lang button[aria-pressed="true"]{background:var(--accent-soft);color:var(--accent)}

/* ---------- shell ---------- */
.shell{display:grid;grid-template-columns:var(--sidebar) minmax(0,1fr);
  min-height:calc(100vh - 56px);flex:1 0 auto}
.sidebar{position:sticky;top:56px;height:calc(100vh - 56px);overflow-y:auto;
  border-right:1px solid var(--border-muted);padding:24px 12px 40px;
  scrollbar-width:thin}
.side-group{margin-bottom:20px}
.side-group>h4{margin:0 0 6px;padding:0 10px;font:600 10.5px/1.5 var(--mono);
  letter-spacing:.13em;text-transform:uppercase;color:var(--dim)}
.side-group a{display:flex;align-items:baseline;gap:0;padding:6px 10px;border-radius:6px;
  color:var(--text-2);font-size:14px;line-height:1.45;
  transition:color .15s var(--ease),background .15s var(--ease)}
.side-group a:hover{color:var(--text);background:var(--surface);text-decoration:none}
.side-group a:hover .brk{color:var(--accent)}
.side-group a[aria-current="page"]{color:var(--accent);background:var(--accent-soft);font-weight:600}
.side-group a[aria-current="page"] .brk{color:var(--accent)}
.side-group a.sub{padding-left:22px;font-size:13.5px;color:var(--muted)}

/* bracketed nav marker: [ - ] where the dash grows in on hover, exactly the
   mechanism opencode uses for its footer and menu links */
.brk{display:inline-flex;align-items:center;color:var(--marker);
  font:400 .92em/1 var(--mono);flex:none;
  transition:color .15s var(--ease)}
.brk .fill{display:inline-block;width:0;overflow:hidden;white-space:nowrap;
  transition:width .22s var(--ease)}
.brk .fill::before{content:"-";display:inline-block;padding:0 .12em}
a:hover>.brk .fill,a[aria-current="page"]>.brk .fill{width:.9em}
@media (prefers-reduced-motion:reduce){
  .brk .fill{transition:none}
}

.side-meta{margin-top:22px;padding:14px 10px 0;border-top:1px solid var(--border-muted);
  color:var(--dim);font:11px/1.7 var(--mono)}
.side-meta code{color:var(--muted)}

/* ---------- layout ---------- */
.layout{display:grid;grid-template-columns:minmax(0,var(--content)) var(--toc);
  gap:52px;padding:40px var(--edge) 88px;
  max-width:calc(var(--content) + var(--toc) + 52px + var(--edge) * 2)}
.toc{position:sticky;top:80px;align-self:start;font-size:13px}
.toc>h4{margin:0 0 10px;font:600 10.5px/1.5 var(--mono);letter-spacing:.13em;
  text-transform:uppercase;color:var(--dim)}
.toc a{display:block;padding:4px 0 4px 12px;border-left:1px solid var(--border-muted);
  color:var(--muted);line-height:1.5;transition:color .15s var(--ease),border-color .15s var(--ease)}
.toc a:hover,.toc a.active{color:var(--accent);border-left-color:var(--accent);
  text-decoration:none}
.toc a.active{font-weight:600}

/* ---------- prose ---------- */
.crumb{font:600 10.5px/1.5 var(--mono);letter-spacing:.12em;text-transform:uppercase;
  color:var(--dim);margin-bottom:10px}
.doc-head h1{font-size:clamp(29px,4.2vw,42px);line-height:1.1;letter-spacing:-.032em;
  margin:0 0 14px;font-weight:700}
.doc-head .lede{font-size:17.5px;line-height:1.62;color:var(--muted);margin:0;max-width:68ch}
.article{margin-top:32px;font-size:16px}
.article h2{font-size:23px;line-height:1.28;letter-spacing:-.02em;margin:44px 0 12px;
  font-weight:650;scroll-margin-top:76px}
.article h2:first-child{margin-top:0}
.article h3{font-size:17px;margin:28px 0 8px;font-weight:650}
.article h4{font-size:15px;margin:22px 0 6px;font-weight:650}
.article p,.article li{color:var(--text-2)}
.article p{margin:0 0 16px}
.article ul,.article ol{padding-left:22px;margin:0 0 18px}
.article li{margin:6px 0}
.article strong{color:var(--text);font-weight:650}
.article code{font-size:.88em;background:var(--code-bg);color:var(--code-text);
  padding:2px 5px;border-radius:4px;border:1px solid var(--border-muted)}
.article pre{overflow:auto;background:var(--code-bg);border:1px solid var(--border);
  border-radius:8px;padding:16px 18px;margin:0 0 20px;font-size:12.5px;line-height:1.65;
  color:var(--text-2)}
.article pre code{background:none;border:0;padding:0;font-size:inherit;color:inherit}
.article hr{border:0;border-top:1px solid var(--border-muted);margin:32px 0}
.article table{width:100%;border-collapse:collapse;margin:0 0 22px;font-size:13.5px}
.article th,.article td{text-align:left;padding:9px 12px;
  border-bottom:1px solid var(--border-muted);vertical-align:top}
.article th{font:600 10.5px/1.5 var(--mono);letter-spacing:.08em;text-transform:uppercase;
  color:var(--dim);border-bottom-color:var(--border)}
.article tbody tr:hover{background:var(--surface)}
.article a{text-decoration:underline;text-decoration-color:color-mix(in srgb,var(--accent) 35%,transparent);
  text-underline-offset:3px}
.article a:hover{text-decoration-color:var(--accent)}

/* evidence callouts */
.callout{margin:22px 0;padding:14px 18px;border-left:3px solid var(--border);
  border-radius:0 8px 8px 0;background:var(--surface)}
.callout p{margin:0;font-size:15px;color:var(--text-2)}
.callout p+p{margin-top:10px}
.callout strong:first-child{font:600 10.5px/1 var(--mono);letter-spacing:.1em}
.callout.verified{border-color:var(--verified-line);background:var(--verified-bg)}
.callout.verified strong:first-child{color:var(--verified)}
.callout.analysis{border-color:var(--analysis-line);background:var(--analysis-bg)}
.callout.analysis strong:first-child{color:var(--analysis)}
.callout.hypothesis{border-color:var(--hypothesis-line);background:var(--hypothesis-bg)}
.callout.hypothesis strong:first-child{color:var(--hypothesis)}

.pager{display:flex;gap:12px;margin-top:52px;padding-top:22px;
  border-top:1px solid var(--border-muted)}
.pager a{flex:1;display:block;padding:13px 16px;border:1px solid var(--border);
  border-radius:9px;color:var(--text-2);
  transition:border-color .15s var(--ease),background .15s var(--ease)}
.pager a:hover{border-color:var(--accent);background:var(--surface);text-decoration:none}
.pager .dir{display:block;font:600 9.5px/1 var(--mono);letter-spacing:.11em;
  text-transform:uppercase;color:var(--dim);margin-bottom:6px}
.pager .title{font-weight:600;color:var(--text);font-size:15px}
.pager a.next{text-align:right}
.pager a.next .title{display:block}

/* ---------- landing ---------- */
.landing{max-width:900px;padding:56px var(--edge) 80px}
.landing .eyebrow{font:600 10.5px/1.5 var(--mono);letter-spacing:.15em;
  text-transform:uppercase;color:var(--accent);margin-bottom:16px;display:flex;
  align-items:center;gap:8px}
.landing h1{font-size:clamp(33px,5vw,52px);line-height:1.06;letter-spacing:-.038em;
  margin:0 0 18px;font-weight:700}
.landing .lede{font-size:19px;line-height:1.6;color:var(--muted);max-width:60ch;margin:0 0 22px}
.pills{display:flex;flex-wrap:wrap;gap:7px;margin:0 0 36px}
.pill{border:1px solid var(--border);border-radius:999px;padding:5px 11px;
  color:var(--muted);font:500 11px/1.4 var(--mono);letter-spacing:.04em;background:var(--bg)}
.landing h2{display:flex;align-items:center;gap:10px;font:600 10.5px/1.5 var(--mono);
  letter-spacing:.13em;text-transform:uppercase;color:var(--dim);margin:40px 0 14px}
.landing h2::after{content:"";flex:1;height:1px;background:var(--border-muted)}
.panels{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:12px}
.panel{border:1px solid var(--border);border-radius:11px;padding:20px;background:var(--bg);
  transition:border-color .15s var(--ease),background .15s var(--ease)}
.panel:hover{border-color:color-mix(in srgb,var(--accent) 45%,var(--border));background:var(--surface)}
.panel h3{margin:0 0 8px;font-size:16.5px;letter-spacing:-.01em}
.panel p{margin:0;color:var(--text-2);font-size:14px;line-height:1.65}
.panel .more{display:inline-flex;align-items:center;gap:0;margin-top:12px;
  font-size:13px;font-weight:600;color:var(--accent)}
.panel .acts{display:flex;flex-wrap:wrap;gap:6px 18px;margin:14px 0 0}
.panel .acts a{font-size:13px;font-weight:600}
.panel .ev{display:flex;gap:10px;align-items:baseline;margin:0 0 10px;font-size:14px}
.panel .ev:last-child{margin-bottom:0}
.tag{display:inline-block;flex:none;font:600 9px/1 var(--mono);letter-spacing:.1em;
  padding:5px 7px;border-radius:4px;border:1px solid}
.tag.verified{color:var(--verified);border-color:var(--verified-line);background:var(--verified-bg)}
.tag.analysis{color:var(--analysis);border-color:var(--analysis-line);background:var(--analysis-bg)}
.tag.hypothesis{color:var(--hypothesis);border-color:var(--hypothesis-line);background:var(--hypothesis-bg)}
.note{margin:0 0 12px;padding:16px 20px;border:1px solid var(--border);border-radius:10px;
  background:var(--surface);max-width:66ch}
.note p{margin:0;color:var(--text-2);font-size:15px}
.note p:first-child{margin-bottom:8px}
.note strong{font:600 10.5px/1 var(--mono);letter-spacing:.12em;color:var(--accent)}
.landing h1 .accent{color:var(--accent)}
.grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:10px}
.item{display:block;padding:15px 16px;border:1px solid var(--border);border-radius:10px;
  color:var(--text-2);background:var(--bg);
  transition:border-color .15s var(--ease),background .15s var(--ease)}
.item:hover{border-color:var(--accent);background:var(--surface);text-decoration:none}
.item:hover .brk{color:var(--accent)}
.item .n{font:600 9.5px/1 var(--mono);letter-spacing:.1em;color:var(--dim)}
.item .t{display:block;font-weight:650;color:var(--text);margin:7px 0 4px;font-size:15px}
.item .d{font-size:13px;color:var(--muted);line-height:1.55}
.item .s{display:block;margin-top:9px;font:600 9px/1 var(--mono);
  letter-spacing:.1em;text-transform:uppercase;color:var(--accent)}

/* ---------- footer ---------- */
.site-footer{border-top:1px solid var(--border-muted);background:var(--surface);
  padding:40px var(--edge) 34px;margin-top:0}
.foot-inner{max-width:1080px;margin:0 auto;display:flex;flex-wrap:wrap;
  align-items:flex-start;gap:32px}
.foot-brand{flex:0 0 auto;min-width:240px}
.foot-logo{display:block;width:clamp(180px,24vw,260px);height:auto;
  background-image:var(--logo);background-size:contain;background-repeat:no-repeat;
  background-position:left center;opacity:.92;
  transition:opacity .2s var(--ease)}
.foot-brand:hover .foot-logo{opacity:1}
.foot-tag{margin:14px 0 0;font:11.5px/1.7 var(--mono);color:var(--dim);max-width:34ch}
.foot-nav{flex:1 1 320px}
.foot-nav ul{list-style:none;margin:0;padding:0;display:flex;flex-wrap:wrap;
  gap:8px 22px}
.foot-nav a{display:inline-flex;align-items:center;gap:0;color:var(--muted);
  font:400 13px/1.9 var(--mono)}
.foot-nav a:hover{color:var(--text);text-decoration:none}
.foot-nav a:hover .brk{color:var(--accent)}
.foot-nav svg{width:6px;height:6px;margin-left:5px;color:var(--dim);flex:none}
.foot-bottom{max-width:1080px;margin:28px auto 0;padding-top:18px;
  border-top:1px solid var(--border-muted);display:flex;flex-wrap:wrap;
  gap:10px 18px;align-items:center;justify-content:space-between;
  font:11px/1.7 var(--mono);color:var(--dim)}
.foot-bottom a{color:var(--muted)}
.foot-bottom a:hover{color:var(--accent)}
.foot-dot{display:inline-block;width:5px;height:5px;border-radius:50%;
  background:var(--verified);margin-right:6px;vertical-align:middle}

/* ---------- responsive ---------- */
.menu-btn{display:none}
@media (max-width:1180px){
  .layout{grid-template-columns:minmax(0,1fr);gap:0;max-width:none}
  .toc{display:none}
}
@media (max-width:900px){
  .shell{grid-template-columns:1fr}
  .sidebar{position:fixed;inset:56px 0 0 0;height:auto;background:var(--bg);z-index:35;
    display:none;border-right:0;padding-bottom:80px}
  body.nav-open .sidebar{display:block}
  body.nav-open{overflow:hidden}
  .menu-btn{display:inline-flex}
  .layout,.landing{padding-left:20px;padding-right:20px}
  .panels,.grid{grid-template-columns:1fr}
  .tb-link{display:none}
  .foot-inner{gap:26px}
}
@media (max-width:520px){
  .pager{flex-direction:column}
  .foot-bottom{flex-direction:column;align-items:flex-start}
}
"""


def css_sheet() -> str:
    """Substitute the dark palette into the light-first stylesheet skeleton."""
    body = DARK.strip()
    indented = "\n".join("    " + line.strip() for line in body.splitlines())
    return STYLE.replace("__DARK__", body).replace("__DARKINDENT__", indented)


def read_page(slug: str) -> dict:
    """Lift the shell-relevant parts out of an authored page source."""
    path = CONTENT / f"{slug}.html"
    src = path.read_text(encoding="utf-8")

    crumb = re.search(r'class="crumb">(.*?)</div>', src, re.S)
    h1 = re.search(r"<h1[^>]*>(.*?)</h1>", src, re.S)
    lede = re.search(r"</h1>\s*<p[^>]*>(.*?)</p>", src, re.S)
    article = re.search(r'<article class="article">(.*?)</article>', src, re.S)
    if not (h1 and article):
        raise SystemExit(f'{path}: need <h1> and <article class="article">')

    title = html.unescape(re.sub(r"<[^>]+>", "", h1.group(1))).strip()
    return {
        "slug": slug,
        "crumb": crumb.group(1).strip() if crumb else "",
        "title": title,
        "title_html": h1.group(1).strip(),
        "lede": lede.group(1).strip() if lede else "",
        "body": article.group(1).strip(),
        "description": (html.unescape(re.sub(r"<[^>]+>", "", lede.group(1))).strip()
                        if lede else title),
    }


def brk() -> str:
    return '<span class="brk" aria-hidden="true">[<span class="fill"></span>]</span>'


def sidebar_html(current: str) -> str:
    out = []
    for group, items in NAV:
        out.append('<div class="side-group">')
        out.append(f"<h4>{html.escape(group)}</h4>")
        for slug in items:
            page, _, anchor = slug.partition("#")
            href = f"./{page}.html" + (f"#{anchor}" if anchor else "")
            label = ANCHOR_LABEL.get(slug) or PAGE_META[page][0]
            is_sub = bool(anchor)
            active = ' aria-current="page"' if page == current and not anchor else ""
            cls = ' class="sub"' if is_sub else ""
            out.append(f'<a href="{href}"{cls}{active}>{brk()}{html.escape(label)}</a>')
        out.append("</div>")
    return "\n".join(out)


def toc_html(body: str) -> str:
    out = []
    for anchor, text in re.findall(r'<h2 id="([^"]+)"[^>]*>(.*?)</h2>', body, re.S):
        label = html.unescape(re.sub(r"<[^>]+>", "", text)).strip()
        out.append(f'<a href="#{anchor}">{html.escape(label)}</a>')
    return "\n".join(out)


def footer_html(ui: dict) -> str:
    links = [
        (REPO, ui["repo"]),
        (UPSTREAM, ui["upstream"]),
        (f"{REPO}/blob/main/harness/tests", ui["code"]),
        (f"{REPO}/blob/main/harness/results/suite-run.txt", ui["sources"]),
        (f"{REPO}/blob/main/README.md", "README"),
    ]
    items = []
    for href, label in links:
        ext = ' target="_blank" rel="noreferrer"'
        arrow = ('<svg viewBox="0 0 6 6" fill="none" aria-hidden="true">'
                 '<path d="M0 .5H5.5V6M5.5.5.25 5.75" stroke="currentColor" '
                 'stroke-width="1"/></svg>' if "github.com" in href else "")
        items.append(f'<li><a href="{href}"{ext}>{brk()}{html.escape(label)}{arrow}</a></li>')

    return f"""<footer class="site-footer">
<div class="foot-inner">
<div class="foot-brand">
<a href="./index.html" class="foot-logo" aria-label="BUGNARÖK" style="height:52px"></a>
<p class="foot-tag">{ui["independent"]}</p>
</div>
<nav class="foot-nav" aria-label="Secondary">
<ul>
{"".join(items)}
</ul>
</nav>
</div>
<div class="foot-bottom">
<span><span class="foot-dot"></span>33 tests · 0 failures · Zed @ <code>2664a0aa</code></span>
<span>{ui["tagline"]}</span>
</div>
</footer>"""


def shell(*, title: str, description: str, body: str, current: str,
          landing: bool = False) -> str:
    ui = UI["en"]
    lang_buttons = "".join(
        f'<button type="button" data-lang="{code}" aria-pressed="{"true" if code == "en" else "false"}">'
        f"{code.upper()}</button>" for code in ("en", "fr"))
    main = f'<div class="landing">{body}</div>' if landing else body

    return f"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<meta name="description" content="{html.escape(description)}">
<title>{html.escape(title)} · Inside Zed</title>
<link rel="stylesheet" href="./style.css">
<link rel="preload" href="./assets/bugnarok-light.png" as="image">
<script>
(function () {{
  try {{
    var l = localStorage.getItem("ized-lang");
    if (l === "fr") document.documentElement.setAttribute("lang", "fr");
    var t = localStorage.getItem("ized-theme");
    if (t === "dark" || t === "light") document.documentElement.setAttribute("data-theme", t);
  }} catch (e) {{}}
}})();
</script>
</head>
<body>
<header class="topbar">
<button class="tb-btn menu-btn" type="button" data-menu aria-label="{ui["menu_label"]}">☰</button>
<a class="brand" href="./index.html"><span class="dot"></span>INSIDE ZED</a>
<span class="spacer"></span>
<a class="tb-link" href="{UPSTREAM}" target="_blank" rel="noreferrer">{ui["upstream"]} ↗</a>
<a class="tb-link" href="{REPO}" target="_blank" rel="noreferrer">{ui["repo"]} ↗</a>
<div class="lang" role="group" aria-label="{ui["lang_label"]}">{lang_buttons}</div>
<button class="tb-btn" type="button" data-theme aria-label="{ui["theme_label"]}">◐</button>
</header>
<div class="shell">
<nav class="sidebar" aria-label="Docs">
{sidebar_html(current)}
<div class="side-meta">
Zed @ <code>2664a0aa</code><br>
33 tests · 0 failures<br>
<a href="{REPO}" target="_blank" rel="noreferrer">{ui["repo"]} ↗</a>
</div>
</nav>
<main>
{main}
</main>
</div>
{footer_html(ui)}
<script src="./assets/site.js" defer></script>
</body>
</html>
"""


def build_page(page: dict, i: int, total: int) -> str:
    body = page["body"]
    prev_slug = PAGES[(i - 1) % total]
    next_slug = PAGES[(i + 1) % total]

    pager = (
        '<div class="pager">'
        f'<a href="./{prev_slug}.html"><span class="dir">{UI["en"]["prev"]}</span>'
        f'<span class="title">{html.escape(PAGE_META[prev_slug][0])}</span></a>'
        f'<a class="next" href="./{next_slug}.html"><span class="dir">{UI["en"]["next"]}</span>'
        f'<span class="title">{html.escape(PAGE_META[next_slug][0])}</span></a>'
        "</div>")

    toc = toc_html(body)
    aside = (f'<aside class="toc"><h4>{UI["en"]["on_this_page"]}</h4>\n{toc}\n</aside>'
             if toc else "")

    inner = f"""<div class="layout">
<article>
<div class="doc-head">
<div class="crumb">{page["crumb"]}</div>
<h1>{page["title_html"]}</h1>
<p class="lede">{page["lede"]}</p>
</div>
<div class="article">
{body}
</div>
{pager}
</article>
{aside}
</div>"""

    return shell(title=page["title"], description=page["description"],
                 body=inner, current=page["slug"])


def build_landing() -> str:
    inner = (CONTENT / "index.html").read_text(encoding="utf-8").strip()
    return shell(
        title="Under the hood",
        description="An independent, source-based technical exploration of Zed's Rust engine.",
        body=inner, current="index", landing=True)


def main() -> int:
    check = "--check" in sys.argv
    stale: list[str] = []

    def emit(path: Path, text: str) -> None:
        current = path.read_text(encoding="utf-8") if path.exists() else ""
        if current == text:
            return
        stale.append(path.name)
        if not check:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")

    emit(DOCS / "style.css", css_sheet())

    for i, slug in enumerate(PAGES):
        emit(DOCS / f"{slug}.html", build_page(read_page(slug), i, len(PAGES)))
    emit(DOCS / "index.html", build_landing())

    # assets are copied verbatim
    for asset in sorted(ASSETS.iterdir()):
        if asset.is_file():
            target = DOCS / "assets" / asset.name
            data = asset.read_bytes()
            if not target.exists() or target.read_bytes() != data:
                stale.append(f"assets/{asset.name}")
                if not check:
                    (DOCS / "assets").mkdir(parents=True, exist_ok=True)
                    target.write_bytes(data)

    if check:
        if stale:
            print("stale:", " ".join(sorted(stale)))
            return 1
        print("docs/ up to date")
        return 0

    print(f"built {len(PAGES) + 1} pages + {len(list(ASSETS.iterdir()))} assets")
    for name in sorted(stale):
        print("  updated", name)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
