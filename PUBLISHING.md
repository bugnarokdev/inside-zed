# Publish the Inside Zed site

`docs/` is the published site. It is generated from `site/` by `site/build.py`, so do not hand-edit anything under `docs/`: the next build overwrites it.

## Build

```sh
python3 site/build.py          # regenerate docs/ from site/
python3 site/build.py --check  # exit 1 if docs/ is stale (used in CI)
```

The builder needs no third-party packages and no network access.

## Layout

- `site/content/*.html` - authored page bodies, one file per page.
- `site/build.py` - layout, navigation, CSS and page assembly.
- `site/assets/` - copied verbatim into `docs/assets/`: `site.js`, the Bugnarok logos, and `i18n-fr.json`.
- `site/i18n_merge.py` - assembles the French dictionary consumed by `site.js`.
- `docs/` - build output, served by GitHub Pages.

## Enable GitHub Pages

1. Open the repository's **Settings**.
2. Select **Pages** under **Code and automation**.
3. Under **Build and deployment**, choose **Deploy from a branch**.
4. Select branch **main** and folder **/docs**.
5. Save.

The site publishes at `https://bugnarokdev.github.io/inside-zed/`.

## Language

Pages are written in English. `docs/assets/i18n-fr.json` maps each English string to its French rendering; `site.js` applies the mapping to text nodes when the reader selects FR, persists the choice in `localStorage`, and sets `<html lang>`. A string missing from the dictionary stays English rather than rendering empty.

## Important

The site is an independent research publication. It is not affiliated with or endorsed by Zed Industries. Source links point to the upstream repository; conclusions stay tied to the exact upstream revision inspected.
