# Chronos-Graph docs site

A dependency-free static documentation site (SDK + REST + MCP reference),
deployed to GitHub Pages from this `site/` directory.

## Local preview

It's plain HTML/CSS/JS — open `index.html` directly, or serve the folder:

```bash
cd site && python3 -m http.server 8000
# http://localhost:8000
```

## Deployment

Pushed automatically to GitHub Pages by `.github/workflows/pages.yml` whenever
`site/**` changes on `main`.

One-time repo setup: **Settings → Pages → Build and deployment → Source: GitHub
Actions**. The published URL is then `https://<owner>.github.io/Chronos-Graph/`.

## Editing

- `index.html` — all content (single page, sidebar nav, tabbed code blocks).
- `styles.css` — theme.
- `app.js` — tab switching + active-nav highlighting.

Keep examples in sync with the actual REST shapes (`crates/chronos-server`) and
SDK signatures (`sdks/`).
