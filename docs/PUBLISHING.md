# Publish the Inside Zed site

The static site is stored in the repository's `docs/` directory. It uses plain HTML and CSS; no build step or package installation is required.

## Enable GitHub Pages

1. Open the repository's **Settings**.
2. In the left sidebar, select **Pages** under **Code and automation**.
3. Under **Build and deployment**, choose **Deploy from a branch**.
4. Select branch **main** and folder **/docs**.
5. Save the setting.

GitHub Pages will publish the site at:

`https://bugnarokdev.github.io/inside-zed/`

The initial deployment may take a short while. Check the Pages section in repository settings for the published URL and deployment status.

## Site files

- `docs/index.html` - landing page.
- `docs/style.css` - dark visual system and responsive layout.
- `docs/architecture.html` - workspace and subsystem map.
- `docs/text-engine.html` - first-pass text engine study.
- `docs/locator.html` - locator investigation.

## Updating the site

Edit the relevant HTML or CSS file and commit it to `main`. GitHub Pages republishes the `/docs` directory automatically.

## Important

The site is an independent research publication. It is not affiliated with or endorsed by Zed Industries. Source links point to the upstream repository; conclusions should be kept tied to the exact upstream revision inspected.
